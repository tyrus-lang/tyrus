use super::*;
use std::ops::Range;
use tyrus_core::db::{DbSchemaSnapshot, SqlToken, SqlTokenKind, tokenize_sql};
use tyrus_core::lexer::Token;

pub(super) struct SqlLiteral<'a> {
    pub(super) text: &'a str,
    start: usize,
}

fn sql_identifier(name: &str) -> Option<String> {
    if name.contains('"') || name.chars().any(char::is_control) {
        return None;
    }
    let mut bytes = name.bytes();
    let simple = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
    Some(if simple {
        name.to_string()
    } else {
        format!("\"{name}\"")
    })
}

pub(super) fn sql_literal_at(source: &str, offset: usize) -> Option<SqlLiteral<'_>> {
    let tokens = tyrus_core::lexer::lex_spanned(source).ok()?;
    let (index, (token, span)) = tokens.iter().enumerate().find(|(_, (token, span))| {
        matches!(token, Token::StringLit(_)) && span.start < offset && offset < span.end
    })?;
    let Token::StringLit(decoded) = token else {
        return None;
    };
    let (Token::LParen, _) = tokens.get(index.checked_sub(1)?)? else {
        return None;
    };
    let query = tokens[..index - 1]
        .iter()
        .rposition(|(token, _)| matches!(token, Token::Ident(name) if name == "query"))?;
    if query < 2
        || !matches!(tokens[query - 1].0, Token::Dot)
        || !matches!(&tokens[query - 2].0, Token::Ident(name) if name == "db")
        || tokens[query + 1..index - 1]
            .iter()
            .any(|(token, _)| matches!(token, Token::LParen | Token::Semicolon))
    {
        return None;
    }
    let raw = source.get(span.start + 1..span.end.checked_sub(1)?)?;
    if !source.get(span.clone())?.starts_with('"') || raw != decoded {
        return None;
    }
    Some(SqlLiteral {
        text: raw,
        start: span.start + 1,
    })
}

fn bindings<'a>(
    tokens: &[SqlToken],
    snapshot: &'a DbSchemaSnapshot,
) -> Vec<(&'a tyrus_core::db::DbTableSchema, String)> {
    let mut result = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if !["FROM", "JOIN", "INTO", "UPDATE"]
            .iter()
            .any(|keyword| token.text.eq_ignore_ascii_case(keyword))
        {
            continue;
        }
        let Some(table_token) = tokens.get(index + 1) else {
            continue;
        };
        let Some(table) = snapshot
            .tables
            .iter()
            .find(|table| table.name.eq_ignore_ascii_case(&table_token.text))
        else {
            continue;
        };
        let next = tokens.get(index + 2);
        let alias = if next.is_some_and(|next| next.text.eq_ignore_ascii_case("AS")) {
            tokens.get(index + 3).map(|token| token.text.clone())
        } else {
            next.filter(|next| {
                next.kind == SqlTokenKind::Word
                    && ![
                        "ON",
                        "WHERE",
                        "HAVING",
                        "GROUP",
                        "ORDER",
                        "LIMIT",
                        "OFFSET",
                        "INNER",
                        "LEFT",
                        "RIGHT",
                        "FULL",
                        "JOIN",
                        "OUTER",
                        "SET",
                        "VALUES",
                        "RETURNING",
                    ]
                    .iter()
                    .any(|keyword| next.text.eq_ignore_ascii_case(keyword))
            })
            .map(|token| token.text.clone())
        };
        result.push((table, alias.unwrap_or_else(|| table.name.clone())));
    }
    result
}

pub(super) fn sql_completion_items(
    source: &str,
    offset: usize,
    snapshot: &DbSchemaSnapshot,
) -> Option<Vec<CompletionItem>> {
    let literal = sql_literal_at(source, offset)?;
    let local_offset = offset.checked_sub(literal.start)?;
    let prefix = literal.text.get(..local_offset)?;
    let prefix_tokens = tokenize_sql(prefix).ok()?;
    let all_tokens = tokenize_sql(literal.text).ok()?;
    let nullability = tyrus_core::db::parse_select_sql(literal.text)
        .ok()
        .map(|query| tyrus_core::db::select_table_nullability(&query));
    let table_keyword = |token: &SqlToken| {
        ["FROM", "JOIN", "INTO", "UPDATE"]
            .iter()
            .any(|keyword| token.text.eq_ignore_ascii_case(keyword))
    };
    let table_context = prefix_tokens.last().is_some_and(table_keyword)
        || (prefix_tokens.len() >= 2
            && prefix_tokens.last().is_some_and(|token| {
                token.kind == SqlTokenKind::Word && token.span.end == prefix.len()
            })
            && table_keyword(&prefix_tokens[prefix_tokens.len() - 2]));
    if table_context {
        return Some(
            snapshot
                .tables
                .iter()
                .filter_map(|table| {
                    Some(CompletionItem {
                        label: table.name.clone(),
                        kind: Some(CompletionItemKind::CLASS),
                        detail: Some("database table".to_string()),
                        insert_text: Some(sql_identifier(&table.name)?),
                        ..CompletionItem::default()
                    })
                })
                .collect(),
        );
    }
    let tables = bindings(&all_tokens, snapshot);
    if tables.is_empty() {
        if prefix_tokens.is_empty() {
            return Some(vec![sql_keyword("SELECT", "Start a query")]);
        }
        if prefix_tokens
            .last()
            .is_some_and(|token| token.text.eq_ignore_ascii_case("SELECT"))
        {
            let mut items = sql_projection_completions();
            items.push(sql_keyword("FROM", "Choose a source table"));
            return Some(items);
        }
        return None;
    }
    let mut items = Vec::new();
    for (index, (table, alias)) in tables.iter().enumerate() {
        for column in &table.columns {
            let Some(column_insert) = sql_identifier(&column.name) else {
                continue;
            };
            let label = if tables.len() == 1 {
                column.name.clone()
            } else {
                format!("{alias}.{}", column.name)
            };
            let insert_text = if tables.len() == 1 {
                column_insert
            } else {
                let Some(qualifier) = sql_identifier(alias) else {
                    continue;
                };
                format!("{qualifier}.{column_insert}")
            };
            items.push(CompletionItem {
                label,
                insert_text: Some(insert_text),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(format!(
                    "{}{} ({})",
                    column.logical_type,
                    if column.nullable
                        || nullability
                            .as_ref()
                            .is_some_and(|nullable| nullable.get(index) == Some(&true))
                    {
                        "?"
                    } else {
                        ""
                    },
                    table.name
                )),
                ..CompletionItem::default()
            });
        }
    }
    let words = prefix_tokens
        .iter()
        .filter(|token| token.kind == SqlTokenKind::Word)
        .map(|token| token.text.to_ascii_uppercase())
        .collect::<Vec<_>>();
    let last = words.last().map(String::as_str).unwrap_or_default();
    let has_where = words.iter().any(|word| word == "WHERE");
    let has_group = words.iter().any(|word| word == "GROUP");
    let has_order = words.iter().any(|word| word == "ORDER");
    if last == "GROUP" || last == "ORDER" {
        items.push(sql_keyword("BY", "Complete the clause"));
    } else if has_where || last == "ON" || last == "AND" || last == "OR" {
        for operator in ["=", "!=", "<>", "<", "<=", ">", ">=", "AND", "OR"] {
            items.push(sql_keyword(operator, "SQL predicate operator"));
        }
    } else if has_group || has_order {
        if has_order {
            items.extend(
                ["ASC", "DESC"]
                    .into_iter()
                    .map(|keyword| sql_keyword(keyword, "Sort direction")),
            );
        }
    } else if last == "SELECT"
        || prefix_tokens
            .last()
            .is_some_and(|token| token.kind == SqlTokenKind::Comma)
    {
        items.extend(sql_projection_completions());
    } else if !has_where {
        for (keyword, detail) in [
            ("WHERE", "Filter rows"),
            ("INNER JOIN", "Join matching rows"),
            ("LEFT JOIN", "Keep every row from the left table"),
            ("ORDER BY", "Sort result rows"),
            ("GROUP BY", "Group result rows"),
            ("HAVING", "Filter aggregate groups"),
            ("LIMIT", "Limit result rows"),
        ] {
            items.push(sql_keyword(keyword, detail));
        }
    }
    Some(items)
}

fn sql_keyword(label: &str, detail: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(detail.to_string()),
        insert_text: Some(label.to_string()),
        ..CompletionItem::default()
    }
}

fn sql_projection_completions() -> Vec<CompletionItem> {
    [
        ("COUNT(*) AS ${1:total}", "Count rows"),
        ("SUM(${1:column}) AS ${2:total}", "Sum a numeric column"),
        (
            "AVG(${1:column}) AS ${2:average}",
            "Average a numeric column",
        ),
        ("MIN(${1:column}) AS ${2:minimum}", "Minimum column value"),
        ("MAX(${1:column}) AS ${2:maximum}", "Maximum column value"),
    ]
    .into_iter()
    .map(|(snippet, detail)| CompletionItem {
        label: snippet
            .split_once('(')
            .map_or(snippet, |(name, _)| name)
            .trim_end_matches("(*) AS ${1:total}")
            .to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(detail.to_string()),
        insert_text: Some(snippet.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..CompletionItem::default()
    })
    .collect()
}

pub(super) fn sql_hover_at(
    source: &str,
    offset: usize,
    snapshot: &DbSchemaSnapshot,
) -> Option<(Range<usize>, String)> {
    let literal = sql_literal_at(source, offset)?;
    let local_offset = offset.checked_sub(literal.start)?;
    let tokens = tokenize_sql(literal.text).ok()?;
    let index = tokens.iter().position(|token| {
        token.kind == SqlTokenKind::Word
            && token.span.start <= local_offset
            && local_offset < token.span.end
    })?;
    let token = &tokens[index];
    if index > 0 && tokens[index - 1].text.eq_ignore_ascii_case("AS") {
        return None;
    }
    let range = literal.start + token.span.start..literal.start + token.span.end;
    if tokens
        .get(index + 1)
        .is_some_and(|next| next.kind == SqlTokenKind::LeftParen)
    {
        let aggregate = match token.text.to_ascii_uppercase().as_str() {
            "COUNT" => Some((
                "COUNT(*) AS alias",
                "Counts matching rows, or nonempty values when given a column. The result is `i64` and is `0` when no rows match.",
            )),
            "SUM" => Some((
                "SUM(column) AS alias",
                "Adds an `i64` or `f64` column. The result is optional because an empty input has no sum.",
            )),
            "AVG" => Some((
                "AVG(column) AS alias",
                "Averages an `i64` or `f64` column. The result is optional because an empty input has no average.",
            )),
            "MIN" => Some((
                "MIN(column) AS alias",
                "Returns the minimum `i64`, `f64`, or `string` value, or an empty optional for no matching rows.",
            )),
            "MAX" => Some((
                "MAX(column) AS alias",
                "Returns the maximum `i64`, `f64`, or `string` value, or an empty optional for no matching rows.",
            )),
            _ => None,
        };
        if let Some((signature, detail)) = aggregate {
            return Some((
                range,
                format!("**SQL aggregate** `{signature}`\n\n{detail}"),
            ));
        }
    }
    if index > 0
        && ["FROM", "JOIN", "INTO", "UPDATE"]
            .iter()
            .any(|keyword| tokens[index - 1].text.eq_ignore_ascii_case(keyword))
    {
        let table = snapshot
            .tables
            .iter()
            .find(|table| table.name.eq_ignore_ascii_case(&token.text))?;
        return Some((
            range,
            format!(
                "**SQL table** `{}` ({} columns)",
                table.name,
                table.columns.len()
            ),
        ));
    }
    let tables = bindings(&tokens, snapshot);
    let qualifier = if index >= 2 && tokens[index - 1].kind == SqlTokenKind::Dot {
        Some(tokens[index - 2].text.as_str())
    } else {
        None
    };
    let matches = tables
        .iter()
        .enumerate()
        .filter(|(_, (table, alias))| {
            qualifier.is_none_or(|qualifier| {
                alias.eq_ignore_ascii_case(qualifier) || table.name.eq_ignore_ascii_case(qualifier)
            })
        })
        .filter_map(|(index, (table, _))| {
            table
                .columns
                .iter()
                .find(|column| column.name.eq_ignore_ascii_case(&token.text))
                .map(|column| (index, table, column))
        })
        .collect::<Vec<_>>();
    let [(table_index, table, column)] = matches.as_slice() else {
        return None;
    };
    let nullable = column.nullable
        || tyrus_core::db::parse_select_sql(literal.text)
            .ok()
            .map(|query| tyrus_core::db::select_table_nullability(&query))
            .is_some_and(|nullability| nullability.get(*table_index) == Some(&true));
    Some((
        range,
        format!(
            "**SQL column** `{}.{}`: `{}{}`",
            table.name,
            column.name,
            column.logical_type,
            if nullable { "?" } else { "" }
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tyrus_core::db::{DbColumnSchema, DbTableSchema};

    fn snapshot() -> DbSchemaSnapshot {
        DbSchemaSnapshot::new(
            "test.sqlite".to_string(),
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
        )
    }

    #[test]
    fn sql_literal_detection_requires_a_db_query_call() {
        let source = r#"const value = "SELECT id FROM users";"#;
        let offset = source.find("users").unwrap();
        assert!(sql_literal_at(source, offset).is_none());

        let source = r#"const rows = db.query<User>("SELECT id FROM users", []);"#;
        let offset = source.find("users").unwrap();
        let literal = sql_literal_at(source, offset).unwrap();
        assert_eq!(literal.text, "SELECT id FROM users");
    }

    #[test]
    fn completion_offers_snapshot_tables_after_from() {
        let source = r#"const rows = db.query("SELECT * FROM ", []);"#;
        let offset = source.find("FROM ").unwrap() + "FROM ".len();
        let items = sql_completion_items(source, offset, &snapshot()).unwrap();
        assert!(items.iter().any(|item| item.label == "users"));
    }

    #[test]
    fn hover_reports_the_exact_column_range_and_schema_type() {
        let source = r#"const rows = db.query<User>("SELECT id FROM users", []);"#;
        let start = source.find("id FROM").unwrap();
        let (range, markdown) = sql_hover_at(source, start, &snapshot()).unwrap();
        assert_eq!(&source[range.clone()], "id");
        assert!(markdown.contains("users.id`"));
        assert!(markdown.contains("`i64`"));
    }
}
