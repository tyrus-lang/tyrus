use crate::ast::{DecimalLiteral, FloatLiteral, Type};
use logos::Logos;
use std::ops::Range;

pub(crate) fn decode_string_content(source: &str) -> String {
    let mut decoded = String::new();
    let chars = source.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        let character = chars[index];
        index += 1;
        if character != '\\' {
            decoded.push(character);
            continue;
        }
        if index >= chars.len() {
            decoded.push('\\');
            break;
        }
        let escaped = chars[index];
        index += 1;
        match escaped {
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            't' => decoded.push('\t'),
            '0' => decoded.push('\0'),
            'u' if matches!(chars.get(index), Some('{')) => {
                index += 1;
                let start = index;
                while index < chars.len() && chars[index] != '}' {
                    index += 1;
                }
                let closed = index < chars.len() && chars[index] == '}';
                let hex = chars[start..index].iter().collect::<String>();
                if closed {
                    index += 1;
                }
                if let Ok(codepoint) = u32::from_str_radix(&hex, 16)
                    && let Some(character) = char::from_u32(codepoint)
                {
                    decoded.push(character);
                    continue;
                }
                decoded.push_str("\\u{");
                decoded.push_str(&hex);
                if closed {
                    decoded.push('}');
                }
            }
            'u' if index + 4 <= chars.len() => {
                let hex = chars[index..index + 4].iter().collect::<String>();
                if hex.chars().all(|character| character.is_ascii_hexdigit())
                    && let Ok(codepoint) = u32::from_str_radix(&hex, 16)
                {
                    if (0xD800..=0xDBFF).contains(&codepoint)
                        && index + 10 <= chars.len()
                        && chars[index + 4] == '\\'
                        && chars[index + 5] == 'u'
                    {
                        let low_hex = chars[index + 6..index + 10].iter().collect::<String>();
                        if low_hex
                            .chars()
                            .all(|character| character.is_ascii_hexdigit())
                            && let Ok(low) = u32::from_str_radix(&low_hex, 16)
                            && (0xDC00..=0xDFFF).contains(&low)
                        {
                            let scalar = 0x10000 + ((codepoint - 0xD800) << 10) + (low - 0xDC00);
                            if let Some(character) = char::from_u32(scalar) {
                                decoded.push(character);
                                index += 10;
                                continue;
                            }
                        }
                    }
                    if let Some(character) = char::from_u32(codepoint) {
                        decoded.push(character);
                        index += 4;
                        continue;
                    }
                }
                decoded.push_str("\\u");
            }
            '\\' => decoded.push('\\'),
            '\'' => decoded.push('\''),
            '"' => decoded.push('"'),
            '`' => decoded.push('`'),
            '$' => decoded.push('$'),
            other => {
                decoded.push('\\');
                decoded.push(other);
            }
        }
    }
    decoded
}

fn decode_quoted_string(source: &str) -> String {
    decode_string_content(&source[1..source.len() - 1])
}

#[derive(Debug, PartialEq, Clone)]
pub struct RegexLiteral {
    pub pattern: String,
    pub flags: String,
}

fn decode_regex_literal(source: &str) -> RegexLiteral {
    let mut escaped = false;
    let mut closing = source.len() - 1;
    for (index, character) in source.char_indices().skip(1) {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' {
            escaped = true;
            continue;
        }
        if character == '/' {
            closing = index;
            break;
        }
    }
    RegexLiteral {
        pattern: source[1..closing].to_string(),
        flags: source[closing + 1..].to_string(),
    }
}

fn decode_float(s: &str) -> Result<FloatLiteral, ()> {
    let value = s.parse::<f64>().map_err(|_| ())?;
    if !value.is_finite() {
        return Err(());
    }
    if !s.contains(['e', 'E']) {
        let integer_digits = s.split('.').next().unwrap_or("").len();
        if integer_digits > 15 {
            return Err(());
        }
    }
    Ok(FloatLiteral::new(value, s))
}

fn decode_decimal(s: &str) -> Result<DecimalLiteral, ()> {
    let text = s.strip_suffix(['d', 'D']).ok_or(())?;
    if text.is_empty() {
        return Err(());
    }
    Ok(DecimalLiteral::new(text.replace('_', "")))
}

fn numeric_suffix(s: &str) -> Option<(&str, Type)> {
    const SUFFIXES: &[(&str, Type)] = &[
        ("u8", Type::U8),
        ("u16", Type::U16),
        ("u32", Type::U32),
        ("u64", Type::U64),
        ("u128", Type::U128),
        ("usize", Type::Usize),
        ("i8", Type::I8),
        ("i16", Type::I16),
        ("i32", Type::I32),
        ("i64", Type::I64),
        ("i128", Type::I128),
        ("isize", Type::Isize),
        ("f32", Type::F32),
        ("f64", Type::F64),
        ("f128", Type::F128),
    ];
    SUFFIXES
        .iter()
        .find_map(|(suffix, ty)| s.strip_suffix(suffix).map(|value| (value, ty.clone())))
}

fn decode_typed_int(s: &str) -> Result<(i128, Type), ()> {
    let (value, ty) = numeric_suffix(s).ok_or(())?;
    if matches!(ty, Type::F32 | Type::F64 | Type::F128) {
        return Err(());
    }
    let value = if value.starts_with("0x") || value.starts_with("0X") {
        decode_hex(value)
    } else if value.starts_with("0b") || value.starts_with("0B") {
        decode_binary(value)
    } else if value.starts_with("0o") || value.starts_with("0O") {
        decode_octal(value)
    } else {
        decode_grouped_decimal(value)?
    };
    Ok((value, ty))
}

fn decode_typed_uint(s: &str) -> Result<(u128, Type), ()> {
    let (value, ty) = numeric_suffix(s).ok_or(())?;
    if !matches!(
        ty,
        Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::Usize
    ) {
        return Err(());
    }
    let value = value.replace('_', "");
    let value = if value.starts_with("0x") || value.starts_with("0X") {
        u128::from_str_radix(&value[2..], 16).map_err(|_| ())?
    } else if value.starts_with("0b") || value.starts_with("0B") {
        u128::from_str_radix(&value[2..], 2).map_err(|_| ())?
    } else if value.starts_with("0o") || value.starts_with("0O") {
        u128::from_str_radix(&value[2..], 8).map_err(|_| ())?
    } else {
        value.parse::<u128>().map_err(|_| ())?
    };
    Ok((value, ty))
}

fn decode_typed_float(s: &str) -> Result<(FloatLiteral, Type), ()> {
    let (value, ty) = numeric_suffix(s).ok_or(())?;
    if !matches!(ty, Type::F32 | Type::F64 | Type::F128) {
        return Err(());
    }
    Ok((decode_float(value)?, ty))
}

fn valid_digit_separators(digits: &str, group_size: usize) -> bool {
    if digits.starts_with('_') || digits.ends_with('_') || digits.contains("__") {
        return false;
    }
    let Some((first, rest)) = digits.split_once('_') else {
        return true;
    };
    !first.is_empty()
        && first.len() <= group_size
        && rest
            .split('_')
            .all(|group| !group.is_empty() && group.len() == group_size)
}

fn decode_hex(s: &str) -> i128 {
    let s = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap();
    i128::from_str_radix(s.replace('_', "").as_str(), 16).unwrap_or(0)
}

fn decode_binary(s: &str) -> i128 {
    let s = s
        .strip_prefix("0b")
        .or_else(|| s.strip_prefix("0B"))
        .unwrap();
    i128::from_str_radix(s.replace('_', "").as_str(), 2).unwrap_or(0)
}

fn decode_octal(s: &str) -> i128 {
    let s = s
        .strip_prefix("0o")
        .or_else(|| s.strip_prefix("0O"))
        .unwrap();
    i128::from_str_radix(s.replace('_', "").as_str(), 8).unwrap_or(0)
}

fn decode_grouped_decimal(s: &str) -> Result<i128, ()> {
    if !valid_digit_separators(s, 3) {
        return Err(());
    }
    s.replace('_', "").parse::<i128>().map_err(|_| ())
}

fn normalized_decimal_overflow(s: &str) -> Option<String> {
    if !s
        .chars()
        .all(|character| character.is_ascii_digit() || character == '_')
    {
        return None;
    }
    if !valid_digit_separators(s, 3) {
        return None;
    }
    let normalized = s.replace('_', "");
    let i128_max = i128::MAX.to_string();
    (normalized.len() > i128_max.len()
        || (normalized.len() == i128_max.len() && normalized > i128_max))
        .then_some(normalized)
}

fn trim_source_range(source: &str, range: Range<usize>) -> Option<Range<usize>> {
    let text = source.get(range.clone())?;
    let start_offset = text.len() - text.trim_start().len();
    let end_offset = text.trim_end().len();
    (start_offset < end_offset).then_some(range.start + start_offset..range.start + end_offset)
}

fn skipped_numeric_token(
    source: &str,
    range: Range<usize>,
) -> Option<Result<(Token, Range<usize>), LexError>> {
    let range = trim_source_range(source, range)?;
    let text = source.get(range.clone())?;
    if text.starts_with("//") || text.starts_with("/*") {
        return None;
    }
    if text.contains(['.', 'e', 'E']) && is_float_literal_shape(text) {
        return Some(Ok((Token::FloatOverflowLit, range)));
    }
    if let Some(normalized) = normalized_decimal_overflow(text) {
        return Some(Ok((Token::IntOverflowLit(normalized), range)));
    }
    if text.chars().any(|character| character.is_ascii_digit())
        && text.chars().all(|character| {
            character.is_ascii_digit() || matches!(character, '_' | '.' | 'e' | 'E' | '+' | '-')
        })
    {
        return Some(Err(LexError { range }));
    }
    None
}

fn is_float_literal_shape(s: &str) -> bool {
    let Some((integer, rest)) = s.split_once('.') else {
        let Some((integer, exponent)) = s.split_once(['e', 'E']) else {
            return false;
        };
        return !integer.is_empty() && !exponent.is_empty();
    };
    !integer.is_empty()
        && rest
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
}

#[derive(Logos, Debug, PartialEq, Clone)]
pub enum Token {
    #[token("const")]
    Const,

    #[token("comptime")]
    Comptime,

    #[token("let")]
    Let,

    #[token("if")]
    If,

    #[token("else")]
    Else,

    #[token("return")]
    Return,

    #[token("while")]
    While,

    #[token("loop")]
    Loop,

    #[token("for")]
    For,

    #[token("of")]
    Of,

    #[token("onfail")]
    OnFail,

    #[token("in")]
    In,

    #[token("is")]
    Is,

    #[token("break")]
    Break,

    #[token("continue")]
    Continue,

    #[token("defer")]
    Defer,

    #[token("errdefer")]
    ErrDefer,

    #[token("do")]
    Do,

    #[token("switch")]
    Switch,

    #[token("throw")]
    Throw,

    #[token("throws")]
    Throws,

    #[token("try")]
    Try,

    #[token("catch")]
    Catch,

    #[token("finally")]
    Finally,

    #[token("function")]
    Function,

    #[token("validator")]
    Validator,

    #[token("async")]
    Async,

    #[token("await")]
    Await,

    #[token("typeof")]
    Typeof,

    #[token("case")]
    Case,

    #[token("default")]
    Default,

    #[token("match")]
    Match,

    #[token("import")]
    Import,

    #[token("export")]
    Export,

    #[token("protect")]
    Protect,

    #[token("interface")]
    Interface,

    #[token("at")]
    At,

    #[token("enum")]
    Enum,

    #[token("type")]
    Type,

    #[token("from")]
    From,

    #[token("as")]
    As,

    #[token("extends")]
    Extends,

    #[token("null")]
    Null,

    #[token("None")]
    None,

    #[token("i8")]
    TyI8,

    #[token("i16")]
    TyI16,

    #[token("i32")]
    TyI32,

    #[token("i64")]
    TyI64,

    #[token("i128")]
    TyI128,

    #[token("isize")]
    TyIsize,

    #[token("u8")]
    TyU8,

    #[token("u16")]
    TyU16,

    #[token("u32")]
    TyU32,

    #[token("u64")]
    TyU64,

    #[token("u128")]
    TyU128,

    #[token("usize")]
    TyUsize,

    #[token("f32")]
    TyF32,

    #[token("f64")]
    TyF64,

    #[token("f128")]
    TyF128,

    #[token("decimal")]
    TyDecimal,

    #[token("char")]
    TyChar,

    #[token("boolean")]
    TyBool,

    #[token("string")]
    TyString,

    #[token("void")]
    TyVoid,

    #[token("unknown")]
    TyUnknown,

    #[token("never")]
    TyNever,

    #[token("readonly")]
    Readonly,

    #[regex(r"(?:[0-9](?:_?[0-9])*\.[0-9](?:_?[0-9])*(?:[eE][+-]?[0-9](?:_?[0-9])*)?|[0-9](?:_?[0-9])*[eE][+-]?[0-9](?:_?[0-9])*|[0-9](?:_?[0-9])*)[dD]", |lex| decode_decimal(lex.slice()))]
    DecimalLit(DecimalLiteral),

    #[regex(r"(?:0[xX][0-9a-fA-F_]+|0[bB][01_]+|0[oO][0-7_]+|[0-9][0-9_]*)u(?:8|16|32|64|128|size)", |lex| decode_typed_uint(lex.slice()))]
    TypedUIntLit((u128, Type)),

    #[regex(r"(?:0[xX][0-9a-fA-F_]+|0[bB][01_]+|0[oO][0-7_]+|[0-9][0-9_]*)i(?:8|16|32|64|128|size)", |lex| decode_typed_int(lex.slice()))]
    TypedIntLit((i128, Type)),

    #[regex(r"[0-9]+\.[0-9]+(?:[eE][+-]?[0-9]+)?f(?:32|64|128)|[0-9]+[eE][+-]?[0-9]+f(?:32|64|128)", |lex| decode_typed_float(lex.slice()))]
    TypedFloatLit((FloatLiteral, Type)),

    #[regex(r"[0-9]+\.[0-9]+(?:[eE][+-]?[0-9]+)?", |lex| decode_float(lex.slice()))]
    #[regex(r"[0-9]+[eE][+-]?[0-9]+", |lex| decode_float(lex.slice()))]
    FloatLit(FloatLiteral),

    #[token("Infinity", |_| FloatLiteral::new(f64::INFINITY, "Infinity"))]
    #[token("NaN", |_| FloatLiteral::new(f64::NAN, "NaN"))]
    SpecialFloatLit(FloatLiteral),

    FloatOverflowLit,

    #[regex(r"0[xX][0-9a-fA-F_]+", |lex| decode_hex(lex.slice()))]
    #[regex(r"0[bB][0-1_]+", |lex| decode_binary(lex.slice()))]
    #[regex(r"0[oO][0-7_]+", |lex| decode_octal(lex.slice()))]
    #[regex(r"[0-9][0-9_]*", |lex| decode_grouped_decimal(lex.slice()))]
    IntLit(i128),

    IntOverflowLit(String),

    #[regex(r#""([^"\\]|\\.)*""#, |lex| decode_quoted_string(lex.slice()))]
    #[regex(r"'([^'\\]|\\.)*'", |lex| decode_quoted_string(lex.slice()))]
    StringLit(String),

    #[regex(r"`([^`\\]|\\.)*`", |lex| {
        let s = lex.slice();
        s[1..s.len() - 1].to_string()
    })]
    TemplateLit(String),

    RegexLit(RegexLiteral),

    #[token("true")]
    True,

    #[token("false")]
    False,

    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),

    #[token("===")]
    EqEqEq,

    #[token("!==")]
    NotEqEq,

    #[token("=>")]
    Arrow,

    #[token("=")]
    Eq,

    #[token("+=")]
    PlusEq,

    #[token("-=")]
    MinusEq,

    #[token("*=")]
    StarEq,

    #[token("/=")]
    SlashEq,

    #[token("%=")]
    PercentEq,

    #[token("**=")]
    PowerEq,

    #[token("&&=")]
    AndEq,

    #[token("||=")]
    OrEq,

    #[token("??=")]
    NullCoalesceEq,

    #[token("&=")]
    AmpersandEq,

    #[token("|=")]
    PipeEq,

    #[token("|>")]
    PipeOperator,

    #[token("^=")]
    CaretEq,

    #[token("<<=")]
    ShlEq,

    #[token(">>=")]
    ShrEq,

    #[token(">>>=")]
    UShrEq,

    #[token("**")]
    Power,

    #[token("++")]
    PlusPlus,

    #[token("--")]
    MinusMinus,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("%")]
    Percent,

    #[token(">=")]
    Gte,

    #[token("<=")]
    Lte,

    #[token(">")]
    Gt,

    #[token("<")]
    Lt,

    #[token("&&")]
    #[token("and")]
    And,

    #[token("||")]
    #[token("or")]
    Or,

    #[token("!")]
    #[token("not")]
    Bang,

    #[token("&")]
    Ampersand,

    #[token("|")]
    Pipe,

    #[token("^")]
    Caret,

    #[token("~")]
    Tilde,

    #[token("<<")]
    Shl,

    #[token(">>")]
    Shr,

    #[token(">>>")]
    UShr,

    #[token("...")]
    Ellipsis,

    #[token("..=")]
    DotDotEq,

    #[token("..")]
    DotDot,

    #[token(".")]
    Dot,

    #[token("?.")]
    OptChain,

    #[token("??")]
    NullCoalesce,

    #[token("?")]
    Question,

    #[token("@")]
    AtSign,

    #[token(":")]
    Colon,

    #[token(",")]
    Comma,

    #[token(";")]
    Semicolon,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[regex(r"//[^\n]*", logos::skip, allow_greedy = true)]
    LineComment,

    #[token("/*", skip_block_comment)]
    BlockComment,

    #[regex(r"[ \t\n\r]+", logos::skip)]
    Whitespace,
}

fn skip_block_comment(lexer: &mut logos::Lexer<'_, Token>) -> Result<logos::Skip, ()> {
    let remainder = lexer.remainder();
    if let Some(end) = remainder.find("*/") {
        lexer.bump(end + 2);
        Ok(logos::Skip)
    } else {
        lexer.bump(remainder.len());
        Err(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub range: Range<usize>,
}

pub fn lex(source: &str) -> Result<Vec<Token>, LexError> {
    Ok(lex_spanned(source)?
        .into_iter()
        .map(|(token, _)| token)
        .collect())
}

fn scan_multiline_string_literal(source: &str, start: usize) -> Option<(Token, usize)> {
    if !source[start..].starts_with("\"\"\"") {
        return None;
    }
    let mut end = start + 3;
    while end < source.len() {
        if source[end..].starts_with("\"\"\"") {
            let raw = &source[start + 3..end];
            let dedented = dedent_multiline_raw(raw);
            return Some((Token::StringLit(dedented), end + 3));
        }
        end += 1;
    }
    None
}

fn dedent_multiline_raw(raw: &str) -> String {
    let (body, closing_line) = match raw.rfind('\n') {
        Some(idx) => (&raw[..idx], &raw[idx + 1..]),
        None => (raw, ""),
    };
    let closing_indent = closing_line.chars().take_while(|c| *c == ' ').count();

    if body.is_empty() {
        return String::new();
    }

    let mut lines = body.split('\n').peekable();
    if lines.peek().is_some_and(|line| line.trim().is_empty()) {
        lines.next();
    }

    let mut result = String::new();
    for line in lines {
        let spaces = line.chars().take_while(|c| *c == ' ').count();
        let to_remove = spaces.min(closing_indent);
        result.push_str(&line[to_remove..]);
        result.push('\n');
    }
    result.pop();
    result
}

pub fn lex_spanned(source: &str) -> Result<Vec<(Token, Range<usize>)>, LexError> {
    match lex_spanned_controlled(source, None) {
        Ok(tokens) => Ok(tokens),
        Err(LexControlError::Lex(error)) => Err(error),
        Err(LexControlError::Cancelled) => unreachable!("uncancellable lexing cannot cancel"),
    }
}

pub(crate) fn lex_spanned_cancellable(
    source: &str,
    cancellation: &std::sync::atomic::AtomicBool,
) -> Result<Vec<(Token, Range<usize>)>, LexControlError> {
    lex_spanned_controlled(source, Some(cancellation))
}

pub(crate) enum LexControlError {
    Lex(LexError),
    Cancelled,
}

fn lex_spanned_controlled(
    source: &str,
    cancellation: Option<&std::sync::atomic::AtomicBool>,
) -> Result<Vec<(Token, Range<usize>)>, LexControlError> {
    let cancelled =
        || cancellation.is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Acquire));
    let mut lexer = Token::lexer(source).spanned().peekable();
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while let Some((result, range)) = lexer.next() {
        if cancelled() {
            return Err(LexControlError::Cancelled);
        }
        if range.start > cursor
            && let Some(skipped) = skipped_numeric_token(source, cursor..range.start)
        {
            tokens.push(skipped.map_err(LexControlError::Lex)?);
        }
        match result {
            Ok(token) => {
                match token {
                    Token::StringLit(ref s)
                        if s.is_empty()
                            && source
                                .get(range.start..)
                                .is_some_and(|rest| rest.starts_with("\"\"\"")) =>
                    {
                        let Some((token, end)) = scan_multiline_string_literal(source, range.start)
                        else {
                            return Err(LexError {
                                range: range.start..source.len(),
                            }
                            .into());
                        };
                        tokens.push((token, range.start..end));
                        cursor = end;
                        while lexer
                            .peek()
                            .is_some_and(|(_, next_range)| next_range.start < end)
                        {
                            lexer.next();
                        }
                        continue;
                    }
                    _ => {}
                }
                if matches!(token, Token::Slash)
                    && can_start_regex_literal(tokens.last().map(|(token, _)| token))
                    && let Some((literal, end)) = scan_regex_literal(source, range.start)
                {
                    tokens.push((Token::RegexLit(literal), range.start..end));
                    while lexer
                        .peek()
                        .is_some_and(|(_, next_range)| next_range.start < end)
                    {
                        lexer.next();
                    }
                    cursor = end;
                    continue;
                }
                if matches!(
                    token,
                    Token::IntLit(_) | Token::FloatLit(_) | Token::SpecialFloatLit(_)
                ) && source
                    .get(range.end..)
                    .and_then(|rest| rest.chars().next())
                    .is_some_and(|character| character.is_ascii_alphabetic())
                {
                    return Err(LexError {
                        range: range.start..range.end + 1,
                    }
                    .into());
                }
                cursor = range.end;
                tokens.push((token, range));
            }
            Err(()) => {
                if source
                    .get(range.clone())
                    .is_some_and(|text| text.starts_with("/*"))
                {
                    return Err(LexError { range }.into());
                }
                if source
                    .get(range.clone())
                    .is_some_and(is_float_literal_shape)
                {
                    tokens.push((Token::FloatOverflowLit, range));
                    cursor = tokens.last().map(|(_, range)| range.end).unwrap_or(cursor);
                    continue;
                }
                if let Some(normalized) = source
                    .get(range.clone())
                    .and_then(normalized_decimal_overflow)
                {
                    tokens.push((Token::IntOverflowLit(normalized), range));
                    cursor = tokens.last().map(|(_, range)| range.end).unwrap_or(cursor);
                    continue;
                }
                let Some((Token::Slash, slash_range)) = tokens.last().cloned() else {
                    return Err(LexError { range }.into());
                };
                let before_slash = tokens
                    .get(tokens.len().saturating_sub(2))
                    .map(|(token, _)| token);
                if !can_start_regex_literal(before_slash) {
                    return Err(LexError { range }.into());
                }
                let Some((literal, end)) = scan_regex_literal(source, slash_range.start) else {
                    return Err(LexError { range }.into());
                };
                tokens.pop();
                tokens.push((Token::RegexLit(literal), slash_range.start..end));
                while lexer
                    .peek()
                    .is_some_and(|(_, next_range)| next_range.start < end)
                {
                    lexer.next();
                }
                cursor = end;
            }
        }
    }
    if cursor < source.len()
        && let Some(skipped) = skipped_numeric_token(source, cursor..source.len())
    {
        tokens.push(skipped.map_err(LexControlError::Lex)?);
    }
    if cancelled() {
        return Err(LexControlError::Cancelled);
    }
    Ok(tokens)
}

impl From<LexError> for LexControlError {
    fn from(error: LexError) -> Self {
        Self::Lex(error)
    }
}

fn can_start_regex_literal(previous: Option<&Token>) -> bool {
    matches!(
        previous,
        None | Some(
            Token::LParen
                | Token::LBracket
                | Token::LBrace
                | Token::Comma
                | Token::Colon
                | Token::Eq
                | Token::Arrow
                | Token::Return
                | Token::Await
                | Token::AtSign
                | Token::Question
                | Token::NullCoalesce
                | Token::Plus
                | Token::Minus
                | Token::Star
                | Token::Slash
                | Token::Percent
                | Token::Bang
                | Token::And
                | Token::Or
                | Token::Pipe
                | Token::Caret
                | Token::Ampersand
                | Token::Tilde
        )
    )
}

fn scan_regex_literal(source: &str, start: usize) -> Option<(RegexLiteral, usize)> {
    let mut escaped = false;
    let mut in_class = false;
    let mut closing = None;
    for (offset, character) in source[start + 1..].char_indices() {
        let index = start + 1 + offset;
        if escaped {
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '[' => in_class = true,
            ']' => in_class = false,
            '/' if !in_class => {
                closing = Some(index);
                break;
            }
            '\n' | '\r' => return None,
            _ => {}
        }
    }
    let closing = closing?;
    if closing == start + 1 {
        return None;
    }
    let mut end = closing + 1;
    for (offset, character) in source[end..].char_indices() {
        if character.is_ascii_alphabetic() {
            end = closing + 1 + offset + character.len_utf8();
        } else {
            break;
        }
    }
    Some((decode_regex_literal(&source[start..end]), end))
}

#[cfg(test)]
#[path = "../../tests/unit/lexer.rs"]
mod tests;
