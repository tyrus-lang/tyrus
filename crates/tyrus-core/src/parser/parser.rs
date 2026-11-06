use crate::ast::*;
use crate::collections::{HashMap, HashSet};
use crate::lexer::Token;
use crate::span::Span;
use std::ops::Range;

#[path = "parser/db_model_decorators.rs"]
mod db_model_decorators;
use db_model_decorators::{parse_db_model_decorator, parse_db_model_field_decorators};

/// A saved parser position used to backtrack speculative parses. It captures
/// `pending_error` alongside `pos` so a reserved-keyword diagnostic stashed
/// inside an abandoned branch does not leak into a later, unrelated error.
#[derive(Clone)]
struct Mark {
    pos: usize,
    pending_error: Option<ParseError>,
}

const READONLY_LET_MESSAGE: &str = "`readonly` qualifies `const`, not `let`: write `readonly const <name>` (a `let` can be rebound, so it cannot be readonly)";

const READONLY_ORDER_MESSAGE: &str =
    "`readonly` goes before `const`: write `readonly const <name>`, not `const readonly <name>`";

const READONLY_DESTRUCTURE_MESSAGE: &str = "`readonly const` cannot destructure: name the binding (`readonly const <name> = ...`) and read its parts through it";

const READONLY_FUNCTION_MESSAGE: &str =
    "`readonly const` declares a value, not a function: drop `readonly` from this declaration";

const READONLY_ANNOTATION_MESSAGE: &str =
    "`readonly const` requires a type annotation: write `readonly const <name>: <type> = ...`";

fn readonly_binding_type(ty: Type) -> Result<Type, String> {
    match ty {
        Type::Infer => Err(READONLY_ANNOTATION_MESSAGE.to_string()),
        Type::Readonly(_) => Ok(ty),
        _ => Ok(Type::Readonly(Box::new(ty))),
    }
}

type ParsedFunctionParts = (Vec<TypeParam>, Vec<Param>, Type, Vec<Type>, Vec<Stmt>);

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub code: Option<&'static str>,
    pub span: Option<Range<usize>>,
    pub help: Vec<String>,
    pub at_end_of_input: bool,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        ParseError {
            message: message.into(),
            code: None,
            span: None,
            help: Vec::new(),
            at_end_of_input: false,
        }
    }
}

impl From<String> for ParseError {
    fn from(message: String) -> Self {
        ParseError::new(message)
    }
}

impl From<&str> for ParseError {
    fn from(message: &str) -> Self {
        ParseError::new(message)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

fn found<T: std::fmt::Debug>(token: &Option<T>) -> String {
    match token {
        Some(token) => format!("{token:?}"),
        None => "end of file".to_string(),
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    token_ranges: Vec<Range<usize>>,
    pos: usize,
    type_parameter_scopes: Vec<HashSet<String>>,
    pending_error: Option<ParseError>,
    depth: usize,
    /// Greater than zero while parsing a statement that a closing delimiter ends
    /// instead of a `;`, which today is only the update clause of a classic
    /// `for`, written `for (let i = 0; i < n; i++)`.
    omitted_terminators: usize,
    /// Whether the postfix `?` at a token index propagates a failure, once it
    /// has been decided. See `question_is_try`.
    question_decisions: HashMap<usize, bool>,
    /// Set while the operand of an `await` is read, and consumed by the first
    /// postfix chain that starts, so parentheses and arguments nested inside
    /// never see it. That chain stops before its first propagating `?`, which
    /// then applies to the awaited value: `await load()?` is
    /// `(await load())?`, while `await (load()?)` keeps its parentheses.
    await_operand: bool,
    cancellation: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

const MAX_RECURSION_DEPTH: usize = 32;

const MAX_INTERSECTION_OPERANDS: usize = 64;

pub const MAX_TOKENS: usize = 2_000_000;

#[derive(Debug, Clone, Copy)]
pub(crate) enum BindingKind {
    Variable,
    Parameter,
    Function,
    Type,
    Enum,
    Import,
    Catch,
}

impl BindingKind {
    fn label(self) -> &'static str {
        match self {
            BindingKind::Variable => "variable",
            BindingKind::Parameter => "parameter",
            BindingKind::Function => "function",
            BindingKind::Type => "type",
            BindingKind::Enum => "enum",
            BindingKind::Import => "import",
            BindingKind::Catch => "catch binding",
        }
    }
}

fn is_i128_min_magnitude(value: &str) -> bool {
    value
        .parse::<u128>()
        .is_ok_and(|magnitude| magnitude == i128::MAX as u128 + 1)
}

fn slice_end(end: Expr, inclusive: bool) -> Expr {
    if !inclusive {
        return end;
    }
    match end.kind() {
        ExprKind::IntLit(value) => {
            return Expr::synthetic(ExprKind::IntLit(value.saturating_add(1)));
        }
        ExprKind::UIntLit(value) => {
            return Expr::synthetic(ExprKind::UIntLit(value.saturating_add(1)));
        }
        _ => {}
    }
    Expr::synthetic(ExprKind::BinOp {
        op: BinOp::Add,
        left: Box::new(end),
        right: Box::new(Expr::synthetic(ExprKind::IntLit(1))),
    })
}

pub(crate) fn keyword_as_name(token: &Token) -> Option<&'static str> {
    Some(match token {
        Token::Const => "const",
        Token::Comptime => "comptime",
        Token::Let => "let",
        Token::If => "if",
        Token::Else => "else",
        Token::Return => "return",
        Token::While => "while",
        Token::Loop => "loop",
        Token::For => "for",
        Token::Of => "of",
        Token::OnFail => "onfail",
        Token::In => "in",
        Token::Break => "break",
        Token::Continue => "continue",
        Token::Defer => "defer",
        Token::ErrDefer => "errdefer",
        Token::Do => "do",
        Token::Switch => "switch",
        Token::Throw => "throw",
        Token::Throws => "throws",
        Token::Try => "try",
        Token::Catch => "catch",
        Token::Finally => "finally",
        Token::Function => "function",
        Token::Validator => "validator",
        Token::Async => "async",
        Token::Typeof => "typeof",
        Token::Case => "case",
        Token::Default => "default",
        Token::Match => "match",
        Token::Import => "import",
        Token::Export => "export",
        Token::Protect => "protect",
        Token::Interface => "interface",
        Token::At => "at",
        Token::Enum => "enum",
        Token::Type => "type",
        Token::From => "from",
        Token::As => "as",
        Token::Extends => "extends",
        Token::Null => "null",
        Token::None => "None",
        Token::TyI8 => "i8",
        Token::TyI16 => "i16",
        Token::TyI32 => "i32",
        Token::TyI64 => "i64",
        Token::TyI128 => "i128",
        Token::TyIsize => "isize",
        Token::TyU8 => "u8",
        Token::TyU16 => "u16",
        Token::TyU32 => "u32",
        Token::TyU64 => "u64",
        Token::TyU128 => "u128",
        Token::TyUsize => "usize",
        Token::TyF32 => "f32",
        Token::TyF64 => "f64",
        Token::TyF128 => "f128",
        Token::TyDecimal => "decimal",
        Token::TyChar => "char",
        Token::TyBool => "boolean",
        Token::TyString => "string",
        Token::TyVoid => "void",
        Token::TyUnknown => "unknown",
        Token::TyNever => "never",
        Token::Readonly => "readonly",
        Token::True => "true",
        Token::False => "false",
        Token::And => "and",
        Token::Or => "or",
        Token::Bang => "not",
        _ => return None,
    })
}

#[path = "declarations/declarations.rs"]
mod declarations;
#[path = "expressions/expressions.rs"]
mod expressions;
#[path = "statements/statements.rs"]
mod statements;
#[path = "types/types.rs"]
mod types;

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        let token_ranges = vec![Span::DUMMY.to_range(); tokens.len()];
        Self::new_with_ranges(tokens, token_ranges)
    }

    pub fn from_source(source: &str) -> Result<Self, crate::lexer::LexError> {
        let spanned = crate::lexer::lex_spanned(source)?;
        let (tokens, ranges) = spanned.into_iter().unzip();
        Ok(Self::new_with_ranges(tokens, ranges))
    }

    pub fn new_with_ranges(tokens: Vec<Token>, token_ranges: Vec<Range<usize>>) -> Self {
        Self {
            tokens,
            token_ranges,
            pos: 0,
            type_parameter_scopes: Vec::new(),
            pending_error: None,
            depth: 0,
            omitted_terminators: 0,
            question_decisions: HashMap::default(),
            await_operand: false,
            cancellation: None,
        }
    }

    pub fn with_cancellation(
        mut self,
        cancellation: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        self.cancellation = Some(cancellation);
        self
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation
            .as_ref()
            .is_some_and(|cancellation| cancellation.load(std::sync::atomic::Ordering::Acquire))
    }

    pub(crate) fn current_span(&self) -> Span {
        self.token_ranges
            .get(self.pos)
            .cloned()
            .map_or(Span::DUMMY, Span::from)
    }

    fn failure_span(&self) -> Option<Range<usize>> {
        let range = self.token_ranges.get(self.pos)?;
        (!Span::from(range.clone()).is_dummy()).then(|| range.clone())
    }

    fn fail(&mut self, message: String) -> String {
        let span = self.failure_span();
        self.stash_plain(message, span)
    }

    fn fail_consumed(&mut self, message: String) -> String {
        let span = self.previous_span();
        let span = (!span.is_dummy()).then(|| span.to_range());
        self.stash_plain(message, span)
    }

    fn stash_plain(&mut self, message: String, span: Option<Range<usize>>) -> String {
        if self.pending_error.is_none() {
            self.pending_error = Some(ParseError {
                message: message.clone(),
                code: None,
                span,
                help: Vec::new(),
                at_end_of_input: self.at_end_of_input(),
            });
        }
        message
    }

    fn stash_coded_error(
        &mut self,
        code: &'static str,
        message: impl Into<String>,
        span: Span,
        help: impl Into<String>,
    ) -> String {
        let message = message.into();
        self.pending_error = Some(ParseError {
            message: message.clone(),
            code: Some(code),
            span: (!span.is_dummy()).then(|| span.to_range()),
            help: vec![help.into()],
            at_end_of_input: false,
        });
        message
    }

    pub(super) fn stash_protect_default_error(&mut self, span: Span) -> String {
        self.stash_coded_error(
            "E3309",
            "`protect` cannot be combined with a default export",
            span,
            "drop `protect`, or give the value a name and write `export protect <name>;`",
        )
    }

    pub(super) fn stash_protect_re_export_error(&mut self, span: Span) -> String {
        self.stash_coded_error(
            "E3310",
            "`protect` cannot be applied to a re-export",
            span,
            "write `export protect` on the declaration in the file that owns it; a re-export keeps the scope of the original declaration",
        )
    }

    fn stash_intersection_error(&mut self, span: Span) -> String {
        let message =
            "`&` combines types only in a `type` alias; it is not allowed in a type annotation"
                .to_string();
        self.pending_error = Some(ParseError {
            message: message.clone(),
            code: Some("E3215"),
            span: (!span.is_dummy()).then(|| span.to_range()),
            help: vec![
                "declare the intersection with `type Name = A & B;` and use `Name` here"
                    .to_string(),
            ],
            at_end_of_input: false,
        });
        message
    }

    fn stash_intersection_union_error(&mut self, span: Span) -> String {
        let message =
            "`&` and `|` cannot be combined in the same type alias; `|` binds tighter, so the \
             intersection would take a union as an operand"
                .to_string();
        self.pending_error = Some(ParseError {
            message: message.clone(),
            code: Some("E3217"),
            span: (!span.is_dummy()).then(|| span.to_range()),
            help: vec![
                "declare the intersection under its own name and use that name in the union, as \
                 in `type AB = A & B;` followed by `type T = AB | C;`"
                    .to_string(),
                "a union is not an object type, so parentheses alone do not make `A & (B | C)` \
                 valid; only the named form above combines the two"
                    .to_string(),
            ],
            at_end_of_input: false,
        });
        message
    }

    fn at_end_of_input(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    pub(crate) fn previous_span(&self) -> Span {
        self.pos
            .checked_sub(1)
            .and_then(|index| self.token_ranges.get(index))
            .cloned()
            .map_or(Span::DUMMY, Span::from)
    }

    pub(crate) fn start_span(&self) -> Span {
        self.current_span()
    }

    pub(crate) fn finish_span(&self, start: Span) -> Span {
        start.join(self.previous_span())
    }

    fn spanned(&self, kind: ExprKind, start: Span) -> Expr {
        Expr::new(kind, self.finish_span(start))
    }

    fn enter_recursion(&mut self, what: &str) -> Result<(), String> {
        self.depth += 1;
        if self.depth > MAX_RECURSION_DEPTH {
            return Err(format!("{what} is nested too deeply"));
        }
        Ok(())
    }

    fn expect_binding_ident(&mut self, kind: BindingKind) -> Result<String, String> {
        let span = self.token_ranges.get(self.pos).cloned();
        match self.peek().cloned() {
            Some(Token::Ident(name)) if crate::keywords::is_reserved_future(&name) => {
                self.advance();
                Err(self.stash_reserved_error(&name, crate::keywords::ReservedKind::Future, span))
            }
            Some(Token::Ident(name)) if crate::keywords::is_ambient_builtin(&name) => {
                self.advance();
                Err(self.stash_reserved_error(&name, crate::keywords::ReservedKind::Builtin, span))
            }
            Some(Token::Ident(name)) => {
                self.advance();
                Ok(name)
            }
            Some(Token::At) => {
                self.advance();
                Ok("at".to_string())
            }
            Some(ref token) if keyword_as_name(token).is_some() => {
                let name = keyword_as_name(token).unwrap().to_string();
                self.advance();
                Err(self.stash_reserved_error(&name, crate::keywords::ReservedKind::Active, span))
            }
            other => {
                self.advance();
                Err(format!(
                    "Expected {} name, found {}",
                    kind.label(),
                    found(&other)
                ))
            }
        }
    }

    fn stash_reserved_error(
        &mut self,
        name: &str,
        kind: crate::keywords::ReservedKind,
        span: Option<Range<usize>>,
    ) -> String {
        let message = match kind {
            crate::keywords::ReservedKind::Active => format!(
                "`{name}` is a reserved keyword in tyrus and cannot be used as an identifier"
            ),
            crate::keywords::ReservedKind::Future => format!(
                "`{name}` is reserved for future use in tyrus and cannot be used as an identifier"
            ),
            crate::keywords::ReservedKind::Builtin => {
                format!("`{name}` is an ambient builtin in tyrus and cannot be redeclared")
            }
        };
        let help = match crate::keywords::reserved_kind(name) {
            Some(crate::keywords::ReservedKind::Builtin) => format!(
                "rename `{name}` to an identifier that does not collide with a builtin function"
            ),
            _ => format!("rename `{name}` to an identifier that is not a reserved keyword"),
        };
        self.pending_error = Some(ParseError {
            message: message.clone(),
            code: Some("E0003"),
            span,
            help: vec![help],
            at_end_of_input: false,
        });
        message
    }

    fn reject_reserved_name(
        &mut self,
        name: &str,
        span: Option<Range<usize>>,
    ) -> Result<(), String> {
        if crate::keywords::is_reserved_future(name) {
            return Err(self.stash_reserved_error(
                name,
                crate::keywords::ReservedKind::Future,
                span,
            ));
        }
        if crate::keywords::is_ambient_builtin(name) {
            return Err(self.stash_reserved_error(
                name,
                crate::keywords::ReservedKind::Builtin,
                span,
            ));
        }
        Ok(())
    }

    fn stash_await_missing_expression_error(&mut self, span: Option<Range<usize>>) -> String {
        let message = "`await` must be followed by an expression".to_string();
        self.pending_error = Some(ParseError {
            message: message.clone(),
            code: Some("async-await-missing-expression"),
            span,
            help: vec!["write an expression after `await` or remove `await`".to_string()],
            at_end_of_input: self.at_end_of_input(),
        });
        message
    }

    fn can_start_unary_expression(token: &Token) -> bool {
        matches!(
            token,
            Token::PlusPlus
                | Token::MinusMinus
                | Token::Bang
                | Token::Minus
                | Token::Plus
                | Token::Tilde
                | Token::Await
                | Token::Ampersand
                | Token::StringLit(_)
                | Token::RegexLit(_)
                | Token::TemplateLit(_)
                | Token::IntLit(_)
                | Token::IntOverflowLit(_)
                | Token::FloatLit(_)
                | Token::SpecialFloatLit(_)
                | Token::FloatOverflowLit
                | Token::True
                | Token::False
                | Token::Null
                | Token::None
                | Token::Percent
                | Token::Typeof
                | Token::Function
                | Token::Async
                | Token::Comptime
                | Token::LBrace
                | Token::LBracket
                | Token::LParen
                | Token::Ident(_)
                | Token::TyI8
                | Token::TyI16
                | Token::TyI32
                | Token::TyI64
                | Token::TyI128
                | Token::TyIsize
                | Token::TyU8
                | Token::TyU16
                | Token::TyU32
                | Token::TyU64
                | Token::TyU128
                | Token::TyUsize
                | Token::TyF32
                | Token::TyF64
                | Token::TyF128
                | Token::TyDecimal
                | Token::If
        )
    }

    /// Hands the token stream back after parsing, so a caller that needs the
    /// tokens as well (the analysis pass, for its symbols and ranges) can move
    /// them out instead of cloning the whole stream to lend it.
    pub fn into_tokens(self) -> (Vec<Token>, Vec<Range<usize>>) {
        (self.tokens, self.token_ranges)
    }

    /// The range of the token the parser stopped at, for an error that carries
    /// no span of its own.
    pub fn position_range(&self) -> Option<Range<usize>> {
        self.token_ranges.get(self.position()).cloned()
    }

    pub fn position(&self) -> usize {
        self.pos.min(self.tokens.len().saturating_sub(1))
    }

    /// Where the parser actually stopped, without the clamp `position` applies
    /// for reporting. Error recovery needs the real index to splice a token in.
    pub fn cursor(&self) -> usize {
        self.pos
    }

    fn mark(&self) -> Mark {
        Mark {
            pos: self.pos,
            pending_error: self.pending_error.clone(),
        }
    }

    fn reset(&mut self, mark: Mark) {
        self.pos = mark.pos;
        self.pending_error = mark.pending_error;
    }

    fn peek(&self) -> Option<&Token> {
        if self.is_cancelled() {
            return None;
        }
        self.tokens.get(self.pos)
    }

    pub(crate) fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn peek2(&self) -> Option<&Token> {
        if self.is_cancelled() {
            return None;
        }
        self.tokens.get(self.pos + 1)
    }

    fn arrow_signature_follows_eq(&self) -> bool {
        self.arrow_signature_at(self.pos + 1)
    }

    fn arrow_signature_at(&self, start: usize) -> bool {
        let mut index = start;
        match self.tokens.get(index) {
            Some(Token::Lt) => return true,
            Some(Token::LParen) => {}
            _ => return false,
        }
        let mut depth = 0usize;
        while let Some(token) = self.tokens.get(index) {
            match token {
                Token::LParen => depth += 1,
                Token::RParen => {
                    depth -= 1;
                    if depth == 0 {
                        return matches!(
                            self.tokens.get(index + 1),
                            Some(Token::Arrow | Token::Colon)
                        );
                    }
                }
                _ => {}
            }
            index += 1;
        }
        false
    }

    fn advance(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        self.pos += 1;
        t
    }

    fn token_start(&self, index: usize) -> usize {
        self.token_ranges
            .get(index)
            .map(|range| range.start)
            .unwrap_or(index)
    }

    fn token_end(&self, index: usize) -> usize {
        self.token_ranges
            .get(index)
            .map(|range| range.end)
            .unwrap_or(index)
    }

    fn previous_token_end(&self) -> usize {
        self.pos
            .checked_sub(1)
            .map(|index| self.token_end(index))
            .unwrap_or(0)
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        match self.peek() {
            Some(t) if t == expected => {
                self.advance();
                Ok(())
            }
            Some(t) => {
                let message = format!("Expected {expected:?}, found {t:?}");
                Err(self.fail(message))
            }
            None => {
                let message = format!("Expected {expected:?}, found end of file");
                Err(self.fail(message))
            }
        }
    }

    fn skip_semicolon(&mut self) {
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }
    }

    /// Consumes the `;` that closes a declaration, which tyrus requires. An
    /// optional terminator also made the grammar ambiguous: a declaration could
    /// end at its own value, and whatever followed was read as a new statement.
    fn expect_semicolon(&mut self, what: &str) -> Result<(), String> {
        if self.omitted_terminators > 0 {
            self.skip_semicolon();
            return Ok(());
        }
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
            return Ok(());
        }
        let span = self.previous_span();
        let message = format!("expected `;` after this {what}");
        self.pending_error = Some(ParseError {
            message: message.clone(),
            code: Some("E3218"),
            span: (!span.is_dummy()).then(|| span.to_range()),
            help: vec![format!("end the {what} with `;`")],
            at_end_of_input: self.at_end_of_input(),
        });
        Err(message)
    }

    /// A statement whose expression ends in a block, such as `if`, `match` or a
    /// loop, closes itself with the `}`, so it needs no `;`, the way it needs
    /// none in every language with block expressions. The two constructs a
    /// delimiter closes instead, `do ... while (cond)` and the update clause of a
    /// classic `for`, are handled where they are parsed.
    fn expect_statement_terminator(&mut self, what: &str) -> Result<(), String> {
        if matches!(self.previous_token(), Some(Token::RBrace)) {
            self.skip_semicolon();
            return Ok(());
        }
        self.expect_semicolon(what)
    }

    fn parse_stmt_without_terminator(&mut self) -> Result<Stmt, String> {
        self.omitted_terminators += 1;
        let statement = self.parse_stmt();
        self.omitted_terminators -= 1;
        statement
    }

    fn previous_token(&self) -> Option<&Token> {
        self.pos
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
    }

    fn expect_gt(&mut self) -> Result<(), String> {
        match self.peek() {
            Some(Token::Gt) => {
                self.advance();
                Ok(())
            }
            Some(Token::Shr) => {
                self.tokens[self.pos] = Token::Gt;
                Ok(())
            }
            Some(Token::UShr) => {
                self.tokens[self.pos] = Token::Shr;
                Ok(())
            }
            other => Err(format!("Expected `>`, found {}", found(&other))),
        }
    }

    fn parse_inline_object_type(&mut self) -> Result<Type, String> {
        self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();
        while !matches!(self.peek(), Some(Token::RBrace) | None) {
            let field_start = self.start_span();
            let decorators = self.parse_decorators()?;
            let readonly = matches!(self.peek(), Some(Token::Readonly))
                && !matches!(self.peek2(), Some(Token::Colon | Token::Question));
            if readonly {
                self.advance();
            }
            let field_name = match self.advance() {
                Some(Token::Ident(name)) => name.clone(),
                Some(token) if keyword_as_name(token).is_some() => {
                    keyword_as_name(token).unwrap().to_string()
                }
                other => return Err(format!("Expected field name, found {}", found(&other))),
            };
            let optional = if matches!(self.peek(), Some(Token::Question)) {
                self.advance();
                true
            } else {
                false
            };
            self.expect(&Token::Colon)?;
            let ty = self.parse_annotation_type()?;
            let span = self.finish_span(field_start);
            self.skip_semicolon();
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            }
            fields.push(Field {
                name: field_name,
                ty,
                readonly,
                optional,
                decorators,
                span,
            });
        }
        self.expect(&Token::RBrace)?;
        Ok(Type::Object(fields))
    }

    fn consume_readonly_binding_prefix(&mut self) -> Result<bool, String> {
        if matches!(self.peek(), Some(Token::Readonly))
            && matches!(self.peek2(), Some(Token::Const | Token::Let))
        {
            self.advance();
            return Ok(true);
        }
        Ok(false)
    }

    fn reject_trailing_readonly_binding_prefix(&mut self) -> Result<(), String> {
        if matches!(self.peek(), Some(Token::Readonly)) {
            return Err(READONLY_ORDER_MESSAGE.to_string());
        }
        Ok(())
    }

    fn parse_var_decl_type(&mut self) -> Result<Type, String> {
        if matches!(self.peek(), Some(Token::Colon)) {
            self.advance();
            return self.parse_annotation_type();
        }
        if matches!(self.peek(), Some(Token::Eq)) {
            return Ok(Type::Infer);
        }
        Err(
            "type annotation required: a declaration without an initializer cannot be inferred"
                .to_string(),
        )
    }

    fn parse_var_decl_value(
        &mut self,
        mutable: bool,
    ) -> Result<(Option<Expr>, Option<OnFail>), String> {
        if matches!(self.peek(), Some(Token::Eq)) {
            self.advance();
            let value = self.parse_expr()?;
            let onfail = self.parse_onfail()?;
            return Ok((Some(value), onfail));
        }
        if !mutable {
            return Err(
                "`const` must be initialized; use `let` for deferred initialization, or add `= value`"
                    .to_string(),
            );
        }
        Ok((None, None))
    }

    fn parse_onfail(&mut self) -> Result<Option<OnFail>, String> {
        if !matches!(self.peek(), Some(Token::OnFail)) {
            return Ok(None);
        }

        let start = self.start_span();
        self.advance();
        let handler = if matches!(self.peek(), Some(Token::LBrace)) {
            OnFailHandler::Block(self.parse_block()?)
        } else {
            OnFailHandler::Expr(self.parse_expr()?)
        };

        Ok(Some(OnFail {
            handler,
            span: self.finish_span(start),
        }))
    }

    fn parse_base_type(&mut self) -> Result<Type, String> {
        let __start = self.start_span();
        match self.advance() {
            Some(Token::TyI8) => Ok(Type::I8),
            Some(Token::TyI16) => Ok(Type::I16),
            Some(Token::TyI32) => Ok(Type::I32),
            Some(Token::TyI64) => Ok(Type::I64),
            Some(Token::TyI128) => Ok(Type::I128),
            Some(Token::TyIsize) => Ok(Type::Isize),
            Some(Token::TyU8) => Ok(Type::U8),
            Some(Token::TyU16) => Ok(Type::U16),
            Some(Token::TyU32) => Ok(Type::U32),
            Some(Token::TyU64) => Ok(Type::U64),
            Some(Token::TyU128) => Ok(Type::U128),
            Some(Token::TyUsize) => Ok(Type::Usize),
            Some(Token::TyF32) => Ok(Type::F32),
            Some(Token::TyF64) => Ok(Type::F64),
            Some(Token::TyF128) => Ok(Type::F128),
            Some(Token::TyDecimal) => Ok(Type::Decimal),
            Some(Token::TyChar) => Ok(Type::Char),
            Some(Token::TyBool) => Ok(Type::Bool),
            Some(Token::TyString) => Ok(Type::Str),
            Some(Token::TyVoid) => Ok(Type::Void),
            Some(Token::TyUnknown) => Ok(Type::Unknown),
            Some(Token::TyNever) => Ok(Type::Never),
            Some(Token::StringLit(s)) => Ok(Type::Lit(Box::new(
                ExprKind::StrLit(s.clone()).spanned(self.finish_span(__start)),
            ))),
            Some(Token::IntLit(n)) => Ok(Type::Lit(Box::new(
                ExprKind::IntLit(*n).spanned(self.finish_span(__start)),
            ))),
            Some(Token::FloatLit(n)) => Ok(Type::Lit(Box::new(
                ExprKind::FloatLit(n.clone()).spanned(self.finish_span(__start)),
            ))),
            Some(Token::SpecialFloatLit(n)) => Ok(Type::Lit(Box::new(
                ExprKind::FloatLit(n.clone()).spanned(self.finish_span(__start)),
            ))),
            Some(Token::True) => Ok(Type::Lit(Box::new(
                ExprKind::BoolLit(true).spanned(self.finish_span(__start)),
            ))),
            Some(Token::False) => Ok(Type::Lit(Box::new(
                ExprKind::BoolLit(false).spanned(self.finish_span(__start)),
            ))),
            Some(Token::Null) => Err(
                "`null` is not a type; use `T?` to make a type nullable (e.g. `string?`)"
                    .to_string(),
            ),
            Some(Token::Typeof) => {
                let expr = self.parse_expr()?;
                Ok(Type::TypeQuery(Box::new(expr)))
            }
            Some(Token::Ident(name)) => {
                let name = name.clone();
                if matches!(self.peek(), Some(Token::Lt)) {
                    let args = self.parse_type_arguments()?;
                    return Ok(Type::Apply { name, args });
                }
                if name == "Regex" {
                    Ok(Type::Regex)
                } else if self
                    .type_parameter_scopes
                    .iter()
                    .rev()
                    .any(|scope| scope.contains(&name))
                {
                    Ok(Type::Generic(name))
                } else {
                    Ok(Type::Named(name))
                }
            }
            Some(t) => {
                let message = format!("Expected type, found {t:?}");
                Err(self.fail_consumed(message))
            }
            None => {
                let message = "Expected type, found end of file".to_string();
                Err(self.fail(message))
            }
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        const PARSER_STACK_SIZE: usize = 64 * 1024 * 1024;
        let cancellation = self.cancellation.as_ref().map(std::sync::Arc::clone);
        let result = std::thread::scope(|scope| {
            std::thread::Builder::new()
                .name("tyrus-parser".into())
                .stack_size(PARSER_STACK_SIZE)
                .spawn_scoped(scope, move || {
                    self.parse_program_inner().map_err(|message| {
                        let mut error = match self.pending_error.take() {
                            Some(stashed)
                                if stashed.code.is_some() || stashed.message == message =>
                            {
                                stashed
                            }
                            _ => ParseError::new(message),
                        };
                        error.at_end_of_input |= self.at_end_of_input();
                        error
                    })
                })
                .expect("failed to spawn the tyrus parser thread")
                .join()
                .unwrap_or_else(|payload| std::panic::resume_unwind(payload))
        });
        if cancellation
            .as_ref()
            .is_some_and(|signal| signal.load(std::sync::atomic::Ordering::Acquire))
        {
            return Err(ParseError::new("analysis cancelled"));
        }
        result
    }

    fn parse_program_inner(&mut self) -> Result<Program, String> {
        let __stmt_start = self.start_span();
        if self.tokens.len() > MAX_TOKENS {
            return Err(format!(
                "input is too large: {} tokens (limit {MAX_TOKENS})",
                self.tokens.len()
            ));
        }
        let mut items = vec![];
        let mut imports = vec![];
        let mut exports = vec![];

        while self.peek().is_some() {
            if matches!(self.peek(), Some(Token::Import)) {
                imports.push(self.parse_import()?);
                continue;
            }
            let mut exported = matches!(self.peek(), Some(Token::AtSign));
            let mut visibility = Visibility::Public;
            let mut default_export_span = None;
            if matches!(self.peek(), Some(Token::Export)) {
                let export_start = self.start_span();
                self.advance();

                let protect_span = if matches!(self.peek(), Some(Token::Protect)) {
                    let protect_start = self.start_span();
                    self.advance();
                    visibility = Visibility::Protect;
                    Some(self.finish_span(protect_start))
                } else {
                    None
                };
                if let Some(span) = protect_span
                    && matches!(self.peek(), Some(Token::Default))
                {
                    return Err(self.stash_protect_default_error(span));
                }

                if let Some(clause) = self.parse_export_clause(visibility, protect_span)? {
                    exports.extend(clause);
                    continue;
                }
                if matches!(self.peek(), Some(Token::Default)) {
                    self.advance();
                    if matches!(self.peek(), Some(Token::Protect)) {
                        let protect_start = self.start_span();
                        self.advance();
                        let span = self.finish_span(protect_start);
                        return Err(self.stash_protect_default_error(span));
                    }
                    default_export_span = Some(self.finish_span(export_start));
                } else {
                    exported = true;
                }
            }

            if let Some(span) = default_export_span {
                if matches!(self.peek(), Some(Token::Ident(_)))
                    && matches!(self.peek2(), Some(Token::Semicolon) | None)
                {
                    let local = match self.advance() {
                        Some(Token::Ident(name)) => name.clone(),
                        other => {
                            return Err(format!("Expected exported name, found {}", found(&other)));
                        }
                    };
                    self.expect_semicolon("export")?;
                    exports.push(ExportDecl::Named {
                        exported: DEFAULT_EXPORT.to_string(),
                        local,
                        inline: false,
                        visibility: Visibility::Public,
                        span,
                    });
                    continue;
                }
                if matches!(
                    self.peek(),
                    Some(
                        Token::Function
                            | Token::Validator
                            | Token::Const
                            | Token::Let
                            | Token::Async
                            | Token::Comptime
                    )
                ) {
                    let item = self.parse_item()?;
                    let name = match &item {
                        Item::FnDecl { name, .. }
                        | Item::Stmt(Stmt {
                            kind: StmtKind::VarDecl { name, .. },
                            ..
                        }) => name.clone(),
                        _ => {
                            return Err(
                                "`export default` must be followed by an expression or a named declaration"
                                    .to_string(),
                            )
                        }
                    };
                    items.push(item);
                    exports.push(ExportDecl::Named {
                        exported: DEFAULT_EXPORT.to_string(),
                        local: name,
                        inline: true,
                        visibility: Visibility::Public,
                        span,
                    });
                    continue;
                }
                if self.arrow_signature_at(self.pos) {
                    let local = format!("__tyrus_default_export_{}", items.len());
                    let (type_params, params, ret_type, throws, body) =
                        self.parse_arrow_fn_parts()?;
                    self.skip_semicolon();
                    items.push(Item::FnDecl {
                        name: local.clone(),
                        is_async: false,
                        is_comptime: false,
                        is_validator: false,
                        type_params,
                        params,
                        ret_type,
                        throws,
                        body,
                        span,
                    });
                    exports.push(ExportDecl::Named {
                        exported: DEFAULT_EXPORT.to_string(),
                        local,
                        inline: true,
                        visibility: Visibility::Public,
                        span,
                    });
                    continue;
                }
                return Err(
                    "`export default` needs a named declaration or a function; bind the value first with `const name = ...;` and write `export default name;`"
                        .to_string(),
                );
            }

            let item = self.parse_item()?;
            if exported {
                let name = match &item {
                    Item::InterfaceDecl { name, .. }
                    | Item::BehaviorInterfaceDecl { name, .. }
                    | Item::FnDecl { name, .. }
                    | Item::TypeAlias { name, .. }
                    | Item::EnumDecl { name, .. }
                    | Item::Stmt(Stmt {
                        kind: StmtKind::VarDecl { name, .. },
                        ..
                    }) => name.clone(),
                    _ => {
                        return Err(
                            "`export` must be followed by a top-level declaration".to_string()
                        );
                    }
                };
                exports.push(ExportDecl::Named {
                    exported: name.clone(),
                    local: name,
                    inline: true,
                    visibility,
                    span: item.span(),
                });
            }
            items.push(item);
        }

        Ok(Program {
            imports,
            exports,
            items,
            auto_route_exports: Vec::new(),
        })
    }

    #[allow(clippy::too_many_lines)]
    fn parse_item(&mut self) -> Result<Item, String> {
        let __stmt_start = self.start_span();
        let mut timestamps = false;
        let model_decorator = if matches!(self.peek(), Some(Token::AtSign)) {
            let decorators = self.parse_decorators()?;
            let mut model = None;
            for decorator in decorators {
                match decorator.name.as_str() {
                    "model" if model.is_none() => model = Some(decorator),
                    "timestamps" if decorator.args.is_empty() && !timestamps => {
                        timestamps = true;
                    }
                    _ => {
                        return Err(
                            "interfaces accept `@model(\"table\")` and optional `@timestamps` metadata".to_string(),
                        );
                    }
                }
            }
            model
        } else {
            None
        };
        if model_decorator.is_some() && matches!(self.peek(), Some(Token::Export)) {
            self.advance();
        }
        if model_decorator.is_some() && !matches!(self.peek(), Some(Token::Interface)) {
            return Err("`@model` must be followed by an interface declaration".to_string());
        }
        if matches!(self.peek(), Some(Token::Type)) {
            self.advance();
            let name = self.expect_binding_ident(BindingKind::Type)?;
            let type_params = self.parse_type_params()?;
            self.type_parameter_scopes
                .push(type_params.iter().map(|param| param.name.clone()).collect());
            self.expect(&Token::Eq)?;
            if matches!(self.peek(), Some(Token::Pipe)) {
                self.advance();
            }
            let mut operands = self.parse_type_alias_body()?;
            let terminator = self.expect_semicolon("type alias");
            self.type_parameter_scopes.pop();
            terminator?;
            if operands.len() > 1 {
                return Ok(Item::InterfaceDecl {
                    name,
                    type_params,
                    extends: operands,
                    fields: Vec::new(),
                    db_model: None,
                    from_type_alias: true,
                    span: self.finish_span(__stmt_start),
                });
            }
            let ty = operands.pop().expect("the alias body has one operand");
            match ty {
                Type::Union(variants) => {
                    return Ok(Item::TypeAlias {
                        name,
                        type_params,
                        ty: Type::Union(variants),
                        merged_fields: vec![],
                        span: self.finish_span(__stmt_start),
                    });
                }
                Type::Object(fields) => {
                    return Ok(Item::InterfaceDecl {
                        name,
                        type_params,
                        extends: Vec::new(),
                        fields,
                        db_model: None,
                        from_type_alias: true,
                        span: self.finish_span(__stmt_start),
                    });
                }
                other => {
                    return Ok(Item::TypeAlias {
                        name,
                        type_params,
                        ty: other,
                        merged_fields: vec![],
                        span: self.finish_span(__stmt_start),
                    });
                }
            }
        }
        if matches!(self.peek(), Some(Token::Enum)) {
            self.advance();
            let name = self.expect_binding_ident(BindingKind::Enum)?;
            let repr = if matches!(self.peek(), Some(Token::Colon)) {
                self.advance();
                self.parse_type()?
            } else {
                Type::Void
            };
            self.expect(&Token::LBrace)?;
            let mut members = Vec::new();
            let mut member_names = HashSet::default();
            while !matches!(self.peek(), Some(Token::RBrace) | None) {
                let member_span = self.failure_span();
                let member_name = self.expect_binding_ident(BindingKind::Enum)?;
                if !member_names.insert(member_name.clone()) {
                    let message =
                        format!("enum variant `{member_name}` is declared more than once");
                    self.pending_error = Some(ParseError {
                        message: message.clone(),
                        code: Some("E0428"),
                        span: member_span,
                        help: vec![format!("rename the duplicate variant `{member_name}`")],
                        at_end_of_input: false,
                    });
                    return Err(message);
                }
                let (payload, value) = if matches!(self.peek(), Some(Token::LParen)) {
                    self.advance();
                    let mut types = Vec::new();
                    while !matches!(self.peek(), Some(Token::RParen) | None) {
                        types.push(self.parse_type()?);
                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.expect(&Token::RParen)?;
                    (EnumPayload::Tuple(types), None)
                } else if matches!(self.peek(), Some(Token::LBrace)) {
                    self.advance();
                    let mut fields = Vec::new();
                    while !matches!(self.peek(), Some(Token::RBrace) | None) {
                        let field_name = self.expect_binding_ident(BindingKind::Variable)?;
                        self.expect(&Token::Colon)?;
                        let field_type = self.parse_annotation_type()?;
                        fields.push(Field {
                            name: field_name,
                            ty: field_type,
                            readonly: false,
                            optional: false,
                            decorators: Vec::new(),
                            span: self.finish_span(self.start_span()),
                        });
                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.expect(&Token::RBrace)?;
                    (EnumPayload::Struct(fields), None)
                } else {
                    let value = if matches!(self.peek(), Some(Token::Eq)) {
                        self.advance();
                        Some(self.parse_expr()?)
                    } else {
                        None
                    };
                    (EnumPayload::Unit, value)
                };
                members.push(EnumMember {
                    name: member_name,
                    value,
                    payload,
                });
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(&Token::RBrace)?;
            self.skip_semicolon();
            return Ok(Item::EnumDecl {
                name,
                repr,
                members,
                span: self.finish_span(__stmt_start),
            });
        }
        if matches!(self.peek(), Some(Token::Interface)) {
            self.advance();
            let name = self.expect_binding_ident(BindingKind::Type)?;
            let type_params = self.parse_type_params()?;
            self.type_parameter_scopes
                .push(type_params.iter().map(|param| param.name.clone()).collect());
            let mut targets = Vec::new();
            if matches!(self.peek(), Some(Token::At)) {
                self.advance();
                loop {
                    targets.push(self.parse_type()?);
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            let mut extends = Vec::new();
            if matches!(self.peek(), Some(Token::Extends)) {
                self.advance();
                loop {
                    extends.push(self.parse_type()?);
                    if matches!(self.peek(), Some(Token::Comma)) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            self.expect(&Token::LBrace)?;
            let behavior_body = matches!(
                (self.peek(), self.peek2()),
                (
                    Some(Token::Ident(_)),
                    Some(Token::LParen | Token::Semicolon)
                )
            );
            if !targets.is_empty() || behavior_body {
                let mut methods = Vec::new();
                let mut bindings = Vec::new();
                while !matches!(self.peek(), Some(Token::RBrace) | None) {
                    let method_start = self.start_span();
                    let method_name = self.expect_binding_ident(BindingKind::Function)?;
                    if !matches!(self.peek(), Some(Token::LParen)) {
                        bindings.push(method_name);
                        self.skip_semicolon();
                        continue;
                    }
                    let (method_type_params, params, ret_type, throws, body) =
                        self.parse_fn_body_parts()?;
                    methods.push(BehaviorMethod {
                        name: method_name,
                        type_params: method_type_params,
                        params,
                        ret_type,
                        throws,
                        body,
                        span: self.finish_span(method_start),
                    });
                    self.skip_semicolon();
                }
                self.expect(&Token::RBrace)?;
                self.skip_semicolon();
                self.type_parameter_scopes.pop();
                return Ok(Item::BehaviorInterfaceDecl {
                    name,
                    type_params,
                    targets,
                    extends,
                    methods,
                    bindings,
                    span: self.finish_span(__stmt_start),
                });
            }
            let mut fields = Vec::new();
            let mut db_model_fields = Vec::new();
            let mut db_model_relations = Vec::new();
            while !matches!(self.peek(), Some(Token::RBrace) | None) {
                let field_start = self.start_span();
                let mut decorators = self.parse_decorators()?;
                let readonly = matches!(self.peek(), Some(Token::Readonly))
                    && !matches!(self.peek2(), Some(Token::Colon | Token::Question));
                if readonly {
                    self.advance();
                }
                let field_name = match self.advance() {
                    Some(Token::Ident(name)) => name.clone(),
                    Some(token) if keyword_as_name(token).is_some() => {
                        keyword_as_name(token).unwrap().to_string()
                    }
                    other => return Err(format!("Expected field name, found {}", found(&other))),
                };
                let optional = if matches!(self.peek(), Some(Token::Question)) {
                    self.advance();
                    true
                } else {
                    false
                };
                self.expect(&Token::Colon)?;
                let ty = self.parse_annotation_type()?;
                let span = self.finish_span(field_start);
                if model_decorator.is_some() {
                    let (remaining, metadata, relation) =
                        parse_db_model_field_decorators(decorators, &field_name, &ty, span)?;
                    decorators = remaining;
                    if let Some(relation) = relation {
                        db_model_relations.push(relation);
                    } else {
                        db_model_fields.push(metadata);
                    }
                }
                self.skip_semicolon();
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                }
                fields.push(Field {
                    name: field_name,
                    ty,
                    readonly,
                    optional,
                    decorators,
                    span,
                });
            }
            self.expect(&Token::RBrace)?;
            self.skip_semicolon();
            self.type_parameter_scopes.pop();
            let mut db_model = model_decorator
                .map(|decorator| {
                    parse_db_model_decorator(
                        decorator,
                        timestamps,
                        db_model_fields,
                        db_model_relations,
                    )
                })
                .transpose()?;
            if let Some(model) = &mut db_model {
                add_synthetic_timestamp_fields(model, &mut fields)?;
            }
            return Ok(Item::InterfaceDecl {
                name,
                type_params,
                extends,
                fields,
                db_model,
                from_type_alias: false,
                span: self.finish_span(__stmt_start),
            });
        }
        let is_validator = matches!(self.peek(), Some(Token::Validator));
        if is_validator {
            self.advance();
            if matches!(self.peek(), Some(Token::Const)) {
                self.advance();
                let name = self.expect_binding_ident(BindingKind::Variable)?;
                self.expect(&Token::Eq)?;
                if !self.arrow_signature_at(self.pos) {
                    return Err(
                        "`validator const` must be initialized with a synchronous arrow function"
                            .to_string(),
                    );
                }
                let (type_params, params, ret_type, throws, body) = self.parse_arrow_fn_parts()?;
                self.expect_semicolon("declaration")?;
                return Ok(Item::FnDecl {
                    name,
                    is_async: false,
                    is_comptime: false,
                    is_validator: true,
                    type_params,
                    params,
                    ret_type,
                    throws,
                    body,
                    span: self.finish_span(__stmt_start),
                });
            }
            if !matches!(self.peek(), Some(Token::Function)) {
                return Err(
                    "`validator` must be followed by `function` or `const name = (...) => ...`"
                        .to_string(),
                );
            }
        }
        let comptime_function = matches!(
            (self.peek(), self.peek2()),
            (Some(Token::Comptime), Some(Token::Function))
        );
        if matches!(self.peek(), Some(Token::Async | Token::Function))
            || comptime_function
            || is_validator
        {
            let is_async = matches!(self.peek(), Some(Token::Async));
            let is_comptime = matches!(self.peek(), Some(Token::Comptime));
            if is_validator && (is_async || is_comptime) {
                return Err(
                    "`validator function` cannot be combined with `async` or `comptime`"
                        .to_string(),
                );
            }
            if matches!(self.peek(), Some(Token::Async)) {
                self.advance();
            }
            if matches!(self.peek(), Some(Token::Comptime)) {
                self.advance();
            }
            if matches!(self.peek(), Some(Token::Function)) {
                self.advance();
                let name = self.expect_binding_ident(BindingKind::Function)?;
                let (type_params, params, ret_type, throws, body) = self.parse_fn_body_parts()?;
                self.skip_semicolon();
                return Ok(Item::FnDecl {
                    name,
                    is_async,
                    is_comptime,
                    is_validator,
                    type_params,
                    params,
                    ret_type,
                    throws,
                    body,
                    span: self.finish_span(__stmt_start),
                });
            }
        }
        let is_comptime = matches!(self.peek(), Some(Token::Comptime));
        if is_comptime {
            self.advance();
        }
        let is_readonly = self.consume_readonly_binding_prefix()?;
        if matches!(self.peek(), Some(Token::Const | Token::Let)) {
            let mutable = matches!(self.peek(), Some(Token::Let));
            self.advance();
            self.reject_trailing_readonly_binding_prefix()?;
            if is_readonly && mutable {
                return Err(READONLY_LET_MESSAGE.to_string());
            }

            if is_comptime && mutable {
                return Err(
                    "`comptime` may only qualify `function` or a `const` arrow function"
                        .to_string(),
                );
            }

            if matches!(self.peek(), Some(Token::LBracket)) {
                if is_readonly {
                    return Err(READONLY_DESTRUCTURE_MESSAGE.to_string());
                }
                return self.parse_array_destructure(mutable);
            }
            if matches!(self.peek(), Some(Token::LBrace)) {
                if is_readonly {
                    return Err(READONLY_DESTRUCTURE_MESSAGE.to_string());
                }
                return self.parse_object_destructure(mutable);
            }

            let name = self.expect_binding_ident(BindingKind::Variable)?;

            if is_readonly
                && matches!(self.peek(), Some(Token::Eq))
                && (self.arrow_signature_follows_eq()
                    || matches!(self.peek2(), Some(Token::Async | Token::Function)))
            {
                return Err(READONLY_FUNCTION_MESSAGE.to_string());
            }
            if matches!(self.peek(), Some(Token::Eq)) && self.arrow_signature_follows_eq() {
                self.advance();
                let (type_params, params, ret_type, throws, body) = self.parse_arrow_fn_parts()?;
                self.expect_semicolon("declaration")?;
                return Ok(Item::FnDecl {
                    name,
                    is_async: false,
                    is_comptime,
                    is_validator: false,
                    type_params,
                    params,
                    ret_type,
                    throws,
                    body,
                    span: self.finish_span(__stmt_start),
                });
            }

            if matches!(self.peek(), Some(Token::Eq))
                && matches!(self.peek2(), Some(Token::Async | Token::Function))
            {
                self.advance();
                let is_async = matches!(self.peek(), Some(Token::Async));
                if matches!(self.peek(), Some(Token::Async)) {
                    self.advance();
                }
                if matches!(self.peek(), Some(Token::Function)) {
                    self.advance();
                    if matches!(self.peek(), Some(Token::Ident(_))) {
                        self.advance();
                    }
                    let (type_params, params, ret_type, throws, body) =
                        self.parse_fn_body_parts()?;
                    self.expect_semicolon("declaration")?;
                    return Ok(Item::FnDecl {
                        name,
                        is_async,
                        is_comptime,
                        is_validator: false,
                        type_params,
                        params,
                        ret_type,
                        throws,
                        body,
                        span: self.finish_span(__stmt_start),
                    });
                }
                let (type_params, params, ret_type, throws, body) = self.parse_arrow_fn_parts()?;
                self.skip_semicolon();
                return Ok(Item::FnDecl {
                    name,
                    is_async,
                    is_comptime,
                    is_validator: false,
                    type_params,
                    params,
                    ret_type,
                    throws,
                    body,
                    span: self.finish_span(__stmt_start),
                });
            }

            if is_comptime {
                return Err("`comptime const` requires an arrow function initializer".to_string());
            }

            let ty = self.parse_var_decl_type()?;
            let ty = if is_readonly {
                readonly_binding_type(ty)?
            } else {
                ty
            };

            let (value, onfail) = self.parse_var_decl_value(mutable)?;
            self.expect_semicolon("declaration")?;

            return Ok(Item::Stmt(
                StmtKind::VarDecl {
                    mutable,
                    name,
                    ty,
                    value,
                    onfail,
                }
                .spanned(self.finish_span(__stmt_start)),
            ));
        } else if is_comptime {
            return Err("expected `function` or `const` after `comptime`".to_string());
        }

        let stmt = self.parse_stmt()?;
        Ok(Item::Stmt(stmt))
    }

    fn parse_assign_op(&mut self) -> Option<AssignOp> {
        let op = match self.peek()? {
            Token::Eq => AssignOp::Assign,
            Token::PlusEq => AssignOp::AddAssign,
            Token::MinusEq => AssignOp::SubAssign,
            Token::StarEq => AssignOp::MulAssign,
            Token::SlashEq => AssignOp::DivAssign,
            Token::PercentEq => AssignOp::ModAssign,
            Token::PowerEq => AssignOp::PowAssign,
            Token::AndEq => AssignOp::AndAssign,
            Token::OrEq => AssignOp::OrAssign,
            Token::AmpersandEq => AssignOp::BitAndAssign,
            Token::PipeEq => AssignOp::BitOrAssign,
            Token::CaretEq => AssignOp::BitXorAssign,
            Token::ShlEq => AssignOp::ShlAssign,
            Token::ShrEq => AssignOp::ShrAssign,
            Token::UShrEq => AssignOp::UShrAssign,
            Token::NullCoalesceEq => AssignOp::NullCoalesceAssign,
            _ => return None,
        };
        self.advance();
        Some(op)
    }

    fn finish_place_assignment(
        &mut self,
        target: Expr,
        op: AssignOp,
        start: Span,
    ) -> Result<Stmt, String> {
        let value = self.parse_expr()?;
        self.expect_semicolon("assignment")?;
        let kind = match target.kind {
            ExprKind::FieldAccess { object, field } => StmtKind::FieldAssign {
                object: *object,
                field,
                op,
                value,
            },
            ExprKind::IndexAccess { object, index } => match object.kind {
                ExprKind::Ident(name) if matches!(op, AssignOp::Assign) => StmtKind::IndexAssign {
                    name,
                    index: *index,
                    value,
                },
                ExprKind::Ident(_) => {
                    return Err(
                        "error: compound assignment to an indexed element is not supported"
                            .to_string(),
                    );
                }
                _ => {
                    return Err(
                        "error: assignment to this indexed target is not supported yet".to_string(),
                    );
                }
            },
            ExprKind::Ident(name) => StmtKind::Assign { name, op, value },
            _ => return Err("error: invalid assignment target".to_string()),
        };
        Ok(kind.spanned(self.finish_span(start)))
    }

    fn parse_pipe(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut left = self.parse_null_coalesce()?;

        while matches!(self.peek(), Some(Token::PipeOperator)) {
            self.advance();
            let right = self.parse_postfix()?;
            left = ExprKind::Pipeline {
                left: Box::new(left),
                right: Box::new(right),
            }
            .spanned(self.finish_span(__start));
        }

        Ok(left)
    }

    fn parse_null_coalesce(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut left = self.parse_or()?;

        while matches!(self.peek(), Some(Token::NullCoalesce)) {
            self.advance();
            let right = self.parse_or()?;
            left = ExprKind::BinOp {
                op: BinOp::NullCoalesce,
                left: Box::new(left),
                right: Box::new(right),
            }
            .spanned(self.finish_span(__start));
        }

        Ok(left)
    }

    /// Reads a ternary's first branch. A `?` inside it that looked ahead, saw an
    /// expression and then `:`, decided to open a ternary of its own; in
    /// `c ? load()? - x : y` that `:` belongs to the outer ternary, and taking
    /// it leaves the outer one without its `:`. When that happens, the last such
    /// decision inside the branch becomes a try and the branch is read again,
    /// until the branch ends at a `:` or no decision is left to revisit.
    fn parse_ternary_branch(&mut self) -> Result<Expr, String> {
        let branch_start = self.pos;
        let mark = self.mark();
        let depth = self.depth;
        loop {
            let branch = self.parse_expr()?;
            if matches!(self.peek(), Some(Token::Colon)) {
                return Ok(branch);
            }
            let revisit = self
                .question_decisions
                .iter()
                .filter(|(position, is_try)| {
                    !**is_try && (branch_start..self.pos).contains(*position)
                })
                .map(|(position, _)| *position)
                .max();
            let Some(question) = revisit else {
                return Ok(branch);
            };
            self.question_decisions.insert(question, true);
            self.reset(mark.clone());
            self.depth = depth;
        }
    }

    fn parse_ternary(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let expr = self.parse_pipe()?;
        if matches!(self.peek(), Some(Token::Question)) {
            self.advance();
            let then_expr = self.parse_ternary_branch()?;
            self.expect(&Token::Colon)?;
            let else_expr = self.parse_ternary()?;
            return Ok(ExprKind::Ternary {
                cond: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            }
            .spanned(self.finish_span(__start)));
        }
        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut left = self.parse_and()?;

        while matches!(self.peek(), Some(Token::Or)) {
            self.advance();
            let right = self.parse_and()?;
            left = ExprKind::BinOp {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            }
            .spanned(self.finish_span(__start));
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut left = self.parse_bitwise_or()?;

        while matches!(self.peek(), Some(Token::And)) {
            self.advance();
            let right = self.parse_bitwise_or()?;
            left = ExprKind::BinOp {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
            }
            .spanned(self.finish_span(__start));
        }

        Ok(left)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut left = self.parse_bitwise_xor()?;

        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            left = ExprKind::BinOp {
                op: BinOp::BitOr,
                left: Box::new(left),
                right: Box::new(right),
            }
            .spanned(self.finish_span(__start));
        }

        Ok(left)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut left = self.parse_bitwise_and()?;

        while matches!(self.peek(), Some(Token::Caret)) {
            self.advance();
            let right = self.parse_bitwise_and()?;
            left = ExprKind::BinOp {
                op: BinOp::BitXor,
                left: Box::new(left),
                right: Box::new(right),
            }
            .spanned(self.finish_span(__start));
        }

        Ok(left)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut left = self.parse_shift()?;

        while matches!(self.peek(), Some(Token::Ampersand)) {
            self.advance();
            let right = self.parse_shift()?;
            left = ExprKind::BinOp {
                op: BinOp::BitAnd,
                left: Box::new(left),
                right: Box::new(right),
            }
            .spanned(self.finish_span(__start));
        }

        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut left = self.parse_equality()?;

        loop {
            let op = match self.peek() {
                Some(Token::Shl) => BinOp::Shl,
                Some(Token::Shr) => BinOp::Shr,
                Some(Token::UShr) => BinOp::UShr,
                _ => break,
            };
            self.advance();
            let right = self.parse_equality()?;
            left = ExprKind::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            }
            .spanned(self.finish_span(__start));
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut left = self.parse_comparison()?;

        while matches!(self.peek(), Some(Token::NotEqEq)) {
            self.advance();
            let right = self.parse_comparison()?;
            left = ExprKind::BinOp {
                op: BinOp::Neq,
                left: Box::new(left),
                right: Box::new(right),
            }
            .spanned(self.finish_span(__start));
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut left = self.parse_chained_comparison()?;

        loop {
            let negate =
                matches!(self.peek(), Some(Token::Bang)) && matches!(self.peek2(), Some(Token::In));
            if negate {
                self.advance();
            } else if !matches!(self.peek(), Some(Token::In)) {
                break;
            }
            self.advance();
            let right = self.parse_chained_comparison()?;
            let in_expr = ExprKind::In {
                key: Box::new(left),
                object: Box::new(right),
            }
            .spanned(self.finish_span(__start));
            left = if negate {
                ExprKind::Not(Box::new(in_expr)).spanned(self.finish_span(__start))
            } else {
                in_expr
            };
        }

        loop {
            let negate =
                matches!(self.peek(), Some(Token::Bang)) && matches!(self.peek2(), Some(Token::Is));
            if negate {
                self.advance();
            } else if !matches!(self.peek(), Some(Token::Is)) {
                break;
            }
            self.advance();
            let ty = self.parse_type()?;
            let is_expr = ExprKind::Is {
                expr: Box::new(left),
                ty,
            }
            .spanned(self.finish_span(__start));
            left = if negate {
                ExprKind::Not(Box::new(is_expr)).spanned(self.finish_span(__start))
            } else {
                is_expr
            };
        }

        Ok(left)
    }

    fn parse_chained_comparison(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut terms = vec![self.parse_range()?];
        let mut ops = Vec::new();

        loop {
            let op = match self.peek() {
                Some(Token::Gt) => BinOp::Gt,
                Some(Token::Lt) => BinOp::Lt,
                Some(Token::Gte) => BinOp::Gte,
                Some(Token::Lte) => BinOp::Lte,
                Some(Token::EqEqEq) => BinOp::Eq,
                _ => break,
            };
            self.advance();
            terms.push(self.parse_range()?);
            ops.push(op);
        }

        match ops.len() {
            0 => Ok(terms.pop().expect("at least one term was parsed")),
            1 => {
                let op = ops.pop().expect("checked len == 1");
                let right = terms.pop().expect("two terms for one op");
                let left = terms.pop().expect("two terms for one op");
                Ok(ExprKind::BinOp {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                }
                .spanned(self.finish_span(__start)))
            }
            _ => Ok(ExprKind::ChainedComparison { terms, ops }.spanned(self.finish_span(__start))),
        }
    }

    fn parse_range(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let start = self.parse_addition()?;

        let inclusive = match self.peek() {
            Some(Token::DotDot) => false,
            Some(Token::DotDotEq) => true,
            _ => return Ok(start),
        };
        self.advance();
        let end = self.parse_addition()?;

        Ok(ExprKind::Range {
            start: Box::new(start),
            end: Box::new(end),
            inclusive,
        }
        .spanned(self.finish_span(__start)))
    }

    fn parse_addition(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut left = self.parse_multiplication()?;

        loop {
            let op = match self.peek() {
                Some(Token::Plus) => BinOp::Add,
                Some(Token::Minus) => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplication()?;
            left = ExprKind::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            }
            .spanned(self.finish_span(__start));
        }

        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let mut left = self.parse_power()?;

        loop {
            let op = match self.peek() {
                Some(Token::Star) => BinOp::Mul,
                Some(Token::Slash) => BinOp::Div,
                Some(Token::Percent) => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            left = ExprKind::BinOp {
                op,
                left: Box::new(left),
                right: Box::new(right),
            }
            .spanned(self.finish_span(__start));
        }

        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let left = self.parse_unary()?;
        if matches!(self.peek(), Some(Token::Power)) {
            self.advance();
            let right = self.parse_power()?;
            return Ok(ExprKind::BinOp {
                op: BinOp::Pow,
                left: Box::new(left),
                right: Box::new(right),
            }
            .spanned(self.finish_span(__start)));
        }
        Ok(left)
    }

    fn parse_collection_constructor(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let constructor = match self.advance() {
            Some(Token::Ident(name)) => name.clone(),
            other => return Err(format!("Expected `Set` or `Map`, found {}", found(&other))),
        };
        match constructor.as_str() {
            "Set" => {
                let type_args = if matches!(self.peek(), Some(Token::Lt)) {
                    self.parse_type_arguments()?
                } else {
                    Vec::new()
                };
                if type_args.len() > 1 {
                    return Err("`Set` expects at most one type argument".to_string());
                }
                let initial = self.parse_new_collection_arg("Set")?;
                Ok(ExprKind::NewSet {
                    ty: type_args.into_iter().next(),
                    initial,
                }
                .spanned(self.finish_span(__start)))
            }
            "Map" => {
                let type_args = if matches!(self.peek(), Some(Token::Lt)) {
                    self.parse_type_arguments()?
                } else {
                    Vec::new()
                };
                if type_args.len() == 1 || type_args.len() > 2 {
                    return Err("`Map` expects two type arguments `<K, V>`".to_string());
                }
                let initial = self.parse_new_collection_arg("Map")?;
                let mut type_args = type_args.into_iter();
                Ok(ExprKind::NewMap {
                    key: type_args.next(),
                    value: type_args.next(),
                    initial,
                }
                .spanned(self.finish_span(__start)))
            }
            other => Err(format!(
                "internal: `{other}` is not a collection constructor"
            )),
        }
    }

    fn parse_new_collection_arg(&mut self, name: &str) -> Result<Option<Box<Expr>>, String> {
        self.expect(&Token::LParen)?;
        let args = self.parse_args()?;
        if args.len() > 1 {
            return Err(format!(
                "new {name} expects at most one argument, but {} were provided",
                args.len()
            ));
        }
        Ok(args.into_iter().next().map(Box::new))
    }

    fn try_parse_slice_spec(&mut self) -> Option<(Option<Expr>, Option<Expr>, bool)> {
        let mark = self.mark();
        let start = if matches!(self.peek(), Some(Token::DotDot | Token::DotDotEq)) {
            None
        } else {
            match self.parse_addition() {
                Ok(expr) => Some(expr),
                Err(_) => {
                    self.reset(mark);
                    return None;
                }
            }
        };
        let inclusive = match self.peek() {
            Some(Token::DotDot) => false,
            Some(Token::DotDotEq) => true,
            _ => {
                self.reset(mark);
                return None;
            }
        };
        self.advance();
        let end = if matches!(self.peek(), Some(Token::RBracket)) {
            None
        } else {
            match self.parse_addition() {
                Ok(expr) => Some(expr),
                Err(_) => {
                    self.reset(mark);
                    return None;
                }
            }
        };
        if !matches!(self.peek(), Some(Token::RBracket)) {
            self.reset(mark);
            return None;
        }
        Some((start, end, inclusive))
    }

    /// Whether the `?` at the cursor propagates a failure rather than opening a
    /// ternary. Both are the same token. When the token after it cannot begin
    /// an expression, it cannot begin the ternary's branch either, so the `?` is
    /// a try: that settles `load()? * 2`, `load()? === 1` and `load()?;`. When it
    /// can, as `+` and `-` can, the branch is read speculatively: a ternary is a
    /// `?` whose branch is followed by `:`, so `load()? - 1` is a try and
    /// `c ? -1 : 2` is not. A speculation that fails to parse decides nothing
    /// and falls back to the ternary, the reading before this existed, so a real
    /// error, such as nesting past the recursion limit, is reported where it
    /// occurs instead of being turned into a try.
    ///
    /// Every decision is cached per token, because a branch holding another
    /// ambiguous `?` would otherwise be re-read once per enclosing speculation.
    /// Caching a failed speculation is sound: the real parse reaches the same
    /// `?` at the same recursion depth, so it would fail in the same place.
    fn question_is_try(&mut self) -> bool {
        match self.peek2() {
            None => return true,
            Some(next) if !Self::can_start_unary_expression(next) => return true,
            Some(_) => {}
        }
        let question = self.pos;
        if let Some(&decided) = self.question_decisions.get(&question) {
            return decided;
        }
        let mark = self.mark();
        let depth = self.depth;
        let omitted_terminators = self.omitted_terminators;
        let type_parameter_scopes = self.type_parameter_scopes.len();
        let decisions_before = self.question_decisions.clone();
        self.advance();
        let branch = self.parse_ternary_branch();
        let decision = branch
            .is_ok()
            .then(|| !matches!(self.peek(), Some(Token::Colon)));
        self.reset(mark);
        self.depth = depth;
        self.omitted_terminators = omitted_terminators;
        self.type_parameter_scopes.truncate(type_parameter_scopes);
        let is_try = decision.unwrap_or(false);
        if is_try {
            // The branch was read as if this `?` opened a ternary, and inner
            // decisions may have been revisited to find its `:`. It does not
            // open one, so those revisions were made for a reading that lost.
            self.question_decisions = decisions_before;
        }
        self.question_decisions.insert(question, is_try);
        is_try
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let __start = self.start_span();
        let stop_before_try = std::mem::take(&mut self.await_operand);
        let expr = self.parse_primary()?;
        self.parse_postfix_chain(expr, __start, stop_before_try)
    }

    fn parse_postfix_from(&mut self, base: Expr) -> Result<Expr, String> {
        let start = self.start_span();
        self.parse_postfix_from_at(base, start)
    }

    fn parse_postfix_from_at(&mut self, base: Expr, __start: Span) -> Result<Expr, String> {
        self.parse_postfix_chain(base, __start, false)
    }

    pub(crate) fn parse_postfix_chain(
        &mut self,
        base: Expr,
        __start: Span,
        stop_before_try: bool,
    ) -> Result<Expr, String> {
        let mut expr = base;

        loop {
            let question_is_try =
                matches!(self.peek(), Some(Token::Question)) && self.question_is_try();
            if question_is_try && stop_before_try {
                return Ok(expr);
            }
            match self.peek() {
                Some(Token::Dot) => {
                    self.advance();
                    let method_start = self.start_span();
                    let method = match self.advance() {
                        Some(Token::Ident(s)) => s.clone(),
                        Some(token) if keyword_as_name(token).is_some() => {
                            keyword_as_name(token).unwrap().to_string()
                        }
                        other => {
                            return Err(format!("Expected method name, found {}", found(&other)));
                        }
                    };
                    let method_span = self.finish_span(method_start);
                    let type_args = if matches!(self.peek(), Some(Token::Lt)) {
                        let mark = self.mark();
                        match self.parse_type_arguments() {
                            Ok(type_args) if matches!(self.peek(), Some(Token::LParen)) => {
                                type_args
                            }
                            _ => {
                                self.reset(mark);
                                Vec::new()
                            }
                        }
                    } else {
                        Vec::new()
                    };
                    if matches!(self.peek(), Some(Token::LParen)) {
                        self.advance();
                        let args = self.parse_args()?;
                        expr = ExprKind::MethodCall {
                            object: Box::new(expr),
                            method,
                            method_span,
                            type_args,
                            args,
                        }
                        .spanned(self.finish_span(__start));
                    } else if matches!(self.peek(), Some(Token::LBrace)) {
                        let argument = self.parse_primary()?;
                        expr = ExprKind::MethodCall {
                            object: Box::new(expr),
                            method,
                            method_span,
                            type_args,
                            args: vec![argument],
                        }
                        .spanned(self.finish_span(__start));
                    } else {
                        expr = ExprKind::FieldAccess {
                            object: Box::new(expr),
                            field: method,
                        }
                        .spanned(self.finish_span(__start));
                    }
                }
                Some(Token::OptChain) => {
                    self.advance();
                    let member = match self.advance() {
                        Some(Token::Ident(s)) => s.clone(),
                        Some(token) if keyword_as_name(token).is_some() => {
                            keyword_as_name(token).unwrap().to_string()
                        }
                        other => {
                            return Err(format!(
                                "Expected member name after ?., found {}",
                                found(&other)
                            ));
                        }
                    };
                    let type_args = if matches!(self.peek(), Some(Token::Lt)) {
                        let mark = self.mark();
                        match self.parse_type_arguments() {
                            Ok(type_args) if matches!(self.peek(), Some(Token::LParen)) => {
                                type_args
                            }
                            _ => {
                                self.reset(mark);
                                Vec::new()
                            }
                        }
                    } else {
                        Vec::new()
                    };
                    if matches!(self.peek(), Some(Token::LParen)) {
                        self.advance();
                        let args = self.parse_args()?;
                        expr = ExprKind::OptionalMethodCall {
                            object: Box::new(expr),
                            method: member,
                            type_args,
                            args,
                        }
                        .spanned(self.finish_span(__start));
                    } else {
                        expr = ExprKind::OptionalFieldAccess {
                            object: Box::new(expr),
                            field: member,
                        }
                        .spanned(self.finish_span(__start));
                    }
                }
                Some(Token::LBracket) => {
                    self.advance();
                    if matches!(self.peek(), Some(Token::RBracket)) {
                        self.advance();
                        let element_type = match expr.kind() {
                            ExprKind::TypeAssert { ty, .. } => ty.clone(),
                            ExprKind::Ident(name) => match name.as_str() {
                                "string" => Type::Str,
                                "boolean" => Type::Bool,
                                "i32" => Type::I32,
                                "i64" => Type::I64,
                                "isize" => Type::Isize,
                                "u32" => Type::U32,
                                "u64" => Type::U64,
                                "usize" => Type::Usize,
                                "f32" => Type::F32,
                                "f64" => Type::F64,
                                "decimal" => Type::Decimal,
                                _ => Type::Named(name.clone()),
                            },
                            _ => {
                                return Err(
                                    "an array response type must start with a type name".into()
                                );
                            }
                        };
                        expr = ExprKind::TypeAssert {
                            value: Box::new(expr),
                            ty: Type::Array(Box::new(element_type)),
                        }
                        .spanned(self.finish_span(__start));
                        continue;
                    }
                    if let Some((start, end, inclusive)) = self.try_parse_slice_spec() {
                        self.advance();
                        let args = match (start, end) {
                            (None, None) => Vec::new(),
                            (Some(start), None) => vec![start],
                            (None, Some(end)) => {
                                vec![
                                    Expr::synthetic(ExprKind::IntLit(0)),
                                    slice_end(end, inclusive),
                                ]
                            }
                            (Some(start), Some(end)) => vec![start, slice_end(end, inclusive)],
                        };
                        expr = ExprKind::MethodCall {
                            object: Box::new(expr),
                            method: "slice".to_string(),
                            method_span: self.finish_span(__start),
                            type_args: Vec::new(),
                            args,
                        }
                        .spanned(self.finish_span(__start));
                        continue;
                    }
                    let index = self.parse_expr()?;
                    self.expect(&Token::RBracket)?;
                    expr = ExprKind::IndexAccess {
                        object: Box::new(expr),
                        index: Box::new(index),
                    }
                    .spanned(self.finish_span(__start));
                }
                Some(Token::LParen) => {
                    self.advance();
                    let args = self.parse_args()?;
                    expr = ExprKind::CallValue {
                        callee: Box::new(expr),
                        type_args: Vec::new(),
                        args,
                    }
                    .spanned(self.finish_span(__start));
                }
                Some(Token::Question) if question_is_try => {
                    self.advance();
                    expr = ExprKind::Try(Box::new(expr)).spanned(self.finish_span(__start));
                }
                Some(Token::Bang) if !matches!(self.peek2(), Some(Token::In | Token::Is)) => {
                    self.advance();
                    expr = ExprKind::Unwrap(Box::new(expr)).spanned(self.finish_span(__start));
                }
                Some(Token::As) => {
                    self.advance();
                    if matches!(self.peek(), Some(Token::Const)) {
                        self.advance();
                        expr = ExprKind::ConstAssert(Box::new(expr))
                            .spanned(self.finish_span(__start));
                    } else {
                        let ty = self.parse_type()?;
                        expr = ExprKind::TypeAssert {
                            value: Box::new(expr),
                            ty,
                        }
                        .spanned(self.finish_span(__start));
                    }
                }
                _ => break,
            }
        }

        if matches!(self.peek(), Some(Token::PlusPlus)) {
            self.advance();
            expr = ExprKind::PostfixIncr(Box::new(expr)).spanned(self.finish_span(__start));
        } else if matches!(self.peek(), Some(Token::MinusMinus)) {
            self.advance();
            expr = ExprKind::PostfixDecr(Box::new(expr)).spanned(self.finish_span(__start));
        }

        Ok(expr)
    }

    fn split_template_format_spec(source: &str) -> (&str, Option<FormatSpec>) {
        let Some(colon) = source.rfind(':') else {
            return (source, None);
        };
        let (expr_part, spec_part) = (&source[..colon], &source[colon + 1..]);
        match Self::parse_format_spec(spec_part) {
            Some(spec) => (expr_part, Some(spec)),
            None => (source, None),
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, String> {
        Ok(self.parse_args_with_ranges()?.0)
    }

    fn parse_args_with_ranges(&mut self) -> Result<(Vec<Expr>, Vec<Range<usize>>), String> {
        let mut args = vec![];
        let mut argument_ranges = vec![];

        while !matches!(self.peek(), Some(Token::RParen) | None) {
            let argument_start = self.token_start(self.pos);
            if matches!(self.peek(), Some(Token::Ellipsis)) {
                let spread_start = self.start_span();
                self.advance();
                let spread_expr = self.parse_expr()?;
                args.push(self.spanned(ExprKind::Spread(Box::new(spread_expr)), spread_start));
            } else {
                args.push(self.parse_expr()?);
            }
            argument_ranges.push(argument_start..self.previous_token_end());
            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            }
        }

        self.expect(&Token::RParen)?;
        Ok((args, argument_ranges))
    }

    fn parse_fn_body_parts(&mut self) -> Result<ParsedFunctionParts, String> {
        let type_params = self.parse_type_params()?;
        self.type_parameter_scopes
            .push(type_params.iter().map(|p| p.name.clone()).collect());
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let ret_type = if matches!(self.peek(), Some(Token::Colon)) {
            self.advance();
            self.parse_annotation_type()?
        } else {
            Type::Infer
        };
        let throws = self.parse_throws_clause()?;
        let body = self.parse_block()?;
        self.type_parameter_scopes.pop();
        Ok((type_params, params, ret_type, throws, body))
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, String> {
        let mut params = vec![];

        while !matches!(self.peek(), Some(Token::RParen) | None) {
            let param_start = self.start_span();
            let decorators = self.parse_decorators()?;

            let rest = if matches!(self.peek(), Some(Token::Ellipsis)) {
                self.advance();
                true
            } else {
                false
            };

            let mutable = if !rest && matches!(self.peek(), Some(Token::Ident(s)) if s == "mut") {
                self.advance();
                true
            } else {
                false
            };

            let pattern = match self.peek() {
                Some(Token::LBrace) => {
                    self.advance();
                    let mut names = vec![];
                    while !matches!(self.peek(), Some(Token::RBrace) | None) {
                        names.push(self.expect_binding_ident(BindingKind::Parameter)?);
                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                        }
                    }
                    if !matches!(self.peek(), Some(Token::RBrace)) {
                        return Err("Expected `}}` after object destructure pattern".to_string());
                    }
                    self.advance();
                    ParamPattern::Object(names)
                }
                Some(Token::LBracket) => {
                    self.advance();
                    let mut names = vec![];
                    while !matches!(self.peek(), Some(Token::RBracket) | None) {
                        names.push(self.expect_binding_ident(BindingKind::Parameter)?);
                        if matches!(self.peek(), Some(Token::Comma)) {
                            self.advance();
                        }
                    }
                    if !matches!(self.peek(), Some(Token::RBracket)) {
                        return Err("Expected `]` after array destructure pattern".to_string());
                    }
                    self.advance();
                    ParamPattern::Array(names)
                }
                _ => {
                    let name = self.expect_binding_ident(BindingKind::Parameter)?;
                    ParamPattern::Name(name)
                }
            };

            let optional = if let ParamPattern::Name(_) = &pattern {
                if matches!(self.peek(), Some(Token::Question)) {
                    self.advance();
                    true
                } else {
                    false
                }
            } else {
                false
            };

            let ty = if matches!(self.peek(), Some(Token::Colon)) {
                self.advance();
                if matches!(self.peek(), Some(Token::RParen | Token::Comma)) {
                    Type::Infer
                } else {
                    self.parse_annotation_type()?
                }
            } else {
                Type::Infer
            };

            let default_value = if let ParamPattern::Name(_) = &pattern {
                if matches!(self.peek(), Some(Token::Eq)) {
                    self.advance();
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                }
            } else {
                None
            };

            params.push(Param {
                decorators,
                pattern,
                ty,
                mutable,
                optional,
                default_value,
                rest,
                span: self.finish_span(param_start),
            });

            if matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
            }
        }

        let mut found_rest = false;
        for param in &params {
            if param.rest {
                if found_rest {
                    return Err("Only one rest parameter is allowed".to_string());
                }
                found_rest = true;
            }
        }
        if let Some(last) = params.last()
            && params.iter().any(|p| p.rest)
            && !last.rest
        {
            return Err("Rest parameter must be last".to_string());
        }

        Ok(params)
    }
}

fn add_synthetic_timestamp_fields(
    model: &mut DbModelMetadata,
    fields: &mut Vec<Field>,
) -> Result<(), String> {
    if !model.timestamps {
        return Ok(());
    }
    for (name, column) in [
        (
            "createdAt",
            if model.snake_case {
                "created_at"
            } else {
                "createdAt"
            },
        ),
        (
            "updatedAt",
            if model.snake_case {
                "updated_at"
            } else {
                "updatedAt"
            },
        ),
    ] {
        if fields.iter().any(|field| field.name == name) {
            return Err(format!(
                "Model field `{name}` is generated automatically by `@model` timestamps; remove its declaration"
            ));
        }
        let ty = Type::Named("Instant".to_string());
        fields.push(Field {
            name: name.to_string(),
            ty: ty.clone(),
            readonly: false,
            optional: false,
            decorators: Vec::new(),
            span: model.span,
        });
        model.fields.push(DbModelFieldMetadata {
            field: name.to_string(),
            column: column.to_string(),
            ty,
            id: false,
            id_strategy: None,
            unique: false,
            default_value: None,
            foreign_key: None,
            span: model.span,
        });
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/parser.rs"]
mod tests;
