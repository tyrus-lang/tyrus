use super::schema::DbSchemaSnapshot;
use crate::ast::{Field, Type};
use std::ops::Range;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SqlTokenKind {
    Word,
    Number,
    String,
    Placeholder,
    Star,
    Comma,
    Dot,
    LeftParen,
    RightParen,
    Operator,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlToken {
    pub kind: SqlTokenKind,
    pub text: String,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlProjection {
    pub name: String,
    pub alias: Option<String>,
    pub aggregate: Option<SqlAggregate>,
    pub span: Range<usize>,
    pub alias_span: Option<Range<usize>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SqlAggregate {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

impl SqlAggregate {
    fn from_keyword(value: &str) -> Option<Self> {
        if value.eq_ignore_ascii_case("COUNT") {
            Some(Self::Count)
        } else if value.eq_ignore_ascii_case("SUM") {
            Some(Self::Sum)
        } else if value.eq_ignore_ascii_case("AVG") {
            Some(Self::Avg)
        } else if value.eq_ignore_ascii_case("MIN") {
            Some(Self::Min)
        } else if value.eq_ignore_ascii_case("MAX") {
            Some(Self::Max)
        } else {
            None
        }
    }

    fn keyword(self) -> &'static str {
        match self {
            Self::Count => "COUNT",
            Self::Sum => "SUM",
            Self::Avg => "AVG",
            Self::Min => "MIN",
            Self::Max => "MAX",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SqlLiteralType {
    Integer,
    Real,
    String,
}

impl SqlLiteralType {
    fn logical_type(self) -> Type {
        match self {
            Self::Integer => Type::I64,
            Self::Real => Type::F64,
            Self::String => Type::Str,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlPredicate {
    pub column: String,
    pub operator: String,
    pub placeholder: Option<usize>,
    pub placeholder_span: Option<Range<usize>>,
    pub literal_type: Option<SqlLiteralType>,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlOrder {
    pub column: String,
    pub descending: bool,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlGroup {
    pub column: String,
    pub span: Range<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlSelect {
    pub projections: Vec<SqlProjection>,
    pub select_all: bool,
    pub table: String,
    pub table_alias: Option<String>,
    pub table_alias_span: Option<Range<usize>>,
    pub table_span: Range<usize>,
    pub joins: Vec<SqlJoin>,
    pub predicates: Vec<SqlPredicate>,
    pub groups: Vec<SqlGroup>,
    pub having: Vec<SqlPredicate>,
    pub order: Vec<SqlOrder>,
    pub limit_placeholder: Option<usize>,
    pub offset_placeholder: Option<usize>,
    pub parameter_count: usize,
    pub parameter_spans: Vec<Range<usize>>,
    pub source_len: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlJoin {
    pub kind: SqlJoinKind,
    pub table: String,
    pub table_span: Range<usize>,
    pub alias: Option<String>,
    pub alias_span: Option<Range<usize>>,
    pub left_column: String,
    pub right_column: String,
    pub span: Range<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SqlJoinKind {
    Inner,
    Left,
    Right,
    Full,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SqlParseError {
    pub message: String,
    pub span: Range<usize>,
}

pub fn tokenize_sql(source: &str) -> Result<Vec<SqlToken>, SqlParseError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        let start = cursor;
        let byte = bytes[cursor];
        if byte.is_ascii_whitespace() {
            cursor += 1;
            continue;
        }
        if byte == b'/' && bytes.get(cursor + 1) == Some(&b'*') {
            cursor += 2;
            while cursor + 1 < bytes.len() && !(bytes[cursor] == b'*' && bytes[cursor + 1] == b'/')
            {
                cursor += 1;
            }
            if cursor + 1 >= bytes.len() {
                return Err(sql_error("unterminated SQL comment", start..bytes.len()));
            }
            cursor += 2;
            continue;
        }
        let kind = match byte {
            b'*' => {
                cursor += 1;
                SqlTokenKind::Star
            }
            b',' => {
                cursor += 1;
                SqlTokenKind::Comma
            }
            b'.' => {
                cursor += 1;
                SqlTokenKind::Dot
            }
            b'(' => {
                cursor += 1;
                SqlTokenKind::LeftParen
            }
            b')' => {
                cursor += 1;
                SqlTokenKind::RightParen
            }
            b'?' => {
                cursor += 1;
                SqlTokenKind::Placeholder
            }
            b';' => {
                return Err(sql_error(
                    "only one SQL statement is allowed",
                    start..start + 1,
                ));
            }
            b'\'' => {
                cursor += 1;
                loop {
                    if cursor >= bytes.len() {
                        return Err(sql_error(
                            "unterminated SQL string literal",
                            start..bytes.len(),
                        ));
                    }
                    if bytes[cursor] == b'\'' {
                        cursor += 1;
                        if bytes.get(cursor) == Some(&b'\'') {
                            cursor += 1;
                        } else {
                            break;
                        }
                    } else {
                        cursor += 1;
                    }
                }
                SqlTokenKind::String
            }
            b'"' | b'`' | b'[' => {
                let terminator = if byte == b'[' { b']' } else { byte };
                cursor += 1;
                let content_start = cursor;
                while cursor < bytes.len() && bytes[cursor] != terminator {
                    cursor += 1;
                }
                if cursor == bytes.len() {
                    return Err(sql_error(
                        "unterminated quoted SQL identifier",
                        start..bytes.len(),
                    ));
                }
                let text = source[content_start..cursor].to_string();
                cursor += 1;
                tokens.push(SqlToken {
                    kind: SqlTokenKind::Word,
                    text,
                    span: start..cursor,
                });
                continue;
            }
            b'=' | b'!' | b'<' | b'>' => {
                cursor += 1;
                if matches!(bytes.get(cursor), Some(b'=' | b'>' | b'<')) {
                    cursor += 1;
                }
                SqlTokenKind::Operator
            }
            b'0'..=b'9' => {
                cursor += 1;
                while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
                    cursor += 1;
                }
                SqlTokenKind::Number
            }
            _ if byte.is_ascii_alphabetic() || byte == b'_' => {
                cursor += 1;
                while bytes
                    .get(cursor)
                    .is_some_and(|next| next.is_ascii_alphanumeric() || *next == b'_')
                {
                    cursor += 1;
                }
                SqlTokenKind::Word
            }
            _ => {
                return Err(sql_error(
                    "unsupported character in SQL statement",
                    start..start + 1,
                ));
            }
        };
        tokens.push(SqlToken {
            kind,
            text: source[start..cursor].to_string(),
            span: start..cursor,
        });
    }
    Ok(tokens)
}

pub fn parse_select_sql(source: &str) -> Result<SqlSelect, SqlParseError> {
    let tokens = tokenize_sql(source)?;
    let mut parser = SqlParser {
        tokens,
        cursor: 0,
        parameter_count: 0,
        parameter_spans: Vec::new(),
        source_len: source.len(),
    };
    parser.parse_select()
}

struct SqlParser {
    tokens: Vec<SqlToken>,
    cursor: usize,
    parameter_count: usize,
    parameter_spans: Vec<Range<usize>>,
    source_len: usize,
}

impl SqlParser {
    fn parse_select(&mut self) -> Result<SqlSelect, SqlParseError> {
        self.expect_keyword("SELECT")?;
        let (select_all, projections) = self.parse_projections()?;
        self.expect_keyword("FROM")?;
        let table_token = self.expect_word("expected a table name after FROM")?;
        let table = table_token.text.clone();
        let (table_alias, table_alias_span) = self.parse_table_alias()?;
        let mut joins = Vec::new();
        loop {
            let kind = if self.consume_keyword("JOIN") {
                SqlJoinKind::Inner
            } else if self.consume_keyword("INNER") {
                self.expect_keyword("JOIN")?;
                SqlJoinKind::Inner
            } else if self.consume_keyword("LEFT") {
                self.consume_keyword("OUTER");
                self.expect_keyword("JOIN")?;
                SqlJoinKind::Left
            } else if self.consume_keyword("RIGHT") {
                self.consume_keyword("OUTER");
                self.expect_keyword("JOIN")?;
                SqlJoinKind::Right
            } else if self.consume_keyword("FULL") {
                self.consume_keyword("OUTER");
                self.expect_keyword("JOIN")?;
                SqlJoinKind::Full
            } else {
                break;
            };
            let joined_table = self.expect_word("expected a table name after JOIN")?;
            let (alias, alias_span) = self.parse_table_alias()?;
            self.expect_keyword("ON")?;
            let (left_column, left_span) =
                self.parse_column_reference("expected a column in JOIN ON")?;
            let operator =
                self.expect_kind(SqlTokenKind::Operator, "expected `=` between JOIN columns")?;
            if operator.text != "=" {
                return Err(sql_error(
                    "JOIN ON currently supports equality comparisons only",
                    operator.span,
                ));
            }
            let (right_column, right_span) =
                self.parse_column_reference("expected a column in JOIN ON")?;
            joins.push(SqlJoin {
                kind,
                table: joined_table.text,
                table_span: joined_table.span,
                alias,
                alias_span,
                left_column,
                right_column,
                span: left_span.start..right_span.end,
            });
        }
        let mut predicates = Vec::new();
        if self.consume_keyword("WHERE") {
            self.parse_predicate_or(&mut predicates, "WHERE")?;
        }
        let mut groups = Vec::new();
        if self.consume_keyword("GROUP") {
            self.expect_keyword("BY")?;
            loop {
                let (column, span) =
                    self.parse_column_reference("expected a column name after GROUP BY")?;
                groups.push(SqlGroup { column, span });
                if !self.consume_kind(SqlTokenKind::Comma) {
                    break;
                }
            }
        }
        let mut having = Vec::new();
        if self.consume_keyword("HAVING") {
            self.parse_predicate_or(&mut having, "HAVING")?;
        }
        let mut order = Vec::new();
        if self.consume_keyword("ORDER") {
            self.expect_keyword("BY")?;
            loop {
                let (column, span) =
                    self.parse_column_reference("expected a column name after ORDER BY")?;
                let descending = if self.consume_keyword("DESC") {
                    true
                } else {
                    self.consume_keyword("ASC");
                    false
                };
                order.push(SqlOrder {
                    column,
                    descending,
                    span,
                });
                if !self.consume_kind(SqlTokenKind::Comma) {
                    break;
                }
            }
        }
        let limit_placeholder = if self.consume_keyword("LIMIT") {
            self.parse_bound_value("LIMIT")?
        } else {
            None
        };
        let offset_placeholder = if self.consume_keyword("OFFSET") {
            self.parse_bound_value("OFFSET")?
        } else {
            None
        };
        if let Some(token) = self.peek() {
            return Err(sql_error(
                "unsupported SQL syntax after the supported SELECT clauses",
                token.span.clone(),
            ));
        }
        Ok(SqlSelect {
            projections,
            select_all,
            table,
            table_alias,
            table_alias_span,
            table_span: table_token.span,
            joins,
            predicates,
            groups,
            having,
            order,
            limit_placeholder,
            offset_placeholder,
            parameter_count: self.parameter_count,
            parameter_spans: std::mem::take(&mut self.parameter_spans),
            source_len: self.source_len,
        })
    }

    fn parse_projections(&mut self) -> Result<(bool, Vec<SqlProjection>), SqlParseError> {
        if self.consume_kind(SqlTokenKind::Star) {
            return Ok((true, Vec::new()));
        }
        let mut projections = Vec::new();
        loop {
            let aggregate = self
                .peek()
                .filter(|token| {
                    SqlAggregate::from_keyword(&token.text).is_some()
                        && self
                            .tokens
                            .get(self.cursor + 1)
                            .is_some_and(|next| next.kind == SqlTokenKind::LeftParen)
                })
                .and_then(|token| {
                    SqlAggregate::from_keyword(&token.text).map(|kind| (kind, token.span.start))
                });
            let (name, span, aggregate) = if let Some((kind, start)) = aggregate {
                self.advance();
                self.expect_kind(SqlTokenKind::LeftParen, "expected `(` after aggregate")?;
                let (name, _) = if self.consume_kind(SqlTokenKind::Star) {
                    if kind != SqlAggregate::Count {
                        let end = self
                            .tokens
                            .get(self.cursor)
                            .map_or(self.tokens[self.cursor - 1].span.end, |token| {
                                token.span.end
                            });
                        return Err(sql_error(
                            &format!("{} requires a column argument", kind.keyword()),
                            start..end,
                        ));
                    }
                    ("*".to_string(), start..start + 1)
                } else {
                    self.parse_column_reference("expected a column name in aggregate")?
                };
                let closing = self.expect_kind(
                    SqlTokenKind::RightParen,
                    "expected `)` after aggregate argument",
                )?;
                (name, start..closing.span.end, Some(kind))
            } else {
                let (name, span) =
                    self.parse_column_reference("expected a column name in SELECT")?;
                (name, span, None)
            };
            let has_as = self.consume_keyword("AS");
            if let Some(aggregate) = aggregate.filter(|_| !has_as) {
                return Err(sql_error(
                    &format!("{} requires an `AS` alias", aggregate.keyword()),
                    span.clone(),
                ));
            }
            let alias = if has_as {
                let alias_token = self.expect_word("expected a column alias after AS")?;
                Some((alias_token.text, alias_token.span))
            } else {
                None
            };
            let (alias, alias_span) = alias
                .map(|(alias, span)| (Some(alias), Some(span)))
                .unwrap_or((None, None));
            projections.push(SqlProjection {
                name,
                alias,
                aggregate,
                span,
                alias_span,
            });
            if !self.consume_kind(SqlTokenKind::Comma) {
                break;
            }
        }
        Ok((false, projections))
    }

    fn parse_predicate(&mut self, clause: &str) -> Result<SqlPredicate, SqlParseError> {
        let (column, column_span) =
            self.parse_column_reference(&format!("expected a column name in {clause}"))?;
        let (operator, operator_span) = if self.consume_keyword("LIKE") {
            let token = &self.tokens[self.cursor - 1];
            ("LIKE".to_string(), token.span.clone())
        } else {
            let token = self.expect_kind(
                SqlTokenKind::Operator,
                &format!("expected a comparison operator in {clause}"),
            )?;
            (token.text, token.span)
        };
        if !matches!(
            operator.as_str(),
            "=" | "!=" | "<>" | "<" | "<=" | ">" | ">=" | "LIKE"
        ) {
            return Err(sql_error(
                &format!("unsupported comparison operator in {clause}"),
                operator_span,
            ));
        }
        let (placeholder, placeholder_span, literal_type) = if self
            .peek()
            .is_some_and(|token| token.kind == SqlTokenKind::Placeholder)
        {
            let token = self.advance().expect("peeked placeholder exists");
            let index = self.parameter_count;
            self.parameter_count += 1;
            self.parameter_spans.push(token.span.clone());
            (Some(index), Some(token.span), None)
        } else {
            let value = self.peek().cloned().ok_or_else(|| {
                sql_error(
                    &format!("expected a value in {clause}"),
                    operator_span.end..operator_span.end,
                )
            })?;
            if !matches!(value.kind, SqlTokenKind::String | SqlTokenKind::Number) {
                return Err(sql_error(
                    &format!("{clause} values must be literals or `?` parameters"),
                    value.span,
                ));
            }
            self.advance();
            let literal_type = match value.kind {
                SqlTokenKind::String => SqlLiteralType::String,
                SqlTokenKind::Number
                    if value.text.contains('.')
                        || value.text.contains('e')
                        || value.text.contains('E') =>
                {
                    SqlLiteralType::Real
                }
                SqlTokenKind::Number => SqlLiteralType::Integer,
                _ => unreachable!("predicate literal kind was validated"),
            };
            (None, None, Some(literal_type))
        };
        let end = self
            .tokens
            .get(self.cursor.saturating_sub(1))
            .map_or(operator_span.end, |token| token.span.end);
        Ok(SqlPredicate {
            column,
            operator,
            placeholder,
            placeholder_span,
            literal_type,
            span: column_span.start..end,
        })
    }

    fn parse_predicate_or(
        &mut self,
        predicates: &mut Vec<SqlPredicate>,
        clause: &str,
    ) -> Result<(), SqlParseError> {
        self.parse_predicate_and(predicates, clause)?;
        while self.consume_keyword("OR") {
            self.parse_predicate_and(predicates, clause)?;
        }
        Ok(())
    }

    fn parse_predicate_and(
        &mut self,
        predicates: &mut Vec<SqlPredicate>,
        clause: &str,
    ) -> Result<(), SqlParseError> {
        self.parse_predicate_primary(predicates, clause)?;
        while self.consume_keyword("AND") {
            self.parse_predicate_primary(predicates, clause)?;
        }
        Ok(())
    }

    fn parse_predicate_primary(
        &mut self,
        predicates: &mut Vec<SqlPredicate>,
        clause: &str,
    ) -> Result<(), SqlParseError> {
        if self.consume_kind(SqlTokenKind::LeftParen) {
            self.parse_predicate_or(predicates, clause)?;
            self.expect_kind(
                SqlTokenKind::RightParen,
                &format!("expected `)` after grouped {clause} conditions"),
            )?;
        } else {
            predicates.push(self.parse_predicate(clause)?);
        }
        Ok(())
    }

    fn parse_table_alias(
        &mut self,
    ) -> Result<(Option<String>, Option<Range<usize>>), SqlParseError> {
        if self.consume_keyword("AS") {
            return self
                .expect_word("expected a table alias")
                .map(|alias| (Some(alias.text), Some(alias.span)));
        }
        if self.peek().is_some_and(|token| {
            token.kind == SqlTokenKind::Word
                && !is_clause_keyword(&token.text)
                && !token.text.eq_ignore_ascii_case("JOIN")
                && !token.text.eq_ignore_ascii_case("INNER")
                && !token.text.eq_ignore_ascii_case("OUTER")
                && !token.text.eq_ignore_ascii_case("ON")
        }) {
            return Ok(self
                .advance()
                .map_or((None, None), |alias| (Some(alias.text), Some(alias.span))));
        }
        Ok((None, None))
    }

    fn parse_column_reference(
        &mut self,
        message: &str,
    ) -> Result<(String, Range<usize>), SqlParseError> {
        let first = self.expect_word(message)?;
        if !self.consume_kind(SqlTokenKind::Dot) {
            return Ok((first.text, first.span));
        }
        let second = self.expect_word("expected a column name after `.`")?;
        Ok((
            format!("{}.{}", first.text, second.text),
            first.span.start..second.span.end,
        ))
    }

    fn parse_bound_value(&mut self, clause: &str) -> Result<Option<usize>, SqlParseError> {
        if self
            .peek()
            .is_some_and(|token| token.kind == SqlTokenKind::Placeholder)
        {
            let token = self.advance().expect("peeked placeholder exists");
            let index = self.parameter_count;
            self.parameter_count += 1;
            self.parameter_spans.push(token.span);
            Ok(Some(index))
        } else if self
            .peek()
            .is_some_and(|token| token.kind == SqlTokenKind::Number)
        {
            self.advance();
            Ok(None)
        } else {
            let span = self
                .peek()
                .map_or(self.source_len..self.source_len, |token| token.span.clone());
            Err(sql_error(
                &format!("{clause} must be a nonnegative integer or `?` parameter"),
                span,
            ))
        }
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<(), SqlParseError> {
        if self.consume_keyword(keyword) {
            Ok(())
        } else {
            Err(self.current_error(&format!("expected `{keyword}`")))
        }
    }
    fn expect_word(&mut self, message: &str) -> Result<SqlToken, SqlParseError> {
        self.expect_kind(SqlTokenKind::Word, message)
    }
    fn expect_kind(
        &mut self,
        kind: SqlTokenKind,
        message: &str,
    ) -> Result<SqlToken, SqlParseError> {
        if self.peek().is_some_and(|token| token.kind == kind) {
            Ok(self.advance().expect("peeked SQL token exists"))
        } else {
            Err(self.current_error(message))
        }
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
    fn consume_kind(&mut self, kind: SqlTokenKind) -> bool {
        if self.peek().is_some_and(|token| token.kind == kind) {
            self.cursor += 1;
            true
        } else {
            false
        }
    }
    fn peek(&self) -> Option<&SqlToken> {
        self.tokens.get(self.cursor)
    }
    fn advance(&mut self) -> Option<SqlToken> {
        let token = self.tokens.get(self.cursor).cloned()?;
        self.cursor += 1;
        Some(token)
    }
    fn current_error(&self, message: &str) -> SqlParseError {
        sql_error(
            message,
            self.peek()
                .map_or(self.source_len..self.source_len, |token| token.span.clone()),
        )
    }
}

fn is_clause_keyword(text: &str) -> bool {
    [
        "WHERE",
        "ORDER",
        "LIMIT",
        "OFFSET",
        "AND",
        "ASC",
        "DESC",
        "JOIN",
        "INNER",
        "OUTER",
        "LEFT",
        "RIGHT",
        "FULL",
        "CROSS",
        "ON",
        "GROUP",
        "HAVING",
        "UNION",
        "EXCEPT",
        "INTERSECT",
    ]
    .iter()
    .any(|keyword| text.eq_ignore_ascii_case(keyword))
}

fn sql_error(message: &str, span: Range<usize>) -> SqlParseError {
    SqlParseError {
        message: message.to_string(),
        span,
    }
}

#[derive(Clone, Debug)]
pub struct SqlDecoderField {
    pub field_name: String,
    pub field_type: Type,
    pub column_name: String,
    pub result_ordinal: u32,
}

#[derive(Clone, Debug)]
pub struct ValidatedSqlSelect {
    pub table: String,
    pub output_fields: Vec<Field>,
    pub selected_columns: Vec<super::schema::DbColumnSchema>,
    pub decoder_fields: Vec<SqlDecoderField>,
}

pub fn select_table_nullability(query: &SqlSelect) -> Vec<bool> {
    let mut nullable = vec![false];
    for join in &query.joins {
        match join.kind {
            SqlJoinKind::Inner => nullable.push(false),
            SqlJoinKind::Left => nullable.push(true),
            SqlJoinKind::Right => {
                nullable.fill(true);
                nullable.push(false);
            }
            SqlJoinKind::Full => {
                nullable.fill(true);
                nullable.push(true);
            }
        }
    }
    nullable
}

pub fn aggregate_projection_type(
    snapshot: &DbSchemaSnapshot,
    query: &SqlSelect,
    projection: &SqlProjection,
) -> Result<Type, SqlParseError> {
    let aggregate = projection
        .aggregate
        .ok_or_else(|| sql_error("projection is not an aggregate", projection.span.clone()))?;
    if aggregate == SqlAggregate::Count && projection.name == "*" {
        return Ok(Type::I64);
    }
    let tables = select_tables(snapshot, query)?;
    let (_, column) = resolve_select_column(&tables, &projection.name, projection.span.clone())?;
    let input = logical_type(&column.logical_type).ok_or_else(|| {
        sql_error(
            &format!(
                "unsupported logical type `{}` for aggregate column `{}`",
                column.logical_type, column.name
            ),
            projection.span.clone(),
        )
    })?;
    let result = match aggregate {
        SqlAggregate::Count => return Ok(Type::I64),
        SqlAggregate::Sum if matches!(input, Type::I64 | Type::F64) => input,
        SqlAggregate::Avg if matches!(input, Type::I64 | Type::F64) => Type::F64,
        SqlAggregate::Min | SqlAggregate::Max
            if matches!(input, Type::I64 | Type::F64 | Type::Str) =>
        {
            input
        }
        _ => {
            return Err(sql_error(
                &format!(
                    "{} does not support column `{}` of type `{input}`",
                    aggregate.keyword(),
                    column.name
                ),
                projection.span.clone(),
            ));
        }
    };
    Ok(Type::Optional(Box::new(result)))
}

pub fn validate_select_against_snapshot(
    snapshot: &DbSchemaSnapshot,
    query: &SqlSelect,
    result_fields: &[Field],
    parameter_types: &[Type],
) -> Result<ValidatedSqlSelect, SqlParseError> {
    validate_select_against_snapshot_with_mappings(
        snapshot,
        query,
        result_fields,
        parameter_types,
        &crate::collections::HashMap::default(),
    )
}

pub fn validate_select_against_snapshot_with_mappings(
    snapshot: &DbSchemaSnapshot,
    query: &SqlSelect,
    result_fields: &[Field],
    parameter_types: &[Type],
    field_columns: &crate::collections::HashMap<String, String>,
) -> Result<ValidatedSqlSelect, SqlParseError> {
    let tables = select_tables(snapshot, query)?;
    let table_nullability = select_table_nullability(query);
    if query.parameter_count != parameter_types.len() {
        let span = query
            .parameter_spans
            .get(parameter_types.len())
            .cloned()
            .unwrap_or(query.source_len..query.source_len);
        return Err(sql_error(
            &format!(
                "query has {} placeholder(s), but {} parameter value(s) were provided",
                query.parameter_count,
                parameter_types.len()
            ),
            span,
        ));
    }
    for (join_index, join) in query.joins.iter().enumerate() {
        let (left_table, left) =
            resolve_select_column(&tables, &join.left_column, join.span.clone())?;
        let (right_table, right) =
            resolve_select_column(&tables, &join.right_column, join.span.clone())?;
        let current_table = join_index + 1;
        if !((left_table == current_table && right_table < current_table)
            || (right_table == current_table && left_table < current_table))
        {
            return Err(sql_error(
                "JOIN ON must compare one joined-table column with one previously joined column",
                join.span.clone(),
            ));
        }
        let left_type = logical_type(&left.logical_type).ok_or_else(|| {
            sql_error(
                &format!(
                    "unsupported logical type `{}` for JOIN column `{}`",
                    left.logical_type, left.name
                ),
                join.span.clone(),
            )
        })?;
        let right_type = logical_type(&right.logical_type).ok_or_else(|| {
            sql_error(
                &format!(
                    "unsupported logical type `{}` for JOIN column `{}`",
                    right.logical_type, right.name
                ),
                join.span.clone(),
            )
        })?;
        if left_type != right_type {
            return Err(sql_error(
                &format!(
                    "JOIN columns `{}` and `{}` have incompatible types `{left_type}` and `{right_type}`",
                    join.left_column, join.right_column
                ),
                join.span.clone(),
            ));
        }
    }
    let grouped_columns = validate_grouping(&tables, query)?;
    let selected = select_result_columns(
        snapshot,
        query,
        result_fields,
        field_columns,
        &tables,
        &table_nullability,
    )?;
    let decoder_fields = build_decoder_fields(query, result_fields, &selected, &tables)?;
    validate_select_filters_and_order(snapshot, query, &tables, parameter_types, &grouped_columns)?;
    Ok(ValidatedSqlSelect {
        table: query.table.clone(),
        output_fields: result_fields.to_vec(),
        selected_columns: selected.into_iter().map(|(_, column)| column).collect(),
        decoder_fields,
    })
}

fn select_result_columns(
    snapshot: &DbSchemaSnapshot,
    query: &SqlSelect,
    result_fields: &[Field],
    field_columns: &crate::collections::HashMap<String, String>,
    tables: &[SqlTableBinding<'_>],
    table_nullability: &[bool],
) -> Result<Vec<(usize, super::schema::DbColumnSchema)>, SqlParseError> {
    let mut selected: Vec<(usize, super::schema::DbColumnSchema)> = Vec::new();
    let mut selected_output_names = crate::collections::HashSet::default();
    if query
        .projections
        .iter()
        .any(|projection| projection.aggregate.is_some())
        && query.groups.is_empty()
        && let Some(column) = query
            .projections
            .iter()
            .find(|projection| projection.aggregate.is_none())
    {
        return Err(sql_error(
            "columns and aggregates cannot be mixed without GROUP BY",
            column.span.clone(),
        ));
    }
    if query.select_all {
        for field in result_fields {
            let column_name = field_columns
                .get(&field.name)
                .map_or(field.name.as_str(), String::as_str);
            let (table_index, column) =
                resolve_select_column(&tables, column_name, query.table_span.clone())?;
            let mut selected_column = column.clone();
            selected_column.nullable |= table_nullability[table_index];
            selected.push((table_index, selected_column));
        }
    } else {
        for projection in &query.projections {
            let output_name = projection_output_name(projection);
            if !selected_output_names.insert(output_name.to_string()) {
                return Err(sql_error(
                    &format!("selected column name `{output_name}` is ambiguous"),
                    projection
                        .alias_span
                        .clone()
                        .unwrap_or_else(|| projection.span.clone()),
                ));
            }
            let result_name = model_result_field_name(output_name);
            if result_fields.iter().all(|field| field.name != result_name) {
                return Err(sql_error(
                    &format!("selected column `{output_name}` is not present in the result type"),
                    projection.span.clone(),
                ));
            }
            if projection.aggregate.is_some() {
                let aggregate_type = aggregate_projection_type(snapshot, query, projection)?;
                let (base_type, nullable) = match aggregate_type {
                    Type::Optional(inner) => (*inner, true),
                    other => (other, false),
                };
                let logical_type = match base_type {
                    Type::I64 => "i64",
                    Type::F64 => "f64",
                    Type::Str => "string",
                    _ => {
                        return Err(sql_error(
                            "unsupported aggregate result type",
                            projection.span.clone(),
                        ));
                    }
                };
                selected.push((
                    0,
                    super::schema::DbColumnSchema {
                        name: output_name.to_string(),
                        logical_type: logical_type.to_string(),
                        nullable,
                        ordinal: u32::try_from(selected.len()).map_err(|_| {
                            sql_error(
                                "selected column ordinal exceeds the supported range",
                                projection.span.clone(),
                            )
                        })?,
                        default: None,
                        native_type: None,
                    },
                ));
            } else {
                let (table_index, column) =
                    resolve_select_column(&tables, &projection.name, projection.span.clone())?;
                let mut selected_column = column.clone();
                selected_column.nullable |= table_nullability[table_index];
                selected.push((table_index, selected_column));
            }
        }
    }
    Ok(selected)
}

fn build_decoder_fields(
    query: &SqlSelect,
    result_fields: &[Field],
    selected: &[(usize, super::schema::DbColumnSchema)],
    tables: &[SqlTableBinding<'_>],
) -> Result<Vec<SqlDecoderField>, SqlParseError> {
    let mut decoder_fields = Vec::with_capacity(result_fields.len());
    for (field_index, field) in result_fields.iter().enumerate() {
        let selected_entry = selected
            .iter()
            .enumerate()
            .find(|(index, _)| {
                if query.select_all {
                    *index == field_index
                } else {
                    model_result_field_name(projection_output_name(&query.projections[*index]))
                        == field.name
                }
            })
            .ok_or_else(|| {
                let span = query.projections.last().map_or_else(
                    || query.table_span.clone(),
                    |projection| projection.span.clone(),
                );
                sql_error(
                    &format!("result field `{}` is not selected by the query", field.name),
                    span,
                )
            })?;
        let (selected_ordinal, (table_index, column)) = selected_entry;
        let table_index = *table_index;
        let span = if query.select_all {
            query.table_span.clone()
        } else {
            query
                .projections
                .iter()
                .find(|projection| {
                    model_result_field_name(projection_output_name(projection)) == field.name
                })
                .map_or_else(
                    || query.table_span.clone(),
                    |projection| projection.span.clone(),
                )
        };
        let is_model_instant_projection = !query.select_all
            && model_result_field_name(projection_output_name(
                &query.projections[selected_ordinal],
            )) != projection_output_name(&query.projections[selected_ordinal]);
        let validation_type = if is_model_instant_projection && column.logical_type == "string" {
            Type::Str
        } else {
            field.ty.clone()
        };
        validate_column_type(column, &validation_type, span.clone())?;
        let result_ordinal = if query.select_all && !query.joins.is_empty() {
            let preceding_columns = tables
                .iter()
                .take(table_index)
                .map(|binding| binding.table.columns.len())
                .sum::<usize>();
            let local_ordinal = tables[table_index]
                .table
                .columns
                .iter()
                .position(|candidate| candidate.name == column.name)
                .ok_or_else(|| {
                    sql_error(
                        "selected JOIN column is missing from the snapshot",
                        span.clone(),
                    )
                })?;
            u32::try_from(preceding_columns + local_ordinal).map_err(|_| {
                sql_error(
                    "selected column ordinal exceeds the supported range",
                    span.clone(),
                )
            })?
        } else {
            u32::try_from(selected_ordinal).map_err(|_| {
                sql_error(
                    "selected column ordinal exceeds the supported range",
                    column_span(query, selected_ordinal),
                )
            })?
        };
        decoder_fields.push(SqlDecoderField {
            field_name: field.name.clone(),
            field_type: field.ty.clone(),
            column_name: column.name.clone(),
            result_ordinal,
        });
    }
    Ok(decoder_fields)
}

fn validate_select_filters_and_order(
    snapshot: &DbSchemaSnapshot,
    query: &SqlSelect,
    tables: &[SqlTableBinding<'_>],
    parameter_types: &[Type],
    grouped_columns: &[(usize, &str)],
) -> Result<(), SqlParseError> {
    for predicate in &query.predicates {
        let (_, column) =
            resolve_select_column(&tables, &predicate.column, predicate.span.clone())?;
        let actual = predicate
            .placeholder
            .map(|index| parameter_types[index].clone())
            .or_else(|| predicate.literal_type.map(SqlLiteralType::logical_type));
        if let Some(actual) = actual {
            let expected = logical_type(&column.logical_type).ok_or_else(|| {
                sql_error(
                    &format!(
                        "unsupported logical type `{}` for column `{}`",
                        column.logical_type, column.name
                    ),
                    predicate.span.clone(),
                )
            })?;
            if predicate.operator == "LIKE" && expected != Type::Str {
                return Err(sql_error(
                    &format!("LIKE requires a string column, found `{}`", column.name),
                    predicate.span.clone(),
                ));
            }
            if !types_compatible(&expected, &actual) {
                let subject = predicate.placeholder.map_or_else(
                    || format!("literal for `{}`", column.name),
                    |index| format!("parameter {} for `{}`", index + 1, column.name),
                );
                return Err(sql_error(
                    &format!("{subject} expects `{expected}`, found `{actual}`"),
                    predicate
                        .placeholder_span
                        .clone()
                        .unwrap_or_else(|| predicate.span.clone()),
                ));
            }
        }
    }
    if !query.having.is_empty() && query.groups.is_empty() {
        let has_aggregate = query
            .projections
            .iter()
            .any(|projection| projection.aggregate.is_some());
        if !has_aggregate {
            return Err(sql_error(
                "HAVING requires GROUP BY or an aggregate projection",
                query.having[0].span.clone(),
            ));
        }
    }
    for predicate in &query.having {
        let expected = if let Some(projection) =
            find_projection_alias(query, &predicate.column, predicate.span.clone())?
        {
            if projection.aggregate.is_some() {
                match aggregate_projection_type(snapshot, query, projection)? {
                    Type::Optional(inner) => *inner,
                    other => other,
                }
            } else {
                let (table_index, column) =
                    resolve_select_column(&tables, &projection.name, predicate.span.clone())?;
                if !grouped_columns.contains(&(table_index, column.name.as_str())) {
                    return Err(sql_error(
                        &format!(
                            "HAVING column `{}` must be grouped or be an aggregate alias",
                            predicate.column
                        ),
                        predicate.span.clone(),
                    ));
                }
                logical_type(&column.logical_type).ok_or_else(|| {
                    sql_error(
                        &format!(
                            "unsupported logical type `{}` for HAVING column `{}`",
                            column.logical_type, column.name
                        ),
                        predicate.span.clone(),
                    )
                })?
            }
        } else {
            let (table_index, column) =
                resolve_select_column(&tables, &predicate.column, predicate.span.clone())?;
            if !grouped_columns.contains(&(table_index, column.name.as_str())) {
                return Err(sql_error(
                    &format!(
                        "HAVING column `{}` must appear in GROUP BY or use an aggregate alias",
                        predicate.column
                    ),
                    predicate.span.clone(),
                ));
            }
            logical_type(&column.logical_type).ok_or_else(|| {
                sql_error(
                    &format!(
                        "unsupported logical type `{}` for HAVING column `{}`",
                        column.logical_type, column.name
                    ),
                    predicate.span.clone(),
                )
            })?
        };
        if predicate.operator == "LIKE" && expected != Type::Str {
            return Err(sql_error(
                "LIKE in HAVING requires a string expression",
                predicate.span.clone(),
            ));
        }
        let actual = predicate
            .placeholder
            .map(|index| parameter_types[index].clone())
            .or_else(|| predicate.literal_type.map(SqlLiteralType::logical_type));
        if let Some(actual) = actual
            && !types_compatible(&expected, &actual)
        {
            let subject = predicate.placeholder.map_or_else(
                || format!("literal in HAVING expression `{}`", predicate.column),
                |index| {
                    format!(
                        "parameter {} for HAVING expression `{}`",
                        index + 1,
                        predicate.column
                    )
                },
            );
            return Err(sql_error(
                &format!("{subject} expects `{expected}`, found `{actual}`"),
                predicate
                    .placeholder_span
                    .clone()
                    .unwrap_or_else(|| predicate.span.clone()),
            ));
        }
    }
    for index in [query.limit_placeholder, query.offset_placeholder]
        .into_iter()
        .flatten()
    {
        let actual = parameter_types
            .get(index)
            .expect("parameter count was validated");
        if !is_integer_type(actual) {
            return Err(sql_error(
                &format!(
                    "parameter {} for LIMIT/OFFSET must be an integer, found `{actual}`",
                    index + 1
                ),
                query.parameter_spans[index].clone(),
            ));
        }
    }
    for order in &query.order {
        if let Some(projection) = find_projection_alias(query, &order.column, order.span.clone())? {
            if projection.aggregate.is_some() {
                continue;
            }
            if !query.groups.is_empty() {
                let column = resolve_select_column(&tables, &projection.name, order.span.clone())?;
                if !grouped_columns.contains(&(column.0, column.1.name.as_str())) {
                    return Err(sql_error(
                        &format!("ORDER BY column `{}` must appear in GROUP BY", order.column),
                        order.span.clone(),
                    ));
                }
            }
            continue;
        }
        let column = resolve_select_column(&tables, &order.column, order.span.clone())?;
        if !query.groups.is_empty()
            && !grouped_columns.contains(&(column.0, column.1.name.as_str()))
        {
            return Err(sql_error(
                &format!("ORDER BY column `{}` must appear in GROUP BY", order.column),
                order.span.clone(),
            ));
        }
    }
    Ok(())
}

fn find_projection_alias<'a>(
    query: &'a SqlSelect,
    reference: &str,
    span: Range<usize>,
) -> Result<Option<&'a SqlProjection>, SqlParseError> {
    if reference.contains('.') {
        return Ok(None);
    }
    let mut aliases = query.projections.iter().filter(|projection| {
        projection
            .alias
            .as_deref()
            .is_some_and(|alias| alias.eq_ignore_ascii_case(reference))
    });
    let first = aliases.next();
    if aliases.next().is_some() {
        return Err(sql_error(
            &format!("projection alias `{reference}` is ambiguous"),
            span,
        ));
    }
    Ok(first)
}

fn validate_grouping<'a>(
    tables: &[SqlTableBinding<'a>],
    query: &'a SqlSelect,
) -> Result<Vec<(usize, &'a str)>, SqlParseError> {
    if query.groups.is_empty() {
        return Ok(Vec::new());
    }
    if query.select_all {
        return Err(sql_error(
            "GROUP BY requires explicit SELECT projections",
            query.groups[0].span.clone(),
        ));
    }
    let mut grouped = Vec::with_capacity(query.groups.len());
    for group in &query.groups {
        let reference = if let Some(projection) =
            find_projection_alias(query, &group.column, group.span.clone())?
        {
            if projection.aggregate.is_some() {
                return Err(sql_error(
                    &format!("GROUP BY cannot use aggregate alias `{}`", group.column),
                    group.span.clone(),
                ));
            }
            let physical_matches = tables
                .iter()
                .enumerate()
                .filter_map(|(index, binding)| {
                    binding
                        .table
                        .columns
                        .iter()
                        .find(|column| column.name.eq_ignore_ascii_case(&group.column))
                        .map(|column| (index, column.name.as_str()))
                })
                .collect::<Vec<_>>();
            if !physical_matches.is_empty() {
                let alias_column =
                    resolve_select_column(tables, &projection.name, group.span.clone())?;
                if physical_matches.as_slice() != [(alias_column.0, alias_column.1.name.as_str())] {
                    return Err(sql_error(
                        &format!(
                            "GROUP BY alias `{}` conflicts with a physical column",
                            group.column
                        ),
                        group.span.clone(),
                    ));
                }
            }
            projection.name.as_str()
        } else {
            group.column.as_str()
        };
        let (table_index, column) = resolve_select_column(tables, reference, group.span.clone())?;
        if grouped.contains(&(table_index, column.name.as_str())) {
            return Err(sql_error(
                &format!("duplicate GROUP BY column `{}`", group.column),
                group.span.clone(),
            ));
        }
        grouped.push((table_index, column.name.as_str()));
    }
    for projection in &query.projections {
        if projection.aggregate.is_some() {
            continue;
        }
        let (table_index, column) =
            resolve_select_column(tables, &projection.name, projection.span.clone())?;
        if !grouped.contains(&(table_index, column.name.as_str())) {
            return Err(sql_error(
                &format!(
                    "selected column `{}` must appear in GROUP BY",
                    projection.name
                ),
                projection.span.clone(),
            ));
        }
    }
    Ok(grouped)
}

struct SqlTableBinding<'a> {
    table: &'a super::schema::DbTableSchema,
    name: &'a str,
    alias: Option<&'a str>,
    alias_span: Option<Range<usize>>,
    table_span: Range<usize>,
}

fn select_tables<'a>(
    snapshot: &'a DbSchemaSnapshot,
    query: &'a SqlSelect,
) -> Result<Vec<SqlTableBinding<'a>>, SqlParseError> {
    let mut bindings = Vec::with_capacity(query.joins.len() + 1);
    let base = snapshot
        .tables
        .iter()
        .find(|table| table.name == query.table)
        .ok_or_else(|| {
            sql_error(
                &format!("unknown table `{}`", query.table),
                query.table_span.clone(),
            )
        })?;
    bindings.push(SqlTableBinding {
        table: base,
        name: &query.table,
        alias: query.table_alias.as_deref(),
        alias_span: query.table_alias_span.clone(),
        table_span: query.table_span.clone(),
    });
    for join in &query.joins {
        let table = snapshot
            .tables
            .iter()
            .find(|table| table.name == join.table)
            .ok_or_else(|| {
                sql_error(
                    &format!("unknown table `{}`", join.table),
                    join.table_span.clone(),
                )
            })?;
        bindings.push(SqlTableBinding {
            table,
            name: &join.table,
            alias: join.alias.as_deref(),
            alias_span: join.alias_span.clone(),
            table_span: join.table_span.clone(),
        });
    }
    for index in 0..bindings.len() {
        let left = bindings[index].alias.unwrap_or(bindings[index].name);
        if bindings[..index].iter().any(|previous| {
            previous
                .alias
                .unwrap_or(previous.name)
                .eq_ignore_ascii_case(left)
        }) {
            return Err(sql_error(
                &format!("duplicate table alias `{left}`"),
                bindings[index]
                    .alias_span
                    .clone()
                    .unwrap_or_else(|| bindings[index].table_span.clone()),
            ));
        }
    }
    Ok(bindings)
}

fn resolve_select_column<'a>(
    tables: &[SqlTableBinding<'a>],
    reference: &str,
    span: Range<usize>,
) -> Result<(usize, &'a super::schema::DbColumnSchema), SqlParseError> {
    let (qualifier, column_name) = reference
        .split_once('.')
        .map_or((None, reference), |(qualifier, column)| {
            (Some(qualifier), column)
        });
    let matches = tables
        .iter()
        .enumerate()
        .filter(|(_, binding)| {
            qualifier.is_none_or(|qualifier| {
                binding
                    .alias
                    .unwrap_or(binding.name)
                    .eq_ignore_ascii_case(qualifier)
            })
        })
        .filter_map(|(index, binding)| {
            binding
                .table
                .columns
                .iter()
                .find(|column| column.name.eq_ignore_ascii_case(column_name))
                .map(|column| (index, column))
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [matched] => Ok(*matched),
        [] => Err(sql_error(&format!("unknown column `{reference}`"), span)),
        _ => Err(sql_error(
            &format!("column `{reference}` is ambiguous; qualify it with a table name or alias"),
            span,
        )),
    }
}

fn projection_output_name(projection: &SqlProjection) -> &str {
    projection.alias.as_deref().unwrap_or_else(|| {
        projection
            .name
            .rsplit_once('.')
            .map_or(projection.name.as_str(), |(_, column)| column)
    })
}

fn model_result_field_name(output_name: &str) -> &str {
    output_name
        .strip_prefix(super::orm::INSTANT_ALIAS_PREFIX)
        .unwrap_or(output_name)
}

fn column_span(query: &SqlSelect, ordinal: usize) -> Range<usize> {
    query.projections.get(ordinal).map_or_else(
        || query.table_span.clone(),
        |projection| projection.span.clone(),
    )
}

fn validate_column_type(
    column: &super::schema::DbColumnSchema,
    target: &Type,
    span: Range<usize>,
) -> Result<(), SqlParseError> {
    let expected = logical_type(&column.logical_type).ok_or_else(|| {
        sql_error(
            &format!(
                "unsupported logical type `{}` for column `{}`",
                column.logical_type, column.name
            ),
            span.clone(),
        )
    })?;
    let target_base = match target {
        Type::Optional(inner) => inner.as_ref(),
        other => other,
    };
    if !types_compatible(&expected, target_base) {
        return Err(sql_error(
            &format!(
                "column `{}` has type `{expected}` but the result field expects `{target}`",
                column.name
            ),
            span.clone(),
        ));
    }
    if column.nullable && !matches!(target, Type::Optional(_)) {
        return Err(sql_error(
            &format!(
                "nullable column `{}` requires an optional result field",
                column.name
            ),
            span,
        ));
    }
    Ok(())
}

fn logical_type(name: &str) -> Option<Type> {
    match name {
        "i64" => Some(Type::I64),
        "u64" => Some(Type::U64),
        "f64" => Some(Type::F64),
        "boolean" => Some(Type::Bool),
        "bytes" => Some(Type::Array(Box::new(Type::U8))),
        "string" => Some(Type::Str),
        "instant" => Some(Type::Named("Instant".to_string())),
        _ => None,
    }
}

fn is_integer_type(ty: &Type) -> bool {
    matches!(
        ty,
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
            | Type::Usize
    )
}

fn types_compatible(expected: &Type, actual: &Type) -> bool {
    expected == actual
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{DbColumnSchema, DbSchemaSnapshot, DbTableSchema};

    fn field(name: &str, ty: Type) -> Field {
        Field {
            name: name.to_string(),
            ty,
            readonly: false,
            optional: false,
            decorators: Vec::new(),
            span: crate::span::Span::DUMMY,
        }
    }

    #[test]
    fn validates_internal_instant_aliases_against_model_result_fields() {
        let snapshot = DbSchemaSnapshot::new(
            "./app.db".to_string(),
            1,
            vec![DbTableSchema {
                name: "users".to_string(),
                engine: None,
                columns: vec![DbColumnSchema {
                    name: "createdAt".to_string(),
                    logical_type: "string".to_string(),
                    nullable: false,
                    ordinal: 0,
                    default: None,
                    native_type: None,
                }],
                primary_key: Vec::new(),
                unique: Vec::new(),
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                triggers: Vec::new(),
            }],
        );
        let sql = "SELECT createdAt AS \"__tyrus_instant__createdAt\" FROM users";
        let query = parse_select_sql(sql).unwrap();
        let validated = validate_select_against_snapshot(
            &snapshot,
            &query,
            &[field("createdAt", Type::Named("Instant".to_string()))],
            &[],
        )
        .unwrap();

        assert_eq!(validated.decoder_fields[0].field_name, "createdAt");
        assert_eq!(validated.decoder_fields[0].column_name, "createdAt");
        assert_eq!(validated.decoder_fields[0].result_ordinal, 0);
    }

    #[test]
    fn validates_internal_instant_aliases_against_native_temporal_columns() {
        let snapshot = DbSchemaSnapshot::new(
            "postgresql://localhost/app".to_string(),
            1,
            vec![DbTableSchema {
                name: "users".to_string(),
                engine: None,
                columns: vec![DbColumnSchema {
                    name: "createdAt".to_string(),
                    logical_type: "instant".to_string(),
                    nullable: false,
                    ordinal: 0,
                    default: None,
                    native_type: Some("timestamptz".to_string()),
                }],
                primary_key: Vec::new(),
                unique: Vec::new(),
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                triggers: Vec::new(),
            }],
        );
        let sql = "SELECT createdAt AS \"__tyrus_instant__createdAt\" FROM users";
        let query = parse_select_sql(sql).unwrap();
        let validated = validate_select_against_snapshot(
            &snapshot,
            &query,
            &[field("createdAt", Type::Named("Instant".to_string()))],
            &[],
        )
        .unwrap();

        assert_eq!(validated.decoder_fields[0].field_name, "createdAt");
        assert_eq!(
            validated.decoder_fields[0].field_type,
            Type::Named("Instant".to_string())
        );
        assert_eq!(validated.decoder_fields[0].column_name, "createdAt");
    }

    #[test]
    fn parses_simple_select_and_records_placeholder_ranges() {
        let sql =
            "SELECT id, name AS displayName FROM users WHERE id = ? ORDER BY name DESC LIMIT ?";
        let parsed = parse_select_sql(sql).unwrap();
        assert_eq!(parsed.table, "users");
        assert_eq!(parsed.parameter_count, 2);
        assert_eq!(parsed.predicates[0].placeholder, Some(0));
        assert_eq!(&sql[parsed.parameter_spans[0].clone()], "?");
        assert_eq!(&sql[parsed.parameter_spans[1].clone()], "?");
        assert_eq!(parsed.order[0].column, "name");
        assert!(parsed.order[0].descending);
        assert_eq!(&sql[parsed.table_span.clone()], "users");
    }

    #[test]
    fn aggregate_projections_require_aliases_and_validate_result_types() {
        let snapshot = DbSchemaSnapshot::new(
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
                ],
                primary_key: vec!["id".to_string()],
                unique: Vec::new(),
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                triggers: Vec::new(),
            }],
        );
        let sql = "SELECT COUNT(*) AS total, COUNT(id) AS matching FROM users WHERE id >= ?";
        let parsed = parse_select_sql(sql).unwrap();
        let fields = [field("total", Type::I64), field("matching", Type::I64)];
        let validated =
            validate_select_against_snapshot(&snapshot, &parsed, &fields, &[Type::I64]).unwrap();
        assert_eq!(validated.decoder_fields[0].field_name, "total");
        assert_eq!(validated.decoder_fields[1].result_ordinal, 1);

        let missing_alias = "SELECT COUNT(*) FROM users";
        let error = parse_select_sql(missing_alias).unwrap_err();
        assert_eq!(&missing_alias[error.span], "COUNT(*)");
        let missing_column = "SELECT COUNT(missing) AS total FROM users";
        let parsed = parse_select_sql(missing_column).unwrap();
        let error =
            validate_select_against_snapshot(&snapshot, &parsed, &fields[..1], &[]).unwrap_err();
        assert_eq!(&missing_column[error.span], "COUNT(missing)");
        let wrong_type = validate_select_against_snapshot(
            &snapshot,
            &parse_select_sql("SELECT COUNT(*) AS total FROM users").unwrap(),
            &[field("total", Type::Str)],
            &[],
        )
        .unwrap_err();
        assert!(wrong_type.message.contains("type `i64`"));
        let mixed = "SELECT COUNT(*) AS total, id FROM users";
        let parsed = parse_select_sql(mixed).unwrap();
        let error = validate_select_against_snapshot(
            &snapshot,
            &parsed,
            &[field("total", Type::I64), field("id", Type::I64)],
            &[],
        )
        .unwrap_err();
        assert_eq!(&mixed[error.span], "id");

        let aggregate_sql = "SELECT SUM(id) AS sum, AVG(id) AS average, MIN(name) AS firstName, MAX(id) AS maximum FROM users";
        let parsed = parse_select_sql(aggregate_sql).unwrap();
        let aggregate_fields = [
            field("sum", Type::Optional(Box::new(Type::I64))),
            field("average", Type::Optional(Box::new(Type::F64))),
            field("firstName", Type::Optional(Box::new(Type::Str))),
            field("maximum", Type::Optional(Box::new(Type::I64))),
        ];
        validate_select_against_snapshot(&snapshot, &parsed, &aggregate_fields, &[]).unwrap();
        let wrong_nullability = validate_select_against_snapshot(
            &snapshot,
            &parsed,
            &[
                field("sum", Type::I64),
                aggregate_fields[1].clone(),
                aggregate_fields[2].clone(),
                aggregate_fields[3].clone(),
            ],
            &[],
        )
        .unwrap_err();
        assert_eq!(&aggregate_sql[wrong_nullability.span], "SUM(id)");
        let invalid_sum = "SELECT SUM(name) AS sum FROM users";
        let parsed = parse_select_sql(invalid_sum).unwrap();
        let error = validate_select_against_snapshot(
            &snapshot,
            &parsed,
            &[field("sum", Type::Optional(Box::new(Type::I64)))],
            &[],
        )
        .unwrap_err();
        assert_eq!(&invalid_sum[error.span], "SUM(name)");
        let invalid_star = "SELECT SUM(*) AS sum FROM users";
        let error = parse_select_sql(invalid_star).unwrap_err();
        assert_eq!(&invalid_star[error.span], "SUM(*)");
    }

    #[test]
    fn rejects_multiple_statements_and_unsupported_sql_with_precise_ranges() {
        let sql = "SELECT * FROM users; DELETE FROM users";
        let error = parse_select_sql(sql).unwrap_err();
        assert_eq!(&sql[error.span], ";");
        let unsupported_join = "SELECT * FROM users CROSS JOIN roles ON users.role_id = roles.id";
        let error = parse_select_sql(unsupported_join).unwrap_err();
        assert_eq!(&unsupported_join[error.span], "CROSS");
        let unsupported_operator = "SELECT id FROM users WHERE id => ?";
        let error = parse_select_sql(unsupported_operator).unwrap_err();
        assert_eq!(&unsupported_operator[error.span], "=>");
    }

    #[test]
    fn validates_result_columns_types_nullability_and_parameters_from_snapshot() {
        let snapshot = DbSchemaSnapshot::new(
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
                ],
                primary_key: vec!["id".to_string()],
                unique: Vec::new(),
                foreign_keys: Vec::new(),
                indexes: Vec::new(),
                triggers: Vec::new(),
            }],
        );
        let parsed = parse_select_sql("SELECT id, name FROM users WHERE id = ?").unwrap();
        let mut unsupported_snapshot = snapshot.clone();
        unsupported_snapshot.tables[0].columns[0].logical_type = "jsonb".to_string();
        let unsupported_error = validate_select_against_snapshot(
            &unsupported_snapshot,
            &parsed,
            &[
                field("id", Type::I64),
                field("name", Type::Optional(Box::new(Type::Str))),
            ],
            &[Type::I64],
        )
        .unwrap_err();
        assert!(
            unsupported_error
                .message
                .contains("unsupported logical type `jsonb`")
        );
        assert_eq!(
            &"SELECT id, name FROM users WHERE id = ?"[unsupported_error.span],
            "id"
        );
        let fields = vec![
            field("id", Type::I64),
            field("name", Type::Optional(Box::new(Type::Str))),
        ];
        let result =
            validate_select_against_snapshot(&snapshot, &parsed, &fields, &[Type::I64]).unwrap();
        assert_eq!(result.selected_columns.len(), 2);
        assert_eq!(result.decoder_fields[0].field_name, "id");
        assert_eq!(result.decoder_fields[0].column_name, "id");
        assert_eq!(result.decoder_fields[0].result_ordinal, 0);
        assert_eq!(result.decoder_fields[1].field_name, "name");
        assert_eq!(result.decoder_fields[1].result_ordinal, 1);
        assert!(
            validate_select_against_snapshot(&snapshot, &parsed, &fields, &[Type::Str]).is_err()
        );
        let wrong_parameter =
            validate_select_against_snapshot(&snapshot, &parsed, &fields, &[Type::Str])
                .unwrap_err();
        assert_eq!(
            &"SELECT id, name FROM users WHERE id = ?"[wrong_parameter.span],
            "?"
        );

        let aliased_sql = "SELECT name AS displayName, id FROM users WHERE id = ?";
        let aliased_query = parse_select_sql(aliased_sql).unwrap();
        let aliased_fields = vec![
            field("id", Type::I64),
            field("displayName", Type::Optional(Box::new(Type::Str))),
        ];
        let aliased = validate_select_against_snapshot(
            &snapshot,
            &aliased_query,
            &aliased_fields,
            &[Type::I64],
        )
        .unwrap();
        assert_eq!(aliased.decoder_fields[0].column_name, "id");
        assert_eq!(aliased.decoder_fields[0].result_ordinal, 1);
        assert_eq!(aliased.decoder_fields[1].column_name, "name");
        assert_eq!(aliased.decoder_fields[1].result_ordinal, 0);

        let duplicate_sql = "SELECT id AS value, name AS value FROM users";
        let duplicate = parse_select_sql(duplicate_sql).unwrap();
        let duplicate_error = validate_select_against_snapshot(
            &snapshot,
            &duplicate,
            &[field("value", Type::I64)],
            &[],
        )
        .unwrap_err();
        assert_eq!(&duplicate_sql[duplicate_error.span], "value");
        let non_optional = vec![field("id", Type::I64), field("name", Type::Str)];
        let nullable_error =
            validate_select_against_snapshot(&snapshot, &parsed, &non_optional, &[Type::I64])
                .unwrap_err();
        assert_eq!(
            &"SELECT id, name FROM users WHERE id = ?"[nullable_error.span],
            "name"
        );

        let wrong_type = vec![
            field("id", Type::Str),
            field("name", Type::Optional(Box::new(Type::Str))),
        ];
        let type_error =
            validate_select_against_snapshot(&snapshot, &parsed, &wrong_type, &[Type::I64])
                .unwrap_err();
        assert_eq!(
            &"SELECT id, name FROM users WHERE id = ?"[type_error.span],
            "id"
        );

        let too_many = parse_select_sql("SELECT * FROM users WHERE id = ?").unwrap();
        let count_error =
            validate_select_against_snapshot(&snapshot, &too_many, &[field("id", Type::I64)], &[])
                .unwrap_err();
        assert_eq!(&"SELECT * FROM users WHERE id = ?"[count_error.span], "?");

        let count_error =
            validate_select_against_snapshot(&snapshot, &parsed, &fields, &[]).unwrap_err();
        assert_eq!(
            &"SELECT id, name FROM users WHERE id = ?"[count_error.span],
            "?"
        );
    }
}
