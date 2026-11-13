use std::borrow::Cow;
use std::fs;
use std::io::{BufRead, IsTerminal, Write};
use std::ops::Range;
#[cfg(unix)]
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{Event, KeyEvent};
use crossterm::terminal;
use nu_ansi_term::{Color, Style};
use reedline::{
    ColumnarMenu, Completer, CompletionResult, EditCommand, EditMode, Emacs, FileBackedHistory,
    Highlighter, History, KeyCode, KeyModifiers, MenuBuilder, Prompt, PromptEditMode,
    PromptHistorySearch, Reedline, ReedlineEvent, ReedlineMenu, ReedlineRawEvent, Signal, Span,
    StyledText, Suggestion, default_emacs_keybindings,
};
use tempfile::TempDir;

use tyrus_codegen::ReplJitSession;
use tyrus_core::analysis;
use tyrus_core::ast::Type;
use tyrus_core::keywords::{
    ACTIVE_KEYWORDS, LITERAL_KEYWORDS, PRIMITIVE_TYPES, is_reserved_future,
};
use tyrus_core::lexer::{Token, lex_spanned};
use tyrus_core::module_loader;
use tyrus_core::parser::{ParseError, Parser};

use crate::compile::ReplCompiler;
use crate::repl_transform;

const COMMANDS: &[&str] = &[
    "help", "clear", "doc", "load", "list", "type", "edit", "save", "restore", "delete", "reset",
    "exit", "quit", "q",
];

const CONTROL_KEYWORDS: &[&str] = &[
    "as", "async", "break", "case", "catch", "continue", "default", "do", "else", "export",
    "finally", "for", "from", "function", "if", "import", "in", "match", "of", "return", "switch",
    "throw", "throws", "try", "when", "while",
];

fn is_control_keyword(word: &str) -> bool {
    CONTROL_KEYWORDS.contains(&word)
}

const CONTROL_KEYWORD_COLOR: Color = Color::Rgb(0xF9, 0x26, 0x72);
const KEYWORD_COLOR: Color = Color::Rgb(0x66, 0xD9, 0xEF);
const STRING_COLOR: Color = Color::Rgb(0xE6, 0xDB, 0x74);
const REGEX_COLOR: Color = STRING_COLOR;
const NUMBER_COLOR: Color = Color::Rgb(0xAE, 0x81, 0xFF);
const FUNCTION_COLOR: Color = Color::Rgb(0xA6, 0xE2, 0x2E);
const IDENTIFIER_COLOR: Color = Color::Rgb(0xF8, 0xF8, 0xF2);
const OPERATOR_COLOR: Color = CONTROL_KEYWORD_COLOR;
const COMMENT_COLOR: Color = Color::Rgb(0x88, 0x84, 0x6F);
const BRACKET_COLORS: [Color; 3] = [
    Color::Rgb(0xFF, 0xD7, 0x00),
    Color::Rgb(0xDA, 0x70, 0xD6),
    Color::Rgb(0x17, 0x9F, 0xFF),
];
const INTERPOLATION_COLOR: Color = CONTROL_KEYWORD_COLOR;

fn bracket_color(depth: usize) -> Color {
    BRACKET_COLORS[depth.saturating_sub(1) % BRACKET_COLORS.len()]
}

struct TyrusHighlighter;

impl Highlighter for TyrusHighlighter {
    fn highlight(&self, line: &str, _cursor: usize) -> StyledText {
        if line.trim_start().starts_with(':') {
            return command_highlight(line);
        }
        match lex_spanned(line) {
            Ok(tokens) => lexed_highlight(line, &tokens),
            Err(_) => fallback_highlight(line),
        }
    }
}

fn command_highlight(line: &str) -> StyledText {
    let mut styled = StyledText::new();
    let leading = line.len() - line.trim_start().len();
    if leading > 0 {
        styled.push((Style::new(), line[..leading].to_string()));
    }
    let rest = &line[leading..];
    let name_len = rest[1..]
        .chars()
        .take_while(|character| character.is_alphanumeric() || *character == '_')
        .fold(0, |length, character| length + character.len_utf8());
    let word_len = 1 + name_len;
    styled.push((
        Style::new().fg(Color::Cyan).bold(),
        rest[..word_len].to_string(),
    ));
    if word_len < rest.len() {
        styled.push((Style::new(), rest[word_len..].to_string()));
    }
    styled
}

fn lexed_highlight(line: &str, tokens: &[(Token, Range<usize>)]) -> StyledText {
    let depths = bracket_depths(tokens);
    let mut styled = StyledText::new();
    let mut cursor = 0;
    for (index, (token, range)) in tokens.iter().enumerate() {
        if range.start > cursor {
            style_gap(line, cursor..range.start, &mut styled);
        }
        push_token(
            line,
            token,
            index,
            tokens,
            depths[index],
            range,
            &mut styled,
        );
        cursor = range.end;
    }
    if cursor < line.len() {
        style_gap(line, cursor..line.len(), &mut styled);
    }
    styled
}

fn bracket_depths(tokens: &[(Token, Range<usize>)]) -> Vec<usize> {
    let mut depths = vec![0; tokens.len()];
    let mut depth = 0usize;
    for (index, (token, _)) in tokens.iter().enumerate() {
        match token {
            Token::LParen | Token::LBracket | Token::LBrace => {
                depth += 1;
                depths[index] = depth;
            }
            Token::RParen | Token::RBracket | Token::RBrace => {
                depths[index] = depth.max(1);
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    depths
}

fn push_token(
    line: &str,
    token: &Token,
    index: usize,
    tokens: &[(Token, Range<usize>)],
    depth: usize,
    range: &Range<usize>,
    styled: &mut StyledText,
) {
    match token {
        Token::TemplateLit(_) => push_template(line, range, styled),
        Token::TypedUIntLit((_, ty))
        | Token::TypedIntLit((_, ty))
        | Token::TypedFloatLit((_, ty)) => {
            push_typed_numeric(line, range, numeric_type_suffix_len(ty), styled);
        }
        Token::DecimalLit(_) => push_typed_numeric(line, range, 1, styled),
        _ => styled.push((
            token_style(token, index, tokens, depth),
            line[range.clone()].to_string(),
        )),
    }
}

fn numeric_type_suffix_len(ty: &Type) -> usize {
    match ty {
        Type::U8 | Type::I8 => 2,
        Type::U16
        | Type::I16
        | Type::U32
        | Type::I32
        | Type::U64
        | Type::I64
        | Type::F32
        | Type::F64 => 3,
        Type::U128 | Type::I128 | Type::F128 => 4,
        Type::Usize | Type::Isize => 5,
        _ => 0,
    }
}

fn push_typed_numeric(
    line: &str,
    range: &Range<usize>,
    suffix_len: usize,
    styled: &mut StyledText,
) {
    let text = &line[range.clone()];
    let split = text.len() - suffix_len;
    styled.push((Style::new().fg(NUMBER_COLOR), text[..split].to_string()));
    if suffix_len > 0 {
        styled.push((Style::new().fg(KEYWORD_COLOR), text[split..].to_string()));
    }
}

fn push_template(line: &str, range: &Range<usize>, styled: &mut StyledText) {
    let text = &line[range.clone()];
    let bytes = text.as_bytes();
    let mut cursor = 0usize;
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] != b'$' || bytes[i + 1] != b'{' {
            i += 1;
            continue;
        }
        if cursor < i {
            styled.push((Style::new().fg(STRING_COLOR), text[cursor..i].to_string()));
        }
        styled.push((Style::new().fg(INTERPOLATION_COLOR), "${".to_string()));

        let expr_start = i + 2;
        let mut depth = 1usize;
        let mut j = expr_start;
        while j < bytes.len() && depth > 0 {
            match bytes[j] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                b'"' | b'\'' => {
                    let quote = bytes[j];
                    j += 1;
                    while j < bytes.len() && bytes[j] != quote {
                        j += if bytes[j] == b'\\' { 2 } else { 1 };
                    }
                }
                _ => {}
            }
            j += 1;
        }
        let closed = depth == 0;
        let expr_end = if closed { j - 1 } else { j };
        push_interpolation_body(&text[expr_start..expr_end], styled);
        if closed {
            styled.push((Style::new().fg(INTERPOLATION_COLOR), "}".to_string()));
        }
        cursor = j;
        i = j;
    }
    if cursor < text.len() {
        styled.push((Style::new().fg(STRING_COLOR), text[cursor..].to_string()));
    }
}

fn push_interpolation_body(body: &str, styled: &mut StyledText) {
    match lex_spanned(body) {
        Ok(tokens) => styled.buffer.extend(lexed_highlight(body, &tokens).buffer),
        Err(_) => styled.push((Style::new().fg(STRING_COLOR), body.to_string())),
    }
}

fn style_gap(line: &str, range: Range<usize>, styled: &mut StyledText) {
    let gap = &line[range];
    let mut start = 0;
    loop {
        let rest = &gap[start..];
        let line_comment = rest.find("//");
        let block_comment = rest.find("/*");
        let line_first = match (line_comment, block_comment) {
            (Some(line_start), Some(block_start)) => line_start < block_start,
            (Some(_), None) => true,
            (None, _) => false,
        };
        if line_first {
            let line_start = line_comment.expect("line_first implies a line comment");
            if line_start > 0 {
                styled.push((Style::new(), rest[..line_start].to_string()));
            }
            styled.push((comment_style(), rest[line_start..].to_string()));
            return;
        }
        if let Some(block_start) = block_comment {
            if block_start > 0 {
                styled.push((Style::new(), rest[..block_start].to_string()));
            }
            match rest[block_start + 2..].find("*/") {
                Some(close) => {
                    let end = block_start + 2 + close + 2;
                    styled.push((comment_style(), rest[block_start..end].to_string()));
                    start += end;
                    continue;
                }
                None => {
                    styled.push((comment_style(), rest[block_start..].to_string()));
                    return;
                }
            }
        }
        styled.push((Style::new(), rest.to_string()));
        return;
    }
}

fn comment_style() -> Style {
    Style::new().fg(COMMENT_COLOR)
}

fn token_style(
    token: &Token,
    index: usize,
    tokens: &[(Token, Range<usize>)],
    depth: usize,
) -> Style {
    match token {
        Token::True | Token::False | Token::Null => Style::new().fg(NUMBER_COLOR),
        Token::None => Style::new().fg(IDENTIFIER_COLOR),
        Token::IntLit(_)
        | Token::IntOverflowLit(_)
        | Token::FloatLit(_)
        | Token::SpecialFloatLit(_)
        | Token::FloatOverflowLit => Style::new().fg(NUMBER_COLOR),
        Token::DecimalLit(_)
        | Token::TypedUIntLit(_)
        | Token::TypedIntLit(_)
        | Token::TypedFloatLit(_) => Style::new().fg(NUMBER_COLOR),
        Token::StringLit(_) | Token::TemplateLit(_) => Style::new().fg(STRING_COLOR),
        Token::RegexLit(_) => Style::new().fg(REGEX_COLOR),
        Token::TyI8
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
        | Token::TyChar
        | Token::TyBool
        | Token::TyString
        | Token::TyVoid
        | Token::TyUnknown
        | Token::TyNever => Style::new().fg(KEYWORD_COLOR),
        Token::If
        | Token::Else
        | Token::Return
        | Token::While
        | Token::Loop
        | Token::For
        | Token::Of
        | Token::OnFail
        | Token::In
        | Token::Is
        | Token::Break
        | Token::Continue
        | Token::Do
        | Token::Switch
        | Token::Case
        | Token::Default
        | Token::Throw
        | Token::Throws
        | Token::Try
        | Token::Catch
        | Token::Finally
        | Token::Defer
        | Token::ErrDefer
        | Token::Match
        | Token::Import
        | Token::Export
        | Token::Protect
        | Token::From
        | Token::As
        | Token::Function
        | Token::Validator
        | Token::Comptime
        | Token::Async => Style::new().fg(CONTROL_KEYWORD_COLOR),
        Token::Const | Token::Let | Token::Interface | Token::Enum | Token::Type | Token::Arrow => {
            Style::new().fg(KEYWORD_COLOR)
        }
        Token::At | Token::Typeof | Token::Await | Token::Extends | Token::Readonly => {
            Style::new().fg(CONTROL_KEYWORD_COLOR)
        }
        Token::EqEqEq
        | Token::NotEqEq
        | Token::Eq
        | Token::PlusEq
        | Token::MinusEq
        | Token::StarEq
        | Token::SlashEq
        | Token::PercentEq
        | Token::PowerEq
        | Token::AndEq
        | Token::OrEq
        | Token::NullCoalesceEq
        | Token::AmpersandEq
        | Token::PipeEq
        | Token::CaretEq
        | Token::ShlEq
        | Token::ShrEq
        | Token::UShrEq
        | Token::PipeOperator
        | Token::Power
        | Token::PlusPlus
        | Token::MinusMinus
        | Token::Plus
        | Token::Minus
        | Token::Star
        | Token::Slash
        | Token::Percent
        | Token::Gte
        | Token::Lte
        | Token::Gt
        | Token::Lt
        | Token::And
        | Token::Or
        | Token::Bang
        | Token::Ampersand
        | Token::Pipe
        | Token::Caret
        | Token::Tilde
        | Token::Shl
        | Token::Shr
        | Token::UShr
        | Token::NullCoalesce
        | Token::OptChain
        | Token::Question
        | Token::Ellipsis
        | Token::DotDotEq
        | Token::DotDot => Style::new().fg(OPERATOR_COLOR),
        Token::LParen
        | Token::RParen
        | Token::LBrace
        | Token::RBrace
        | Token::LBracket
        | Token::RBracket => Style::new().fg(bracket_color(depth)),
        Token::Colon => Style::new().fg(CONTROL_KEYWORD_COLOR),
        Token::Dot | Token::Comma | Token::Semicolon | Token::AtSign => Style::new(),
        Token::Ident(name) => ident_style(name, index, tokens),
        Token::LineComment | Token::BlockComment | Token::Whitespace => Style::new(),
    }
}

fn ident_style(name: &str, index: usize, tokens: &[(Token, Range<usize>)]) -> Style {
    if is_control_keyword(name) {
        return Style::new().fg(CONTROL_KEYWORD_COLOR);
    }
    let previous_is_dot = matches!(tokens.get(index.wrapping_sub(1)), Some((Token::Dot, _)));
    if is_reserved_future(name) && !previous_is_dot {
        return Style::new().fg(CONTROL_KEYWORD_COLOR);
    }
    let is_builtin_member = previous_is_dot && matches!(name, "log" | "error" | "warn");
    let is_function_call = matches!(tokens.get(index + 1), Some((Token::LParen, _)));
    if is_function_call || is_builtin_member {
        Style::new().fg(FUNCTION_COLOR)
    } else {
        Style::new().fg(IDENTIFIER_COLOR)
    }
}

fn fallback_highlight(line: &str) -> StyledText {
    let mut styled = StyledText::new();
    let chars: Vec<char> = line.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut depth = 0usize;

    while i < len {
        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '/' {
            styled.push((comment_style(), line[i..].to_string()));
            break;
        }

        if i + 1 < len && chars[i] == '/' && chars[i + 1] == '*' {
            let start = i;
            i += 2;
            while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(len);
            styled.push((comment_style(), line[start..i].to_string()));
            continue;
        }

        if chars[i] == '"' || chars[i] == '\'' || chars[i] == '`' {
            let quote = chars[i];
            let start = i;
            i += 1;
            while i < len && chars[i] != quote {
                if chars[i] == '\\' {
                    i += 1;
                    if i < len {
                        i += 1;
                    }
                    continue;
                }
                i += 1;
            }
            if i < len {
                i += 1;
            }
            if quote == '`' {
                push_template(line, &(start..i), &mut styled);
            } else {
                styled.push((Style::new().fg(STRING_COLOR), line[start..i].to_string()));
            }
            continue;
        }

        if chars[i].is_ascii_digit() {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '.') {
                i += 1;
            }
            styled.push((Style::new().fg(NUMBER_COLOR), line[start..i].to_string()));
            continue;
        }

        if chars[i].is_alphabetic() || chars[i] == '_' {
            let start = i;
            while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word = &line[start..i];
            let style = if matches!(word, "and" | "or" | "not") {
                Style::new().fg(OPERATOR_COLOR)
            } else if is_control_keyword(word) {
                Style::new().fg(CONTROL_KEYWORD_COLOR)
            } else if PRIMITIVE_TYPES.contains(&word)
                || LITERAL_KEYWORDS.contains(&word)
                || ACTIVE_KEYWORDS.contains(&word)
                || is_reserved_future(word)
            {
                Style::new().fg(KEYWORD_COLOR)
            } else {
                Style::new()
            };
            styled.push((style, word.to_string()));
            continue;
        }

        let style = match chars[i] {
            '(' | '[' | '{' => {
                depth += 1;
                Style::new().fg(bracket_color(depth))
            }
            ')' | ']' | '}' => {
                let color = bracket_color(depth);
                depth = depth.saturating_sub(1);
                Style::new().fg(color)
            }
            '=' | '+' | '-' | '*' | '/' | '%' | '<' | '>' | '&' | '|' | '!' | '^' | '~' | '?' => {
                Style::new().fg(OPERATOR_COLOR)
            }
            _ => Style::new(),
        };
        styled.push((style, chars[i].to_string()));
        i += 1;
    }

    styled
}

struct ReplCompleter {
    buffer: Arc<Mutex<String>>,
}

impl ReplCompleter {
    fn new(buffer: Arc<Mutex<String>>) -> Self {
        Self { buffer }
    }

    fn buffer_analysis(&self) -> Option<analysis::DocumentAnalysis> {
        let locked = self.buffer.lock().ok()?;
        if locked.is_empty() {
            return None;
        }
        Some(analysis::analyze(&locked))
    }
}

impl ReplCompleter {
    pub(crate) fn suggestions_for(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let line_before = &line[..pos];
        let trimmed = line_before.trim_start();

        if let Some(cmd_line) = trimmed.strip_prefix(':') {
            let leading = line_before.len() - trimmed.len();
            let cmd_word = cmd_line.split_whitespace().next().unwrap_or("");
            let after_cmd = cmd_line.trim_start_matches(cmd_word).trim_start();

            if !cmd_word.is_empty() && !after_cmd.is_empty() {
                let file_cmds = ["load", "restore"];
                let session_cmds = ["restore", "delete"];
                if file_cmds.contains(&cmd_word) || session_cmds.contains(&cmd_word) {
                    let mut suggestions = Vec::new();
                    if file_cmds.contains(&cmd_word) {
                        suggestions.extend(complete_path(after_cmd, pos));
                    }
                    if session_cmds.contains(&cmd_word) {
                        suggestions.extend(complete_saved_sessions(
                            after_cmd,
                            pos,
                            &sessions_dir(),
                        ));
                    }
                    return suggestions;
                }
                return vec![];
            }

            let prefix = cmd_word;
            let span_start = leading;
            return COMMANDS
                .iter()
                .filter(|c| c.starts_with(prefix))
                .map(|c| Suggestion {
                    value: format!(":{c} "),
                    description: None,
                    style: None,
                    extra: None,
                    span: Span::new(span_start, pos),
                    append_whitespace: false,
                    display_override: None,
                    match_indices: None,
                })
                .collect();
        }

        if let Some((receiver, partial)) = member_context(trimmed) {
            let analysis = self.buffer_analysis();
            return member_completions(receiver, partial, analysis.as_ref(), pos);
        }

        let mut suggestions = code_completions(trimmed, pos);

        if trimmed.len() >= 2
            && let Ok(locked) = self.buffer.lock()
            && !locked.is_empty()
        {
            let analysis = analysis::analyze(&locked);
            add_symbol_completions(&mut suggestions, &analysis, trimmed, pos);
        }

        suggestions
    }
}

impl Completer for ReplCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> CompletionResult {
        CompletionResult::fresh(self.suggestions_for(line, pos))
    }
}

fn member_context(trimmed: &str) -> Option<(&str, &str)> {
    let bytes = trimmed.as_bytes();
    let mut end = bytes.len();
    while end > 0 {
        let character = bytes[end - 1] as char;
        if character.is_ascii_alphanumeric() || character == '_' {
            end -= 1;
        } else {
            break;
        }
    }
    let partial = &trimmed[end..];
    let before = trimmed[..end]
        .strip_suffix("?.")
        .or_else(|| trimmed[..end].strip_suffix('.'))?;
    if before.is_empty() {
        return None;
    }
    let start = before
        .rfind(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
        .map_or(0, |index| index + 1);
    let receiver = &before[start..];
    if receiver.is_empty() {
        return None;
    }
    Some((receiver, partial))
}

fn member_completions(
    receiver: &str,
    partial: &str,
    analysis: Option<&analysis::DocumentAnalysis>,
    pos: usize,
) -> Vec<Suggestion> {
    let span = Span::new(pos - partial.len(), pos);
    let mut suggestions = Vec::new();
    let push = |suggestions: &mut Vec<Suggestion>, value: String, description: String| {
        suggestions.push(Suggestion {
            value,
            description: Some(description),
            extra: None,
            style: None,
            span,
            append_whitespace: false,
            display_override: None,
            match_indices: None,
        });
    };

    if receiver == "console" {
        for member in tyrus_core::completion_common::CONSOLE_MEMBERS {
            if member.name.starts_with(partial) {
                push(
                    &mut suggestions,
                    format!("{}(", member.name),
                    member.description.to_string(),
                );
            }
        }
        return suggestions;
    }

    let Some(analysis) = analysis else {
        return suggestions;
    };
    let is_session_enum = analysis
        .symbols
        .iter()
        .any(|symbol| symbol.kind == analysis::SymbolKind::Enum && symbol.name == receiver);
    if is_session_enum {
        let enum_prefix = format!("({receiver}) ");
        for symbol in &analysis.symbols {
            if symbol.kind == analysis::SymbolKind::EnumMember
                && symbol.ty.starts_with(&enum_prefix)
                && symbol.name.starts_with(partial)
            {
                push(
                    &mut suggestions,
                    symbol.name.clone(),
                    format!("({receiver}) {}", symbol.name),
                );
            }
        }
        return suggestions;
    }

    if let Some(symbol) = analysis
        .symbols
        .iter()
        .find(|symbol| symbol.kind == analysis::SymbolKind::Import && symbol.name == receiver)
        && let (Some(source), Some(imported)) = (
            symbol.import_source.as_deref(),
            symbol.imported_name.as_deref(),
        )
        && let Some(resolved) = module_loader::native_export_enum(source, imported)
    {
        for (name, _) in resolved.members {
            if name.starts_with(partial) {
                push(
                    &mut suggestions,
                    name.clone(),
                    format!("({receiver}) {name}"),
                );
            }
        }
    }
    suggestions
}

fn add_symbol_completions(
    suggestions: &mut Vec<Suggestion>,
    analysis: &analysis::DocumentAnalysis,
    trimmed: &str,
    pos: usize,
) {
    let word_start = trimmed
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map_or(0, |i| i + 1);
    let partial = &trimmed[word_start..];
    if partial.is_empty() {
        return;
    }

    let word_abs_start = pos - trimmed.len() + word_start;

    for symbol in &analysis.symbols {
        if symbol.name.starts_with(partial)
            && !symbol.name.starts_with("__tyrus")
            && !suggestions.iter().any(|s| s.value == symbol.name)
        {
            let kind_str = match symbol.kind {
                analysis::SymbolKind::Function => "fn",
                analysis::SymbolKind::Variable => "var",
                analysis::SymbolKind::Parameter => "param",
                analysis::SymbolKind::Import => "import",
                analysis::SymbolKind::Interface => "interface",
                analysis::SymbolKind::TypeAlias => "type",
                analysis::SymbolKind::Property => "prop",
                analysis::SymbolKind::Enum => "enum",
                analysis::SymbolKind::EnumMember => "member",
            };
            suggestions.push(Suggestion {
                value: symbol.name.clone(),
                description: Some(format!("{kind_str} ({})", symbol.ty)),
                extra: None,
                style: None,
                span: Span::new(word_abs_start, pos),
                append_whitespace: false,
                display_override: None,
                match_indices: None,
            });
        }
    }
}

fn code_completions(trimmed: &str, pos: usize) -> Vec<Suggestion> {
    if trimmed.is_empty() {
        return vec![];
    }

    let word_start = trimmed
        .rfind(|c: char| !c.is_alphanumeric() && c != '_')
        .map_or(0, |i| i + 1);
    let partial = &trimmed[word_start..];
    if partial.is_empty() {
        return vec![];
    }

    let word_abs_start = pos - trimmed.len() + word_start;

    let mut suggestions: Vec<Suggestion> = ACTIVE_KEYWORDS
        .iter()
        .filter(|k| !PRIMITIVE_TYPES.contains(k) && !LITERAL_KEYWORDS.contains(k))
        .chain(PRIMITIVE_TYPES.iter())
        .chain(LITERAL_KEYWORDS.iter())
        .filter(|k| k.starts_with(partial))
        .map(|k| Suggestion {
            value: format!("{k} "),
            description: Some("keyword".to_string()),
            extra: None,
            style: None,
            span: Span::new(word_abs_start, pos),
            append_whitespace: false,
            display_override: None,
            match_indices: None,
        })
        .collect();

    for function in tyrus_core::completion_common::PRINT_FUNCTIONS {
        if function.name.starts_with(partial) {
            suggestions.push(Suggestion {
                value: format!("{}(", function.name),
                description: Some(function.description.to_string()),
                extra: None,
                style: None,
                span: Span::new(word_abs_start, pos),
                append_whitespace: false,
                display_override: None,
                match_indices: None,
            });
        }
    }

    if "console.log".starts_with(partial) {
        suggestions.push(Suggestion {
            value: "console.log(".to_string(),
            description: Some("write to stdout".to_string()),
            extra: None,
            style: None,
            span: Span::new(word_abs_start, pos),
            append_whitespace: false,
            display_override: None,
            match_indices: None,
        });
    }

    suggestions
}

fn complete_path(partial: &str, pos: usize) -> Vec<Suggestion> {
    let path_start = pos - partial.len();

    let (dir_part, file_prefix) = match std::path::Path::new(partial).parent() {
        Some(p) if !p.as_os_str().is_empty() => {
            let dir = p.to_string_lossy().to_string();
            let file = std::path::Path::new(partial)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            (dir, file)
        }
        _ => (".".to_string(), partial.to_string()),
    };

    let entries = match fs::read_dir(&dir_part) {
        Ok(entries) => entries,
        Err(_) => return vec![],
    };

    entries
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(&file_prefix))
        })
        .map(|p| {
            let display = p.to_string_lossy().to_string();
            let is_dir = p.is_dir();
            let value = if is_dir {
                format!("{display}/")
            } else {
                display
            };
            Suggestion {
                value,
                description: if is_dir {
                    Some("directory".to_string())
                } else {
                    None
                },
                style: None,
                extra: None,
                span: Span::new(path_start, pos),
                append_whitespace: !is_dir,
                display_override: None,
                match_indices: None,
            }
        })
        .collect()
}

fn complete_saved_sessions(partial: &str, pos: usize, dir: &Path) -> Vec<Suggestion> {
    let span_start = pos - partial.len();
    let Ok(entries) = fs::read_dir(dir) else {
        return vec![];
    };
    entries
        .filter_map(std::result::Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "tyr"))
        .filter_map(|path| {
            let name = path.file_name()?.to_string_lossy().into_owned();
            name.starts_with(partial).then_some((name, path))
        })
        .map(|(name, path)| Suggestion {
            value: name,
            description: Some(path.display().to_string()),
            style: None,
            extra: None,
            span: Span::new(span_start, pos),
            append_whitespace: false,
            display_override: None,
            match_indices: None,
        })
        .collect()
}

const PROMPT_MAIN: &str = "tyrus> ";
const PROMPT_CONT: &str = ".... ";
const HISTORY_SIZE: usize = 5000;

struct ReplPrompt {
    continuation: bool,
}

#[cfg(unix)]
struct ReplTerminalMode {
    fd: std::os::fd::RawFd,
    original: libc::termios,
}

#[cfg(unix)]
impl ReplTerminalMode {
    fn enter() -> std::io::Result<Self> {
        let fd = std::io::stdin().as_raw_fd();
        let mut original = std::mem::MaybeUninit::<libc::termios>::uninit();
        if unsafe { libc::tcgetattr(fd, original.as_mut_ptr()) } == -1 {
            return Err(std::io::Error::last_os_error());
        }
        let original = unsafe { original.assume_init() };
        let active = raw_input_attributes(original);
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &active) } == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self { fd, original })
    }
}

#[cfg(unix)]
fn raw_input_attributes(original: libc::termios) -> libc::termios {
    let mut active = original;
    unsafe { libc::cfmakeraw(&mut active) };
    active.c_iflag |= libc::INLCR;
    active.c_oflag = original.c_oflag;
    active
}

#[cfg(unix)]
impl Drop for ReplTerminalMode {
    fn drop(&mut self) {
        let _ = unsafe { libc::tcsetattr(self.fd, libc::TCSANOW, &self.original) };
    }
}

#[cfg(not(unix))]
struct ReplTerminalMode;

#[cfg(not(unix))]
impl ReplTerminalMode {
    fn enter() -> std::io::Result<Self> {
        Ok(Self)
    }
}

fn read_repl_line(editor: &mut Reedline, prompt: &dyn Prompt) -> std::io::Result<Signal> {
    terminal::enable_raw_mode()?;
    if let Err(error) = map_line_feed_to_carriage_return() {
        let _ = terminal::disable_raw_mode();
        return Err(error);
    }
    editor.read_line(prompt)
}

#[cfg(unix)]
fn map_line_feed_to_carriage_return() -> std::io::Result<()> {
    let fd = std::io::stdin().as_raw_fd();
    let mut attributes = std::mem::MaybeUninit::<libc::termios>::uninit();
    if unsafe { libc::tcgetattr(fd, attributes.as_mut_ptr()) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    let mut attributes = unsafe { attributes.assume_init() };
    attributes.c_iflag |= libc::INLCR;
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &attributes) } == -1 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(not(unix))]
fn map_line_feed_to_carriage_return() -> std::io::Result<()> {
    Ok(())
}

impl Prompt for ReplPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        if self.continuation {
            PROMPT_CONT.into()
        } else {
            PROMPT_MAIN.into()
        }
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_history_search_indicator(&self, _: PromptHistorySearch) -> Cow<'_, str> {
        Cow::Borrowed("")
    }
}

#[derive(Debug)]
enum InputStatus {
    Ready,
    Incomplete,
    Error(String),
}

fn eof_error(error: &ParseError) -> bool {
    error.at_end_of_input
}

fn input_status(source: &str) -> InputStatus {
    if parses(source) || parses(&terminated_input(source)) {
        return InputStatus::Ready;
    }
    let Ok(mut parser) = Parser::from_source(source) else {
        return InputStatus::Incomplete;
    };
    match parser.parse_program() {
        Ok(_) => InputStatus::Ready,
        Err(error) if eof_error(&error) => InputStatus::Incomplete,
        Err(error) => InputStatus::Error(error.message),
    }
}

fn parses(source: &str) -> bool {
    Parser::from_source(source).is_ok_and(|mut parser| parser.parse_program().is_ok())
}

/// Typing an expression on its own, `x`, is how a REPL is used, so the prompt
/// supplies the `;` the language requires instead of refusing the line or waiting
/// for more input. A line that already ends in `;` or closes a block is kept as
/// written, and so is one that is still open, which has to stay incomplete.
fn terminated_input(source: &str) -> String {
    let trimmed = source.trim_end();
    if trimmed.is_empty()
        || trimmed.ends_with(';')
        || trimmed.ends_with('}')
        || trimmed.ends_with('{')
    {
        return source.to_string();
    }
    format!("{trimmed};")
}

fn history_path() -> PathBuf {
    directories::BaseDirs::new().map_or_else(
        || PathBuf::from(".tyrus_repl_history"),
        |d| d.home_dir().join(".tyrus_repl_history"),
    )
}

fn sessions_dir() -> PathBuf {
    directories::BaseDirs::new().map_or_else(
        || PathBuf::from(".tyrus/repl"),
        |d| d.home_dir().join(".tyrus").join("repl"),
    )
}

struct SavedSession {
    name: String,
    path: PathBuf,
    lines: usize,
    bytes: usize,
    modified: u64,
}

fn session_dest(dir: &Path, name: &str) -> Result<PathBuf, String> {
    if !name.ends_with(".tyr") {
        return Err(":save requires a `.tyr` extension (e.g. `:save session.tyr`)".to_string());
    }
    if name.contains('/') || name.contains('\\') {
        return Err(format!(
            ":save stores sessions in `{}`; use a bare name like `:save session.tyr`",
            dir.display()
        ));
    }
    Ok(dir.join(name))
}

fn session_file(dir: &Path, arg: &str) -> PathBuf {
    let name = if arg.ends_with(".tyr") {
        arg.to_string()
    } else {
        format!("{arg}.tyr")
    };
    dir.join(name)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn relative_time(now: u64, epoch_secs: u64) -> String {
    let delta = now.saturating_sub(epoch_secs);
    if delta < 60 {
        "just now".to_string()
    } else if delta < 3600 {
        format!("{}m ago", delta / 60)
    } else if delta < 86_400 {
        format!("{}h ago", delta / 3600)
    } else {
        format!("{}d ago", delta / 86_400)
    }
}

fn display_sessions(dir: &Path) -> String {
    let mut sessions: Vec<SavedSession> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "tyr"))
            .filter_map(|path| {
                let metadata = fs::metadata(&path).ok()?;
                let file = fs::File::open(&path).ok()?;
                let lines = std::io::BufReader::new(file).lines().count();
                let modified = metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs());
                Some(SavedSession {
                    name: path.file_name()?.to_string_lossy().into_owned(),
                    path,
                    lines,
                    bytes: metadata.len() as usize,
                    modified,
                })
            })
            .collect(),
        Err(_) => vec![],
    };
    sessions.sort_by_key(|s| s.modified);
    sessions.reverse();
    if sessions.is_empty() {
        return "No saved sessions yet. Use :save <name.tyr> to save the session buffer."
            .to_string();
    }
    let mut out = String::from("Saved sessions:\n");
    for session in sessions {
        out.push_str(&format!(
            "  {:<16} {:>5} lines {:>6} B  {:<10} {}\n",
            session.name,
            session.lines,
            session.bytes,
            relative_time(now_secs(), session.modified),
            session.path.display(),
        ));
    }
    out
}

fn declared_names(source: &str) -> std::collections::HashSet<String> {
    analysis::analyze(source)
        .symbols
        .into_iter()
        .filter(|symbol| {
            matches!(
                symbol.kind,
                analysis::SymbolKind::Variable
                    | analysis::SymbolKind::Function
                    | analysis::SymbolKind::Interface
                    | analysis::SymbolKind::TypeAlias
                    | analysis::SymbolKind::Enum
            )
        })
        .map(|symbol| symbol.name)
        .collect()
}

fn wrap_buffer(buffer: &str) -> String {
    let mut code = String::new();
    code.push_str("const main = (): void => {\n");
    for line in buffer.lines() {
        code.push_str("    ");
        code.push_str(line);
        code.push('\n');
    }
    code.push_str("};\n");
    code
}

struct Session {
    buffer: String,
    buffer_arc: Arc<Mutex<String>>,
    temp_dir: TempDir,
    temp_path: PathBuf,
    compiler: ReplCompiler,
    units: Vec<String>,
    sessions_dir: PathBuf,
    repl: Option<ReplJitSession>,
}

struct Snapshot {
    buffer: String,
    units: Vec<String>,
}

impl Session {
    fn new() -> Self {
        let temp_dir = TempDir::new().expect("failed to create temp directory");
        let temp_path = temp_dir.path().to_path_buf();
        let buffer_arc = Arc::new(Mutex::new(String::new()));
        Self {
            buffer: String::new(),
            buffer_arc,
            temp_dir,
            temp_path,
            compiler: ReplCompiler::new(),
            units: Vec::new(),
            sessions_dir: sessions_dir(),
            repl: ReplJitSession::new().ok(),
        }
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            buffer: self.buffer.clone(),
            units: self.units.clone(),
        }
    }

    fn restore_snapshot(&mut self, snapshot: Snapshot) {
        self.buffer = snapshot.buffer;
        self.units = snapshot.units;
        self.sync_buffer();
    }

    fn sync_buffer(&self) {
        if let Ok(mut locked) = self.buffer_arc.lock() {
            (*locked).clone_from(&self.buffer);
        }
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.units.clear();
        self.sync_buffer();
        self.temp_dir = TempDir::new().expect("failed to create temp directory");
        self.temp_path = self.temp_dir.path().to_path_buf();
        self.repl = ReplJitSession::new().ok();
    }

    fn write_entry(&self, source: &str) -> Result<PathBuf, String> {
        let src_dir = self.temp_path.join("src");
        fs::create_dir_all(&src_dir).map_err(|e| format!("failed to create src/: {e}"))?;
        let entry = src_dir.join("main.tyr");
        fs::write(&entry, source).map_err(|e| format!("failed to write main.tyr: {e}"))?;
        Ok(entry)
    }

    fn compile_incremental(&mut self) -> Result<String, String> {
        let transform = repl_transform::transform_units(&self.units, |_name, initializer| {
            self.cmd_type(initializer).ok()
        });
        let entry = self.write_entry(&transform.source)?;
        let program =
            module_loader::load_program(&entry, None).map_err(|e| format!("error: {e}"))?;
        let repl = self
            .repl
            .as_mut()
            .ok_or("internal: incremental JIT session is missing")?;
        if transform.last_has_step {
            let step_symbol = repl_transform::step_symbol(self.units.len());
            repl.add_and_run(&program, "__repl__", &step_symbol)
        } else {
            repl.add_module(&program, "__repl__")?;
            Ok(String::new())
        }
    }

    fn compile_whole_buffer(&self) -> Result<String, String> {
        let source = wrap_buffer(&self.buffer);
        self.compiler.compile_and_run(&source, &self.temp_path)
    }

    fn reset_jit(&mut self) {
        self.repl = ReplJitSession::new().ok();
    }

    fn eval(&mut self, code: &str) -> Result<(), String> {
        let saved = self.snapshot();
        // The prompt accepts a bare expression, so the unit is stored with the
        // terminator the language requires; the saved session then holds code
        // that compiles on its own.
        let code = if parses(code) {
            code.to_string()
        } else {
            terminated_input(code)
        };
        let code = code.as_str();
        self.buffer.push_str(code);
        self.buffer.push('\n');
        self.units.push(code.to_string());
        self.sync_buffer();

        let result = if self.repl.is_some() {
            self.compile_incremental()
        } else {
            self.compile_whole_buffer()
        };

        match result {
            Ok(stdout) => {
                if !stdout.is_empty() {
                    print!("{stdout}");
                }
                Ok(())
            }
            Err(stderr) => {
                self.restore_snapshot(saved);
                if stderr.starts_with("error[jit]") {
                    self.reset_jit();
                }
                Err(stderr)
            }
        }
    }

    fn cmd_doc(&self, symbol: &str) {
        let analysis = analysis::analyze(&self.buffer);
        if let Some(sym) = analysis.symbol_named(symbol) {
            println!("symbol: {}", sym.name);
            println!("type:   {}", sym.ty);
            if !sym.parameters.is_empty() {
                let params: Vec<String> = sym
                    .parameters
                    .iter()
                    .map(|p| {
                        let name = match &p.pattern {
                            tyrus_core::ast::ParamPattern::Name(n) => n.clone(),
                            _ => "[destructured]".to_string(),
                        };
                        format!("{}: {}", name, p.ty)
                    })
                    .collect();
                println!("params: {}", params.join(", "));
            }
            if let Some(doc) = &sym.documentation {
                if !doc.summary.is_empty() {
                    println!("\n{}", doc.summary);
                }
                if !doc.description.is_empty() && doc.description != doc.summary {
                    println!("{}", doc.description);
                }
                for (name, desc) in &doc.params {
                    println!("  @param {name}, {desc}");
                }
                if let Some(ret) = &doc.returns {
                    println!("  @returns {ret}");
                }
                if let Some(msg) = &doc.deprecated {
                    println!("  @deprecated {msg}");
                }
                for example in &doc.examples {
                    println!("\n```\n{example}\n```");
                }
            }
        } else {
            eprintln!("error: symbol `{symbol}` not found");
        }
    }

    fn cmd_type(&self, expr: &str) -> Result<String, String> {
        let mut source = self.buffer.clone();
        source.push('\n');
        source.push_str(&format!("const __tyrus_type_result = {expr};\n"));
        let analysis = analysis::analyze(&source);
        if let Some(sym) = analysis.symbol_named("__tyrus_type_result")
            && sym.ty != "unknown"
        {
            return Ok(sym.ty.clone());
        }
        let mut msgs = Vec::new();
        let expr_offset = self.buffer.len() + 1;
        for d in &analysis.diagnostics {
            if d.range.start >= expr_offset {
                msgs.push(d.message.clone());
            }
        }
        if msgs.is_empty() {
            Err("error: could not determine type of expression".to_string())
        } else {
            Err(format!("error: {}", msgs.join("\nerror: ")))
        }
    }

    fn save(&self, name: &str) {
        let dest = match session_dest(&self.sessions_dir, name) {
            Ok(dest) => dest,
            Err(message) => {
                eprintln!("error: {message}");
                return;
            }
        };
        if let Err(e) = fs::create_dir_all(&self.sessions_dir) {
            eprintln!(
                "error: failed to create `{}`: {e}",
                self.sessions_dir.display()
            );
            return;
        }
        match fs::write(&dest, &self.buffer) {
            Ok(()) => println!(
                "Session saved to `{}` ({} bytes).",
                dest.display(),
                self.buffer.len()
            ),
            Err(e) => eprintln!("error: failed to save to `{}`: {e}", dest.display()),
        }
    }

    fn list(&self) -> String {
        display_sessions(&self.sessions_dir)
    }

    fn delete(&self, name: &str) {
        if name.contains('/') || name.contains('\\') {
            eprintln!(
                "error: :delete only removes sessions from `{}`; use a bare name like `:delete session.tyr`",
                self.sessions_dir.display()
            );
            return;
        }
        let path = session_file(&self.sessions_dir, name);
        let file_name = path
            .file_name()
            .map_or_else(|| name.to_string(), |n| n.to_string_lossy().into_owned());
        match fs::remove_file(&path) {
            Ok(()) => println!("File {file_name} deleted successfully."),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("error: File {file_name} not found.\nRun :list to see saved sessions.");
            }
            Err(e) => eprintln!("error: failed to delete `{}`: {e}", path.display()),
        }
    }

    fn restore(&mut self, arg: &str, merge: bool) {
        let path = if fs::metadata(arg).is_ok() {
            Cow::Borrowed(arg)
        } else {
            Cow::Owned(session_file(&self.sessions_dir, arg).display().to_string())
        };

        let content = match fs::read_to_string(&*path) {
            Ok(c) => c,
            Err(e) => {
                let hint = if arg.contains('/') || arg.contains('\\') {
                    ""
                } else {
                    "\nRun :list to see saved sessions."
                };
                eprintln!("error: cannot read `{arg}`: {e}{hint}");
                return;
            }
        };

        if merge {
            let mut conflicts: Vec<String> = declared_names(&self.buffer)
                .intersection(&declared_names(&content))
                .cloned()
                .collect();
            if !conflicts.is_empty() {
                conflicts.sort();
                eprintln!(
                    "error: cannot merge `{path}`: already defined in the current session: {}\nUse :reset to clear the current session, or drop --merge to reset before restoring.",
                    conflicts.join(", ")
                );
                return;
            }
        } else {
            self.reset();
        }

        let saved = self.snapshot();
        let line_count = content.lines().count();
        eprintln!("Restored {line_count} lines from `{path}`:");
        self.replay_source(path.as_ref(), &content, saved);
    }

    fn replay_source(&mut self, path: &str, content: &str, saved: Snapshot) {
        let mut pending = String::new();
        let mut pending_lines: Vec<&str> = Vec::new();
        let mut failed = None;
        for line in content.lines() {
            pending.push_str(line);
            pending.push('\n');
            pending_lines.push(line);
            match input_status(&pending) {
                InputStatus::Ready => {
                    if !self.replay_unit(pending.trim_end(), &pending_lines) {
                        failed = Some(format!("`{path}` contains invalid tyrus code"));
                        break;
                    }
                    pending.clear();
                    pending_lines.clear();
                }
                InputStatus::Incomplete => {}
                InputStatus::Error(message) => {
                    failed = Some(message);
                    break;
                }
            }
        }
        if failed.is_none() && !pending_lines.is_empty() {
            failed = Some(format!("`{path}` ends with incomplete code"));
        }
        if let Some(message) = failed {
            self.restore_snapshot(saved);
            self.reset_jit();
            eprintln!("error: {message}; session unchanged.");
        }
    }

    fn replay_unit(&mut self, code: &str, lines: &[&str]) -> bool {
        for (index, line) in lines.iter().enumerate() {
            let prompt = if index == 0 { PROMPT_MAIN } else { PROMPT_CONT };
            let styled = TyrusHighlighter.highlight(line, line.len()).render_simple();
            eprintln!("{prompt}{styled}");
        }
        match self.eval(code) {
            Ok(()) => true,
            Err(stderr) => {
                eprint!("{stderr}");
                false
            }
        }
    }

    fn edit(&mut self) {
        if self.buffer.is_empty() {
            eprintln!("error: nothing to edit (buffer is empty)");
            return;
        }
        let editor = std::env::var("EDITOR")
            .or_else(|_| std::env::var("VISUAL"))
            .unwrap_or_else(|_| "nano".to_string());

        let edit_path = self.temp_path.join("_repl_edit.tyr");
        if fs::write(&edit_path, &self.buffer).is_err() {
            eprintln!("error: failed to write temporary file for editing");
            return;
        }

        let launched = process::Command::new(&editor).arg(&edit_path).status();

        match launched {
            Ok(_) => {
                let new_content = match fs::read_to_string(&edit_path) {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("error: could not read edited file: {e}");
                        return;
                    }
                };
                if new_content == self.buffer {
                    println!("(unchanged)");
                    return;
                }
                let saved = self.snapshot();
                self.buffer = new_content;
                self.units = vec![self.buffer.clone()];
                self.sync_buffer();
                self.reset_jit();
                match self.compile_incremental() {
                    Ok(stdout) => {
                        println!("Buffer updated from editor.");
                        if !stdout.is_empty() {
                            print!("{stdout}");
                        }
                    }
                    Err(stderr) => {
                        self.restore_snapshot(saved);
                        self.reset_jit();
                        eprint!("{stderr}");
                        eprintln!("error: edited code has errors; session unchanged.");
                    }
                }
            }
            Err(e) => eprintln!("error: failed to launch editor `{editor}`: {e}"),
        }
    }

    fn load_file(&mut self, path: &str) {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: file `{path}` not found: {e}");
                return;
            }
        };

        let saved = self.snapshot();
        let line_count = content.lines().count();
        eprintln!("Loaded {line_count} lines from `{path}`:");
        self.replay_source(path, &content, saved);
    }
}

fn help_lines() -> Vec<String> {
    let commands = [
        (":help", "Show this help"),
        (":clear", "Clear screen and session buffer"),
        (":load <file>", "Load and evaluate a .tyr file"),
        (":type <expr>", "Show the inferred type of an expression"),
        (":doc <symbol>", "Show documentation for a symbol"),
        (":edit", "Edit session buffer in external editor ($EDITOR)"),
        (
            ":save <name.tyr>",
            "Save session buffer to ~/.tyrus/repl/<name>.tyr",
        ),
        (
            ":restore <name|file> [--merge|-m]",
            "Restore a saved session (resets first; --merge keeps the current buffer)",
        ),
        (
            ":delete <name>",
            "Delete a saved session from ~/.tyrus/repl",
        ),
        (":list", "List sessions saved via :save (~/.tyrus/repl)"),
        (":reset", "Reset the entire session"),
        (":exit", "Exit the REPL"),
        (":quit", "Alias for :exit"),
        (":q", "Alias for :exit"),
    ];
    let width = commands.iter().map(|(cmd, _)| cmd.len()).max().unwrap_or(0);
    commands
        .iter()
        .map(|(cmd, desc)| format!("  {cmd:<width$}  {desc}"))
        .collect()
}

fn cmd_help() {
    println!("Commands (prefix with :):");
    for line in help_lines() {
        println!("{line}");
    }
    println!();
    println!("Multi-line input is auto-detected by bracket matching.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn test_type_resolver(units: &[String]) -> impl FnMut(&str, &str) -> Option<String> + '_ {
        let analysis = analysis::analyze(&units.join("\n"));
        move |name, _initializer| {
            analysis
                .symbols
                .iter()
                .find(|symbol| symbol.name == name)
                .filter(|symbol| symbol.ty != "unknown")
                .map(|symbol| symbol.ty.clone())
        }
    }

    fn make_completer() -> ReplCompleter {
        ReplCompleter::new(Arc::new(Mutex::new(String::new())))
    }

    #[test]
    fn banner_text_renders_a_multiline_figlet_art() {
        let text = banner_text();
        assert!(
            text.lines().count() > 1,
            "expected multi-line FIGlet art, got: {text:?}"
        );
        assert!(text.chars().all(|character| character != '\t'));
    }

    #[cfg(unix)]
    #[test]
    fn raw_input_attributes_preserve_terminal_output_processing() {
        let mut original: libc::termios = unsafe { std::mem::zeroed() };
        original.c_oflag = libc::OPOST;

        let active = raw_input_attributes(original);

        assert_eq!(active.c_oflag, original.c_oflag);
        assert_ne!(active.c_iflag & libc::INLCR, 0);
    }

    #[test]
    fn completer_offers_all_commands_on_colon() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for(":", 1);
        assert_eq!(suggestions.len(), COMMANDS.len());
        for cmd in COMMANDS {
            assert!(
                suggestions.iter().any(|s| s.value == format!(":{cmd} ")),
                "missing suggestion for :{cmd}",
            );
        }
    }

    #[test]
    fn completer_filters_by_prefix() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for(":sa", 3);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].value, ":save ");
    }

    #[test]
    fn completer_offers_doc() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for(":do", 3);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].value, ":doc ");
    }

    #[test]
    fn completer_offers_help() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for(":he", 3);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].value, ":help ");
    }

    #[test]
    fn completer_no_match_returns_empty() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for(":zzz", 4);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn completer_span_covers_colon_prefix() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for("  :he", 5);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].span.start, 2);
        assert_eq!(suggestions[0].span.end, 5);
    }

    #[test]
    fn completer_suggests_keywords() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for("con", 3);
        assert!(
            !suggestions.is_empty(),
            "expected keyword completions for `con`"
        );
        assert!(
            suggestions.iter().any(|s| s.value.starts_with("const")),
            "expected `const` in results"
        );
    }

    #[test]
    fn completer_suggests_primitives() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for("st", 2);
        assert!(
            suggestions.iter().any(|s| s.value.starts_with("string")),
            "expected `string` in results"
        );
    }

    #[test]
    fn completer_suggests_print_builtins() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for("pri", 3);
        let values: Vec<&str> = suggestions
            .iter()
            .map(|suggestion| suggestion.value.as_str())
            .collect();
        assert_eq!(
            values,
            vec![
                "print(",
                "printError(",
                "printWarn(",
                "printRaw(",
                "printCR("
            ]
        );
    }

    #[test]
    fn completer_suggests_console_members_after_dot() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for("console.l", 9);
        assert_eq!(suggestions.len(), 1, "got: {suggestions:?}");
        assert_eq!(suggestions[0].value, "log(");
        assert_eq!(
            suggestions[0].description.as_deref(),
            Some("write to stdout")
        );
        assert_eq!(suggestions[0].span.start, 8);
        assert_eq!(suggestions[0].span.end, 9);
    }

    #[test]
    fn completer_suggests_all_console_members_after_dot() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for("console.", 8);
        let values: Vec<&str> = suggestions.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, vec!["log(", "error(", "warn("]);
    }

    #[test]
    fn completer_suggests_console_members_with_optional_chain() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for("console?.e", 10);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].value, "error(");
    }

    #[test]
    fn completer_does_not_offer_keywords_inside_member_context() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for("console.l", 9);
        assert!(
            suggestions.iter().all(|s| s.value != "let "),
            "keyword `let` leaked into member completion: {suggestions:?}"
        );
    }

    #[test]
    fn completer_suggests_enum_members_from_buffer() {
        let buf = Arc::new(Mutex::new(
            "enum Status { Ok, NotFound, InternalError }\n".to_string(),
        ));
        let mut c = ReplCompleter::new(buf);
        let suggestions = c.suggestions_for("Status.N", 8);
        assert_eq!(suggestions.len(), 1, "got: {suggestions:?}");
        assert_eq!(suggestions[0].value, "NotFound");
        assert_eq!(
            suggestions[0].description.as_deref(),
            Some("(Status) NotFound")
        );
        let suggestions = c.suggestions_for("Status.", 7);
        let values: Vec<&str> = suggestions.iter().map(|s| s.value.as_str()).collect();
        assert_eq!(values, vec!["Ok", "NotFound", "InternalError"]);
    }

    #[test]
    fn completer_suggests_native_enum_members_from_import() {
        let buf = Arc::new(Mutex::new(
            "import { HttpStatus } from \"tyrus:http\";\n".to_string(),
        ));
        let mut c = ReplCompleter::new(buf);
        let suggestions = c.suggestions_for("HttpStatus.N", 12);
        assert!(
            suggestions.iter().any(|s| s.value == "NotFound"),
            "expected native enum member `NotFound`, got: {suggestions:?}"
        );
    }

    #[test]
    fn completer_unknown_receiver_offers_no_members() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for("unknownThing.", 13);
        assert!(suggestions.is_empty());
    }

    #[test]
    fn completer_suggests_buffer_symbols() {
        let buf = Arc::new(Mutex::new("const myVar: i32 = 42;\n".to_string()));
        let mut c = ReplCompleter::new(buf);
        let suggestions = c.suggestions_for("my", 2);
        assert!(
            suggestions.iter().any(|s| s.value == "myVar"),
            "expected `myVar` in symbol completions, got: {suggestions:?}",
        );
    }

    #[test]
    fn completer_skips_internal_symbols() {
        let buf = Arc::new(Mutex::new(
            "const __tyrus_internal: i32 = 0;\nconst userVar: string = \"hi\";\n".to_string(),
        ));
        let mut c = ReplCompleter::new(buf);
        let suggestions = c.suggestions_for("__", 2);
        assert!(
            !suggestions.iter().any(|s| s.value.starts_with("__tyrus")),
            "internal symbols should not appear in completions",
        );
    }

    #[test]
    fn completer_empty_buffer_returns_keywords() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for("co", 2);
        assert!(!suggestions.is_empty(), "expected keyword completions");
        assert!(
            suggestions.iter().any(|s| s.value.starts_with("const")),
            "expected `const` keyword"
        );
    }

    #[test]
    fn completer_load_save_restore_have_path_completion() {
        let mut c = make_completer();
        let suggestions = c.suggestions_for(":load ", 6);
        assert!(
            !suggestions.is_empty(),
            "expected path completions for :load "
        );
    }

    #[test]
    fn completer_restore_offers_saved_session_names() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("alpha.tyr"), "let a = 1;").unwrap();
        fs::write(dir.path().join("beta.tyr"), "let b = 2;").unwrap();
        fs::write(dir.path().join("ignore.txt"), "not a session").unwrap();
        let suggestions = complete_saved_sessions("al", 3, dir.path());
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].value, "alpha.tyr");
        assert_eq!(suggestions[0].span.start, 1);
        assert_eq!(suggestions[0].span.end, 3);
    }

    #[test]
    fn session_dest_requires_tyr_and_bare_name() {
        let dir = PathBuf::from("/home/user/.tyrus/repl");
        assert_eq!(
            session_dest(&dir, "session.tyr"),
            Ok(dir.join("session.tyr"))
        );
        assert!(session_dest(&dir, "session").is_err());
        assert!(session_dest(&dir, "session.txt").is_err());
        assert!(session_dest(&dir, "../session.tyr").is_err());
        assert!(session_dest(&dir, "/tmp/session.tyr").is_err());
    }

    #[test]
    fn session_file_appends_tyr_when_missing() {
        let dir = PathBuf::from("/home/user/.tyrus/repl");
        assert_eq!(session_file(&dir, "foo"), dir.join("foo.tyr"));
        assert_eq!(session_file(&dir, "foo.tyr"), dir.join("foo.tyr"));
        assert_eq!(session_file(&dir, "bar.txt"), dir.join("bar.txt.tyr"));
    }

    #[test]
    fn relative_time_covers_units() {
        let now = 1_000_000;
        assert_eq!(relative_time(now, now), "just now");
        assert_eq!(relative_time(now, now - 59), "just now");
        assert_eq!(relative_time(now, now - 120), "2m ago");
        assert_eq!(relative_time(now, now - 5_000), "1h ago");
        assert_eq!(relative_time(now, now - 100_000), "1d ago");
    }

    #[test]
    fn display_sessions_lists_tyr_files_newest_first() {
        let dir = TempDir::new().unwrap();
        let old = dir.path().join("old.tyr");
        let new = dir.path().join("new.tyr");
        fs::write(&old, "let a = 1;\n").unwrap();
        fs::write(&new, "let b = 2;\nlet c = 3;\n").unwrap();
        let times_old = fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(1_000));
        let times_new = fs::FileTimes::new().set_modified(UNIX_EPOCH + Duration::from_secs(2_000));
        fs::OpenOptions::new()
            .write(true)
            .open(&old)
            .unwrap()
            .set_times(times_old)
            .unwrap();
        fs::OpenOptions::new()
            .write(true)
            .open(&new)
            .unwrap()
            .set_times(times_new)
            .unwrap();
        fs::write(dir.path().join("notes.txt"), "not a session").unwrap();

        let out = display_sessions(dir.path());
        assert!(out.starts_with("Saved sessions:\n"), "got: {out}");
        assert!(
            out.find("new.tyr").unwrap() < out.find("old.tyr").unwrap(),
            "newest session should come first: {out}"
        );
        assert!(
            out.contains("2 lines"),
            "line counts should be shown: {out}"
        );
        assert!(
            !out.contains("notes.txt"),
            "non-.tyr files should be ignored: {out}"
        );
    }
    #[test]
    fn display_sessions_empty_has_guidance() {
        let dir = TempDir::new().unwrap();
        let expected = "No saved sessions yet. Use :save <name.tyr> to save the session buffer.";
        assert_eq!(display_sessions(dir.path()), expected);
        assert_eq!(display_sessions(&dir.path().join("missing")), expected);
    }

    #[test]
    fn help_aligns_descriptions_to_longest_command() {
        let lines = help_lines();
        let columns: Vec<usize> = lines
            .iter()
            .map(|line| line.find("  ").unwrap_or(line.len()) + 2)
            .collect();
        assert!(columns.iter().all(|c| *c == columns[0]), "{lines:?}");
        assert!(
            lines
                .iter()
                .any(|l| l.trim_start().starts_with(":restore <name|file>"))
        );
    }

    #[test]
    fn delete_removes_saved_session_file() {
        let _guard = crate::compile::acquire_jit_lock();
        let mut session = Session::new();
        let dir = TempDir::new().unwrap();
        session.sessions_dir = dir.path().to_path_buf();
        fs::write(dir.path().join("session.tyr"), "let a = 1;").unwrap();
        session.delete("session.tyr");
        assert!(!dir.path().join("session.tyr").exists());
        session.delete("session.tyr");
        fs::write(dir.path().join("other.tyr"), "let b = 2;").unwrap();
        session.delete("other");
        assert!(!dir.path().join("other.tyr").exists());
    }

    #[test]
    fn restore_without_merge_resets_before_loading() {
        let _guard = crate::compile::acquire_jit_lock();
        let mut session = Session::new();
        let dir = TempDir::new().unwrap();
        session.sessions_dir = dir.path().to_path_buf();
        session.eval("let x: i32 = 10;").unwrap();
        session.save("state1.tyr");
        session.eval("let y: i32 = 20;").unwrap();

        session.restore("state1", false);

        assert!(session.buffer.contains("let x: i32 = 10;"));
        assert!(!session.buffer.contains("let y: i32 = 20;"));
    }

    #[test]
    fn restore_merge_rejects_conflicting_names_without_changing_the_session() {
        let _guard = crate::compile::acquire_jit_lock();
        let mut session = Session::new();
        let dir = TempDir::new().unwrap();
        session.sessions_dir = dir.path().to_path_buf();
        session.eval("let x: i32 = 10;").unwrap();
        session.save("state1.tyr");
        session.eval("let y: i32 = 20;").unwrap();
        let buffer_before = session.buffer.clone();

        session.restore("state1", true);

        assert_eq!(session.buffer, buffer_before);
    }

    #[test]
    fn restore_merge_loads_without_conflict() {
        let _guard = crate::compile::acquire_jit_lock();
        let mut session = Session::new();
        let dir = TempDir::new().unwrap();
        session.sessions_dir = dir.path().to_path_buf();
        session.eval("let a: i32 = 1;").unwrap();
        session.save("session_a.tyr");
        session.reset();
        session.eval("let b: i32 = 2;").unwrap();

        session.restore("session_a", true);

        assert!(session.buffer.contains("let a: i32 = 1;"));
        assert!(session.buffer.contains("let b: i32 = 2;"));
    }

    #[test]
    fn load_file_replays_each_statement_as_a_separate_unit() {
        let _guard = crate::compile::acquire_jit_lock();
        let mut session = Session::new();
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("file.tyr");
        fs::write(
            &path,
            "const message: string = \"Hello World\";\nconsole.log(message);\n",
        )
        .unwrap();

        session.load_file(path.to_str().unwrap());

        assert!(
            session
                .buffer
                .contains("const message: string = \"Hello World\";")
        );
        assert!(session.buffer.contains("console.log(message);"));
        assert_eq!(session.units.len(), 2);
    }

    #[test]
    fn load_file_rolls_back_on_invalid_code() {
        let _guard = crate::compile::acquire_jit_lock();
        let mut session = Session::new();
        session.eval("let x: i32 = 1;").unwrap();
        let buffer_before = session.buffer.clone();

        let dir = TempDir::new().unwrap();
        let path = dir.path().join("broken.tyr");
        fs::write(&path, "let z: i32 = \n").unwrap();

        session.load_file(path.to_str().unwrap());

        assert_eq!(session.buffer, buffer_before);
    }

    fn highlight_line(line: &str) -> StyledText {
        TyrusHighlighter.highlight(line, line.len())
    }

    fn style_of(styled: &StyledText, text: &str) -> Style {
        styled
            .buffer
            .iter()
            .find(|(_, segment)| segment == text)
            .unwrap_or_else(|| panic!("no highlighted segment `{text}` in {:?}", styled.buffer))
            .0
    }

    fn fg_color(styled: &StyledText, text: &str) -> Option<Color> {
        style_of(styled, text).foreground
    }

    #[test]
    fn highlighter_colors_keywords() {
        let styled = highlight_line("catch (error: unknown) {");
        assert_eq!(fg_color(&styled, "catch"), Some(CONTROL_KEYWORD_COLOR));
        assert_eq!(fg_color(&styled, "unknown"), Some(KEYWORD_COLOR));
        assert_eq!(
            fg_color(&highlight_line("try { } finally { }"), "finally"),
            Some(CONTROL_KEYWORD_COLOR)
        );
        assert_eq!(
            fg_color(&highlight_line("for (const x of xs)"), "of"),
            Some(CONTROL_KEYWORD_COLOR)
        );
        assert_eq!(
            fg_color(&highlight_line("const x = 1;"), "const"),
            Some(KEYWORD_COLOR)
        );
        assert_eq!(
            fg_color(&highlight_line("import { x } from \"mod\";"), "from"),
            Some(CONTROL_KEYWORD_COLOR)
        );
        assert_eq!(
            fg_color(&highlight_line("match (v) { when 1 => ok, }"), "when"),
            Some(CONTROL_KEYWORD_COLOR)
        );
        assert_eq!(fg_color(&styled, ":"), Some(CONTROL_KEYWORD_COLOR));
    }

    #[test]
    fn highlighter_colors_keyword_family_edge_cases() {
        assert_eq!(
            fg_color(&highlight_line("const f = () => 1;"), "=>"),
            Some(KEYWORD_COLOR)
        );
        assert_eq!(
            fg_color(&highlight_line("const t = typeof x;"), "typeof"),
            Some(CONTROL_KEYWORD_COLOR)
        );
        assert_eq!(
            fg_color(&highlight_line("@decorator at x"), "at"),
            Some(CONTROL_KEYWORD_COLOR)
        );
        assert_eq!(
            fg_color(&highlight_line("const v = await p;"), "await"),
            Some(CONTROL_KEYWORD_COLOR)
        );
        assert_eq!(
            fg_color(&highlight_line("interface A extends B {}"), "extends"),
            Some(CONTROL_KEYWORD_COLOR)
        );
        assert_eq!(
            fg_color(
                &highlight_line("interface A { readonly x: i32 }"),
                "readonly"
            ),
            Some(CONTROL_KEYWORD_COLOR)
        );
        assert_eq!(
            fg_color(&highlight_line("const n = None;"), "None"),
            Some(IDENTIFIER_COLOR)
        );
    }

    #[test]
    fn highlighter_colors_primitive_types() {
        let styled = highlight_line("const x: i32 = 1;");
        assert_eq!(fg_color(&styled, "i32"), Some(KEYWORD_COLOR));
    }

    #[test]
    fn highlighter_colors_strings_templates_and_regex() {
        assert_eq!(
            fg_color(&highlight_line(r#"const s = "hi";"#), r#""hi""#),
            Some(STRING_COLOR)
        );
        assert_eq!(
            fg_color(&highlight_line("const c = 'x';"), "'x'"),
            Some(STRING_COLOR)
        );
        let styled = highlight_line("const t = `hi ${name}`;");
        assert_eq!(fg_color(&styled, "`hi "), Some(STRING_COLOR));
        assert_eq!(fg_color(&styled, "${"), Some(INTERPOLATION_COLOR));
        assert_eq!(fg_color(&styled, "name"), Some(IDENTIFIER_COLOR));
        assert_eq!(fg_color(&styled, "}"), Some(INTERPOLATION_COLOR));
        assert_eq!(fg_color(&styled, "`"), Some(STRING_COLOR));
        assert_eq!(
            fg_color(&highlight_line("const r = /ab+c/;"), "/ab+c/"),
            Some(REGEX_COLOR)
        );
        let styled = highlight_line(r#"console.log("// not a comment");"#);
        assert_eq!(
            fg_color(&styled, r#""// not a comment""#),
            Some(STRING_COLOR)
        );
        assert!(
            styled
                .buffer
                .iter()
                .all(|(_, segment)| segment != "// not a comment")
        );
    }

    #[test]
    fn highlighter_colors_template_interpolation() {
        let styled = highlight_line(r#"console.log(`PI = ${y}`);"#);
        assert_eq!(fg_color(&styled, "${"), Some(INTERPOLATION_COLOR));
        assert_eq!(fg_color(&styled, "}"), Some(INTERPOLATION_COLOR));
        assert_eq!(fg_color(&styled, "y"), Some(IDENTIFIER_COLOR));

        let styled = highlight_line(r#"`${add(1, "x")}`"#);
        assert_eq!(fg_color(&styled, "add"), Some(FUNCTION_COLOR));
        assert_eq!(fg_color(&styled, "1"), Some(NUMBER_COLOR));
        assert_eq!(fg_color(&styled, r#""x""#), Some(STRING_COLOR));

        let styled = highlight_line(r#"`${f({ a: 1 })}`"#);
        let close_brace_colors: Vec<Color> = styled
            .buffer
            .iter()
            .filter(|(_, segment)| segment == "}")
            .filter_map(|(style, _)| style.foreground)
            .collect();
        assert_eq!(
            close_brace_colors,
            vec![BRACKET_COLORS[1], INTERPOLATION_COLOR]
        );

        let styled = highlight_line(r#"`price: \${amount}`"#);
        assert!(styled.buffer.iter().all(|(_, segment)| segment != "${"));
    }

    #[test]
    fn highlighter_colors_numbers_and_literals() {
        assert_eq!(
            fg_color(&highlight_line("const n = 42;"), "42"),
            Some(NUMBER_COLOR)
        );
        assert_eq!(
            fg_color(&highlight_line("const f = 3.14;"), "3.14"),
            Some(NUMBER_COLOR)
        );
        assert_eq!(
            fg_color(&highlight_line("const h = 0xFF;"), "0xFF"),
            Some(NUMBER_COLOR)
        );
        let styled = highlight_line("const ok = true;");
        assert_eq!(fg_color(&styled, "true"), Some(NUMBER_COLOR));
        assert_eq!(
            fg_color(&highlight_line("const missing = null;"), "null"),
            Some(NUMBER_COLOR)
        );
    }

    #[test]
    fn highlighter_colors_typed_numeric_suffix() {
        let styled = highlight_line("const x = 5u32;");
        assert_eq!(fg_color(&styled, "5"), Some(NUMBER_COLOR));
        assert_eq!(fg_color(&styled, "u32"), Some(KEYWORD_COLOR));

        let styled = highlight_line("const x = 5i64;");
        assert_eq!(fg_color(&styled, "5"), Some(NUMBER_COLOR));
        assert_eq!(fg_color(&styled, "i64"), Some(KEYWORD_COLOR));

        let styled = highlight_line("const x = 3.14f64;");
        assert_eq!(fg_color(&styled, "3.14"), Some(NUMBER_COLOR));
        assert_eq!(fg_color(&styled, "f64"), Some(KEYWORD_COLOR));

        let styled = highlight_line("const x = 9usize;");
        assert_eq!(fg_color(&styled, "9"), Some(NUMBER_COLOR));
        assert_eq!(fg_color(&styled, "usize"), Some(KEYWORD_COLOR));

        let styled = highlight_line("const x = 2d;");
        assert_eq!(fg_color(&styled, "2"), Some(NUMBER_COLOR));
        assert_eq!(fg_color(&styled, "d"), Some(KEYWORD_COLOR));

        let styled = highlight_line("const x = 0xdeadu32;");
        assert_eq!(fg_color(&styled, "0xdead"), Some(NUMBER_COLOR));
        assert_eq!(fg_color(&styled, "u32"), Some(KEYWORD_COLOR));
    }

    #[test]
    fn highlighter_colors_comments() {
        let styled = highlight_line("const x = 1; // note");
        assert_eq!(fg_color(&styled, "// note"), Some(COMMENT_COLOR));
        let styled = highlight_line("/* doc */ const x = 1;");
        assert_eq!(fg_color(&styled, "/* doc */"), Some(COMMENT_COLOR));
        let styled = highlight_line("const x = 1; /* a */ // b");
        assert_eq!(fg_color(&styled, "/* a */"), Some(COMMENT_COLOR));
        assert_eq!(fg_color(&styled, "// b"), Some(COMMENT_COLOR));
    }

    #[test]
    fn highlighter_colors_functions_and_operators() {
        let styled = highlight_line("const y = add(1, 2) + 3;");
        assert_eq!(fg_color(&styled, "add"), Some(FUNCTION_COLOR));
        assert_eq!(fg_color(&styled, "+"), Some(OPERATOR_COLOR));
        let styled = highlight_line("console.log(x);");
        assert_eq!(fg_color(&styled, "log"), Some(FUNCTION_COLOR));
        assert_eq!(fg_color(&styled, "console"), Some(IDENTIFIER_COLOR));
    }

    #[test]
    fn highlighter_colors_identifiers() {
        let styled = highlight_line("let name = 1;");
        assert_eq!(fg_color(&styled, "let"), Some(KEYWORD_COLOR));
        assert_eq!(fg_color(&styled, "name"), Some(IDENTIFIER_COLOR));
        assert_eq!(fg_color(&styled, "="), Some(OPERATOR_COLOR));
    }

    #[test]
    fn highlighter_colors_brackets_and_interpolation() {
        let styled = highlight_line("f(x[y]) { }");
        assert_eq!(fg_color(&styled, "("), Some(BRACKET_COLORS[0]));
        assert_eq!(fg_color(&styled, "["), Some(BRACKET_COLORS[1]));
        assert_eq!(fg_color(&styled, "]"), Some(BRACKET_COLORS[1]));
        assert_eq!(fg_color(&styled, ")"), Some(BRACKET_COLORS[0]));
        assert_eq!(fg_color(&styled, "{"), Some(BRACKET_COLORS[0]));
        assert_eq!(fg_color(&styled, "}"), Some(BRACKET_COLORS[0]));
        let styled = highlight_line("a((b))");
        let lparen_colors: Vec<Color> = styled
            .buffer
            .iter()
            .filter(|(_, segment)| segment == "(")
            .filter_map(|(style, _)| style.foreground)
            .collect();
        assert_eq!(lparen_colors, vec![BRACKET_COLORS[0], BRACKET_COLORS[1]]);
    }

    #[test]
    fn highlighter_colors_commands() {
        let styled = highlight_line(":load foo.tyr");
        assert_eq!(fg_color(&styled, ":load"), Some(Color::Cyan));
        assert!(style_of(&styled, ":load").is_bold);
    }

    #[test]
    fn highlighter_colors_reserved_future_keywords() {
        let styled = highlight_line("const class = 1;");
        assert_eq!(fg_color(&styled, "class"), Some(CONTROL_KEYWORD_COLOR));
        let styled = highlight_line("settings.delete");
        assert_eq!(fg_color(&styled, "delete"), Some(IDENTIFIER_COLOR));
    }

    #[test]
    fn highlighter_fallback_colors_unterminated_string() {
        let styled = highlight_line(r#"const s = "abc"#);
        assert_eq!(fg_color(&styled, r#""abc"#), Some(STRING_COLOR));
        assert_eq!(fg_color(&styled, "const"), Some(KEYWORD_COLOR));
    }

    #[test]
    fn highlighter_fallback_colors_brackets_and_interpolation() {
        let styled = highlight_line(r#"const t = `hi ${na"#);
        assert_eq!(fg_color(&styled, "`hi "), Some(STRING_COLOR));
        assert_eq!(fg_color(&styled, "${"), Some(INTERPOLATION_COLOR));
        assert_eq!(fg_color(&styled, "na"), Some(IDENTIFIER_COLOR));
        let styled = highlight_line(r#"f((x); "oops"#);
        let lparen_colors: Vec<Color> = styled
            .buffer
            .iter()
            .filter(|(_, segment)| segment == "(")
            .filter_map(|(style, _)| style.foreground)
            .collect();
        assert_eq!(lparen_colors, vec![BRACKET_COLORS[0], BRACKET_COLORS[1]]);
        assert_eq!(fg_color(&styled, r#""oops"#), Some(STRING_COLOR));
    }

    #[test]
    fn input_status_is_ready_for_a_bare_expression() {
        assert!(matches!(input_status("x"), InputStatus::Ready));
        assert!(matches!(input_status("x + 1"), InputStatus::Ready));
        assert!(matches!(
            input_status("const y: i32 = 2"),
            InputStatus::Ready
        ));
    }

    #[test]
    fn input_status_still_waits_for_an_open_construct() {
        assert!(matches!(
            input_status("function f(): void {"),
            InputStatus::Incomplete
        ));
    }

    #[test]
    fn a_bare_expression_is_stored_with_its_terminator() {
        let mut session = Session::new();
        session.eval("let x: i32 = 10;").unwrap();
        session.eval("x").unwrap();
        assert_eq!(session.units.last().map(String::as_str), Some("x;"));
    }

    #[test]
    fn input_status_ready_for_complete_input() {
        assert!(matches!(input_status("let x = 1;"), InputStatus::Ready));
        assert!(matches!(
            input_status("console.log(numbers[1]);"),
            InputStatus::Ready
        ));
        assert!(matches!(
            input_status(r#"console.log(")");"#),
            InputStatus::Ready
        ));
        assert!(matches!(
            input_status("function add(a: i32, b: i32): i32 { return a + b; }"),
            InputStatus::Ready
        ));
    }

    #[test]
    fn input_status_incomplete_for_unclosed_constructs() {
        assert!(matches!(
            input_status("console.log(numbers[1]"),
            InputStatus::Incomplete
        ));
        assert!(matches!(
            input_status("function add(a: i32, b: i32): i32 {"),
            InputStatus::Incomplete
        ));
        assert!(matches!(
            input_status("let s = \"abc"),
            InputStatus::Incomplete
        ));
    }

    #[test]
    fn input_status_errors_on_stray_tokens() {
        match input_status("console.log(numbers[1];") {
            InputStatus::Error(_) => {}
            other => panic!("expected Error, got {other:?}"),
        }
        assert!(matches!(input_status(")"), InputStatus::Error(_)));
        assert!(matches!(
            input_status("console.log(;"),
            InputStatus::Error(_)
        ));
    }

    #[test]
    fn repl_incremental_eval_semantics() {
        let _guard = crate::compile::acquire_jit_lock();
        let mut session = ReplJitSession::new().unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        let mut units: Vec<String> = Vec::new();

        fn run_step(
            session: &mut ReplJitSession,
            dir: &tempfile::TempDir,
            units: &mut Vec<String>,
            code: &str,
        ) -> Result<String, String> {
            units.push(code.to_string());
            let transform = repl_transform::transform_units(units, test_type_resolver(units));
            let src = dir.path().join("src");
            fs::create_dir_all(&src).unwrap();
            let entry = src.join("main.tyr");
            fs::write(&entry, &transform.source).unwrap();
            let program =
                module_loader::load_program(&entry, None).map_err(|e| format!("error: {e}"))?;
            if transform.last_has_step {
                let step_symbol = repl_transform::step_symbol(units.len());
                session.add_and_run(&program, "__repl__", &step_symbol)
            } else {
                session.add_module(&program, "__repl__")?;
                Ok(String::new())
            }
        }

        assert_eq!(
            run_step(&mut session, &dir, &mut units, "console.log(1);")
                .unwrap()
                .trim(),
            "1"
        );
        assert_eq!(
            run_step(&mut session, &dir, &mut units, "console.log(2);")
                .unwrap()
                .trim(),
            "2"
        );
        assert_eq!(
            run_step(
                &mut session,
                &dir,
                &mut units,
                "console.log(\"hello\".includes(\"he\"));",
            )
            .unwrap()
            .trim(),
            "true",
            "the ORC JIT must resolve runtime symbols from the host process"
        );
        assert_eq!(
            run_step(&mut session, &dir, &mut units, "console.log(3);")
                .unwrap()
                .trim(),
            "3"
        );

        assert_eq!(
            run_step(
                &mut session,
                &dir,
                &mut units,
                "const add = (a: i32, b: i32): i32 => a + b;"
            )
            .unwrap()
            .trim(),
            ""
        );
        assert_eq!(
            run_step(&mut session, &dir, &mut units, "console.log(add(1, 2));")
                .unwrap()
                .trim(),
            "3"
        );

        run_step(&mut session, &dir, &mut units, "let x: i32 = 5;").unwrap();
        run_step(&mut session, &dir, &mut units, "x = x + 10;").unwrap();
        assert_eq!(
            run_step(&mut session, &dir, &mut units, "console.log(x);")
                .unwrap()
                .trim(),
            "15"
        );

        let err = run_step(&mut session, &dir, &mut units, "let bad: i32 = ;");
        assert!(err.is_err(), "broken input should be rejected");
        units.pop();
        assert_eq!(
            run_step(&mut session, &dir, &mut units, "console.log(9);")
                .unwrap()
                .trim(),
            "9"
        );
    }

    #[test]
    fn cmd_type_resolves_session_symbols() {
        let _guard = crate::compile::acquire_jit_lock();
        let mut session = Session::new();
        session.eval("let x: i32 = 5;").unwrap();
        session
            .eval("const add = (a: i32, b: i32): i32 => a + b;")
            .unwrap();

        assert_eq!(session.cmd_type("42").unwrap(), "i32");
        assert_eq!(session.cmd_type("x").unwrap(), "i32");
        assert_eq!(session.cmd_type("x + 10").unwrap(), "i32");
        assert_eq!(session.cmd_type("add(1, 2)").unwrap(), "i32");

        let err = session.cmd_type("missing").unwrap_err();
        assert!(
            err.contains("missing"),
            "error should mention the unresolved symbol, got: {err}"
        );
    }

    /// A contained array declared on one REPL line and read on the next.
    ///
    /// The escape analysis promotes an array literal to a stack slot when
    /// nothing in the body it can see carries the array out of scope. A REPL
    /// line is its own compilation unit, so "the body it can see" is one line,
    /// and the binding has to outlive that line. If the promotion applies here,
    /// the second line reads a frame that is already gone: a use-after-return
    /// that no leak check can catch, because stack memory does not leak.
    #[test]
    fn repl_array_declared_on_one_line_survives_the_next() {
        let _guard = crate::compile::acquire_jit_lock();
        let mut session = ReplJitSession::new().unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        let mut units: Vec<String> = Vec::new();

        fn run_step(
            session: &mut ReplJitSession,
            dir: &tempfile::TempDir,
            units: &mut Vec<String>,
            code: &str,
        ) -> Result<String, String> {
            units.push(code.to_string());
            let transform = repl_transform::transform_units(units, test_type_resolver(units));
            let src = dir.path().join("src");
            fs::create_dir_all(&src).unwrap();
            let entry = src.join("main.tyr");
            fs::write(&entry, &transform.source).unwrap();
            let program =
                module_loader::load_program(&entry, None).map_err(|e| format!("error: {e}"))?;
            if transform.last_has_step {
                let step_symbol = repl_transform::step_symbol(units.len());
                session.add_and_run(&program, "__repl__", &step_symbol)
            } else {
                session.add_module(&program, "__repl__")?;
                Ok(String::new())
            }
        }

        assert_eq!(
            run_step(
                &mut session,
                &dir,
                &mut units,
                r#"const numbers: i32[] = [11, 22, 33];"#
            )
            .unwrap(),
            ""
        );
        assert_eq!(
            run_step(
                &mut session,
                &dir,
                &mut units,
                r#"console.log(numbers[0]);"#
            )
            .unwrap(),
            "11\n"
        );
        assert_eq!(
            run_step(
                &mut session,
                &dir,
                &mut units,
                r#"console.log(numbers[0] + numbers[1] + numbers[2]);"#
            )
            .unwrap(),
            "66\n"
        );
    }

    #[test]
    fn repl_template_string_globals_survive_steps() {
        let _guard = crate::compile::acquire_jit_lock();
        let mut session = ReplJitSession::new().unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        let mut units: Vec<String> = Vec::new();

        fn run_step(
            session: &mut ReplJitSession,
            dir: &tempfile::TempDir,
            units: &mut Vec<String>,
            code: &str,
        ) -> Result<String, String> {
            units.push(code.to_string());
            let transform = repl_transform::transform_units(units, test_type_resolver(units));
            let src = dir.path().join("src");
            fs::create_dir_all(&src).unwrap();
            let entry = src.join("main.tyr");
            fs::write(&entry, &transform.source).unwrap();
            let program =
                module_loader::load_program(&entry, None).map_err(|e| format!("error: {e}"))?;
            if transform.last_has_step {
                let step_symbol = repl_transform::step_symbol(units.len());
                session.add_and_run(&program, "__repl__", &step_symbol)
            } else {
                session.add_module(&program, "__repl__")?;
                Ok(String::new())
            }
        }

        assert_eq!(
            run_step(&mut session, &dir, &mut units, r#"let name = "Thomas";"#).unwrap(),
            ""
        );
        assert_eq!(
            run_step(
                &mut session,
                &dir,
                &mut units,
                r#"let surname = "Anderson";"#
            )
            .unwrap(),
            ""
        );
        assert_eq!(
            run_step(
                &mut session,
                &dir,
                &mut units,
                r#"let completeName = `${name} ${surname}`;"#
            )
            .unwrap(),
            ""
        );
        assert_eq!(
            run_step(&mut session, &dir, &mut units, r#"console.log(name);"#).unwrap(),
            "Thomas\n"
        );
        assert_eq!(
            run_step(
                &mut session,
                &dir,
                &mut units,
                r#"console.log(completeName);"#
            )
            .unwrap(),
            "Thomas Anderson\n"
        );
    }

    #[test]
    fn repl_declaration_referencing_global_is_not_a_phantom_move() {
        let _guard = crate::compile::acquire_jit_lock();
        let mut session = ReplJitSession::new().unwrap();
        let dir = tempfile::TempDir::new().unwrap();
        let mut units: Vec<String> = Vec::new();

        fn run_step(
            session: &mut ReplJitSession,
            dir: &tempfile::TempDir,
            units: &mut Vec<String>,
            code: &str,
        ) -> Result<String, String> {
            units.push(code.to_string());
            let transform = repl_transform::transform_units(units, test_type_resolver(units));
            let src = dir.path().join("src");
            fs::create_dir_all(&src).unwrap();
            let entry = src.join("main.tyr");
            fs::write(&entry, &transform.source).unwrap();
            let program =
                module_loader::load_program(&entry, None).map_err(|e| format!("error: {e}"))?;
            if transform.last_has_step {
                let step_symbol = repl_transform::step_symbol(units.len());
                session.add_and_run(&program, "__repl__", &step_symbol)
            } else {
                session.add_module(&program, "__repl__")?;
                Ok(String::new())
            }
        }

        run_step(&mut session, &dir, &mut units, r#"let name = "Thomas";"#).unwrap();
        run_step(
            &mut session,
            &dir,
            &mut units,
            r#"let surname = "Anderson";"#,
        )
        .unwrap();
        run_step(
            &mut session,
            &dir,
            &mut units,
            r#"let completeName = `${name} ${surname}`;"#,
        )
        .unwrap();
        assert_eq!(
            run_step(&mut session, &dir, &mut units, r#"console.log(name);"#).unwrap(),
            "Thomas\n"
        );
        assert_eq!(
            run_step(&mut session, &dir, &mut units, r#"console.log(surname);"#).unwrap(),
            "Anderson\n"
        );
        assert_eq!(
            run_step(
                &mut session,
                &dir,
                &mut units,
                r#"console.log(completeName);"#
            )
            .unwrap(),
            "Thomas Anderson\n"
        );

        run_step(&mut session, &dir, &mut units, r#"let newName = name;"#).unwrap();
        assert_eq!(
            run_step(&mut session, &dir, &mut units, r#"console.log(newName);"#).unwrap(),
            "Thomas\n"
        );

        let err = run_step(&mut session, &dir, &mut units, r#"console.log(name);"#).unwrap_err();
        assert!(
            err.contains("E0382"),
            "a genuine post-move use must still report E0382, got: {err}"
        );
        units.pop();
        assert_eq!(
            run_step(&mut session, &dir, &mut units, r#"console.log(newName);"#).unwrap(),
            "Thomas\n"
        );
    }

    #[test]
    fn repl_session_declaration_from_global_compiles() {
        let _guard = crate::compile::acquire_jit_lock();
        let mut session = Session::new();
        session.eval(r#"let name = "Thomas";"#).unwrap();
        session.eval(r#"let surname = "Anderson";"#).unwrap();
        session
            .eval(r#"let completeName = `${name} ${surname}`;"#)
            .unwrap();
        session.eval(r#"console.log(name);"#).unwrap();
        session.eval(r#"console.log(completeName);"#).unwrap();
        session.eval(r#"let newName = name;"#).unwrap();
        session.eval(r#"console.log(newName);"#).unwrap();
    }

    /// REPL memory leak regression: exercise Session methods (eval, load_file,
    /// reset, save, restore) 200 times under valgrind, including an owned
    /// (heap-allocated) `string` module-scope declaration, use, and
    /// reassignment through the real incremental JIT path, not just `i32`,
    /// a `Copy` type that never exercises string ownership. Run via:
    ///   cargo test --bin tyrus --no-run -- -- repl_leak_loop
    ///   valgrind --leak-check=summary <test-binary> repl_leak_loop --nocapture
    #[test]
    fn repl_leak_loop() {
        let _guard = crate::compile::acquire_jit_lock();

        let load_dir = TempDir::new().unwrap();
        let load_path = load_dir.path().join("test.tyr");
        fs::write(
            &load_path,
            "const loaded: i32 = 99;\nconst loadedName: string = \"loaded\";\n",
        )
        .unwrap();
        let load_str = load_path.to_str().unwrap().to_string();

        let save_dir = TempDir::new().unwrap();
        let sessions_dir = save_dir.path().join("sessions");

        for i in 0..200 {
            let mut session = Session::new();
            session.sessions_dir = sessions_dir.clone();

            session
                .eval("let x: i32 = 10;")
                .unwrap_or_else(|e| panic!("iteration {i}: eval statement failed: {e}"));

            session
                .eval("x")
                .unwrap_or_else(|e| panic!("iteration {i}: eval expression failed: {e}"));

            session
                .eval(r#"let name: string = "hello world";"#)
                .unwrap_or_else(|e| panic!("iteration {i}: eval string decl failed: {e}"));
            session
                .eval("console.log(name);")
                .unwrap_or_else(|e| panic!("iteration {i}: eval string use failed: {e}"));

            session
                .eval(r#"let mutableName: string = "first";"#)
                .unwrap_or_else(|e| panic!("iteration {i}: eval mutable decl failed: {e}"));
            session
                .eval(r#"mutableName = "second";"#)
                .unwrap_or_else(|e| panic!("iteration {i}: eval reassign failed: {e}"));

            let _ = session.eval("let z: i32 = ");

            session.load_file(&load_str);

            session.save("session.tyr");
            session.restore("session.tyr", false);

            session.reset();
        }
    }
}

fn banner_text() -> String {
    figlet_rs::FIGlet::slant()
        .ok()
        .and_then(|font| font.convert("Tyrus REPL").map(|figure| figure.to_string()))
        .unwrap_or_else(|| "Tyrus REPL".to_string())
}

fn print_banner() {
    eprintln!("{}", Style::new().fg(Color::Cyan).paint(banner_text()));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Delimiter {
    open: char,
    close: char,
}

impl Delimiter {
    fn open(self) -> char {
        self.open
    }

    fn close(self) -> char {
        self.close
    }

    fn is_quote(self) -> bool {
        matches!(
            self,
            Delimiter::PUNCT | Delimiter::SINGLE_QUOTE | Delimiter::BACKTICK
        )
    }

    fn is_symmetric(self) -> bool {
        self.open == self.close
    }
}

impl Delimiter {
    const PAREN: Delimiter = Delimiter {
        open: '(',
        close: ')',
    };
    const BRACKET: Delimiter = Delimiter {
        open: '[',
        close: ']',
    };
    const BRACE: Delimiter = Delimiter {
        open: '{',
        close: '}',
    };
    const PUNCT: Delimiter = Delimiter {
        open: '"',
        close: '"',
    };
    const SINGLE_QUOTE: Delimiter = Delimiter {
        open: '\'',
        close: '\'',
    };
    const BACKTICK: Delimiter = Delimiter {
        open: '`',
        close: '`',
    };
}

fn delimiter_of(c: char) -> Option<Delimiter> {
    match c {
        '(' | ')' => Some(Delimiter::PAREN),
        '[' | ']' => Some(Delimiter::BRACKET),
        '{' | '}' => Some(Delimiter::BRACE),
        '"' => Some(Delimiter::PUNCT),
        '\'' => Some(Delimiter::SINGLE_QUOTE),
        '`' => Some(Delimiter::BACKTICK),
        _ => None,
    }
}

fn is_typing_modifier(modifiers: KeyModifiers) -> bool {
    modifiers == KeyModifiers::NONE || modifiers == KeyModifiers::SHIFT
}

struct AutoCloseEmacs {
    inner: Emacs,
    stack: Vec<Delimiter>,
    depths: Vec<usize>,
    cursor_before_closer: bool,
}

impl AutoCloseEmacs {
    fn new(inner: Emacs) -> Self {
        Self {
            inner,
            stack: Vec::new(),
            depths: Vec::new(),
            cursor_before_closer: false,
        }
    }

    fn reset(&mut self) {
        self.stack.clear();
        self.depths.clear();
        self.cursor_before_closer = false;
    }

    fn cursor_before_closer(&self) -> bool {
        self.cursor_before_closer
    }

    fn on_typing_char(&mut self, c: char) -> Option<ReedlineEvent> {
        match delimiter_of(c) {
            Some(delim) if delim.is_symmetric() => {
                if self.cursor_before_closer() && self.stack.last() == Some(&delim) {
                    self.on_closer(delim)
                } else {
                    self.on_opener(delim)
                }
            }
            Some(delim) if c == delim.close() => self.on_closer(delim),
            Some(delim) => self.on_opener(delim),
            None => {
                if self.cursor_before_closer()
                    && let Some(depth) = self.depths.last_mut()
                {
                    *depth += 1;
                }
                None
            }
        }
    }

    fn on_opener(&mut self, delim: Delimiter) -> Option<ReedlineEvent> {
        if self.cursor_before_closer()
            && let Some(top) = self.stack.last().copied()
        {
            if top.is_quote() {
                self.bump_depth();
                return None;
            }
            self.stack.push(delim);
            self.depths.push(0);
            return Some(self.auto_close(delim));
        }
        self.stack.push(delim);
        self.depths.push(0);
        self.cursor_before_closer = true;
        Some(self.auto_close(delim))
    }

    fn on_closer(&mut self, delim: Delimiter) -> Option<ReedlineEvent> {
        if self.cursor_before_closer()
            && let Some(top) = self.stack.last().copied()
        {
            if top == delim {
                self.stack.pop();
                self.depths.pop();
                self.cursor_before_closer = !self.stack.is_empty();
                return Some(ReedlineEvent::Edit(vec![EditCommand::MoveRight {
                    select: false,
                }]));
            }
            if top.is_quote() {
                self.bump_depth();
                return None;
            }
            self.stack.pop();
            self.depths.pop();
            self.cursor_before_closer = !self.stack.is_empty();
            return Some(ReedlineEvent::Edit(vec![
                EditCommand::Delete,
                EditCommand::InsertChar(delim.close()),
            ]));
        }
        None
    }

    fn bump_depth(&mut self) {
        if let Some(depth) = self.depths.last_mut() {
            *depth += 1;
        }
    }

    fn auto_close(&self, delim: Delimiter) -> ReedlineEvent {
        ReedlineEvent::Edit(vec![
            EditCommand::InsertString(format!("{}{}", delim.open(), delim.close())),
            EditCommand::MoveLeft { select: false },
        ])
    }

    fn on_backspace(&mut self) -> Option<ReedlineEvent> {
        if self.cursor_before_closer()
            && let Some(depth) = self.depths.last().copied()
        {
            if depth > 0 {
                let stack_len = self.stack.len();
                self.depths[stack_len - 1] -= 1;
                return None;
            }
            self.stack.pop();
            self.depths.pop();
            self.cursor_before_closer = !self.stack.is_empty();
            return Some(ReedlineEvent::Edit(vec![
                EditCommand::Backspace,
                EditCommand::Delete,
            ]));
        }
        None
    }

    fn on_delete(&mut self) -> Option<ReedlineEvent> {
        self.reset();
        None
    }

    fn on_enter(&mut self) -> Option<ReedlineEvent> {
        if self.cursor_before_closer() {
            let pending = self.stack.len();
            let mut commands = Vec::with_capacity(pending + 1);
            for _ in 0..pending {
                commands.push(ReedlineEvent::Edit(vec![EditCommand::Delete]));
            }
            self.reset();
            commands.push(ReedlineEvent::Enter);
            return Some(ReedlineEvent::Multiple(commands));
        }
        None
    }
}

impl EditMode for AutoCloseEmacs {
    fn edit_mode(&self) -> PromptEditMode {
        self.inner.edit_mode()
    }

    fn parse_event(&mut self, raw: ReedlineRawEvent) -> ReedlineEvent {
        let event = Event::from(raw);
        let handled = match &event {
            Event::Key(KeyEvent {
                code, modifiers, ..
            }) => match (*modifiers, *code) {
                (m, KeyCode::Char(c)) if is_typing_modifier(m) => self.on_typing_char(c),
                (KeyModifiers::NONE, KeyCode::Backspace) => self.on_backspace(),
                (KeyModifiers::NONE, KeyCode::Delete) => self.on_delete(),
                (KeyModifiers::NONE, KeyCode::Enter) => self.on_enter(),
                _ => {
                    self.reset();
                    None
                }
            },
            Event::Paste(_) => {
                self.reset();
                None
            }
            Event::Mouse(_) | Event::Resize(_, _) | Event::FocusGained | Event::FocusLost => None,
        };
        match handled {
            Some(event) => event,
            None => {
                let raw = ReedlineRawEvent::try_from(event).expect(
                    "the engine only feeds parse_event events it already normalized \
                     (no key releases)",
                );
                self.inner.parse_event(raw)
            }
        }
    }
}

pub fn run() {
    if !std::io::stdin().is_terminal() {
        eprintln!("error: tyrus repl requires an interactive terminal");
        process::exit(1);
    }

    let _terminal_mode = match ReplTerminalMode::enter() {
        Ok(mode) => mode,
        Err(error) => {
            eprintln!("error: failed to configure terminal input: {error}");
            return;
        }
    };

    let mut session = Session::new();

    let hist_path = history_path();
    let history: Option<Box<dyn History>> = FileBackedHistory::with_file(HISTORY_SIZE, hist_path)
        .ok()
        .map(|h| Box::new(h) as Box<dyn History>);
    let input_buffer = session.buffer_arc.clone();
    let completion_menu = Box::new(ColumnarMenu::default().with_name("completion_menu"));
    let mut keybindings = default_emacs_keybindings();
    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );
    let mut editor = Reedline::create()
        .with_completer(Box::new(ReplCompleter::new(input_buffer)))
        .with_highlighter(Box::new(TyrusHighlighter))
        .with_menu(ReedlineMenu::EngineCompleter(completion_menu))
        .with_edit_mode(Box::new(AutoCloseEmacs::new(Emacs::new(keybindings))));
    if let Some(h) = history {
        editor = editor.with_history(h);
    }

    let mut prompt = ReplPrompt {
        continuation: false,
    };

    print_banner();
    eprintln!("Type :help for available commands.");
    eprintln!();

    loop {
        prompt.continuation = false;
        let mut multi = String::new();

        loop {
            let sig = read_repl_line(&mut editor, &prompt);

            match sig {
                Ok(Signal::CtrlC) => {
                    multi.clear();
                    break;
                }
                Ok(Signal::CtrlD) => return,
                Ok(Signal::HostCommand(_) | Signal::ExternalBreak(_)) => continue,
                Ok(Signal::Success(line)) => {
                    let trimmed = line.trim();

                    if let Some(command) = trimmed.strip_prefix(':') {
                        if !multi.is_empty() {
                            multi.clear();
                        }
                        let parts: Vec<&str> = command.splitn(2, char::is_whitespace).collect();
                        match parts[0] {
                            "help" => cmd_help(),
                            "clear" => {
                                session.buffer.clear();
                                session.sync_buffer();
                                print!("\x1b[2J\x1b[H");
                                let _ = std::io::stdout().flush();
                            }
                            "doc" => {
                                if let Some(symbol) = parts.get(1) {
                                    session.cmd_doc(symbol);
                                } else {
                                    eprintln!("error: usage: :doc <symbol>");
                                }
                            }
                            "type" => {
                                if let Some(expr) = parts.get(1) {
                                    match session.cmd_type(expr) {
                                        Ok(ty) => println!("{ty}"),
                                        Err(message) => eprintln!("{message}"),
                                    }
                                } else {
                                    eprintln!("error: usage: :type <expression>");
                                }
                            }
                            "load" => {
                                if let Some(path) = parts.get(1) {
                                    session.load_file(path);
                                } else {
                                    eprintln!("error: usage: :load <file>");
                                }
                            }
                            "list" => print!("{}", session.list()),
                            "edit" => session.edit(),
                            "save" => {
                                if let Some(name) = parts.get(1) {
                                    session.save(name);
                                } else {
                                    eprintln!("error: usage: :save <name.tyr>");
                                }
                            }
                            "restore" => {
                                if let Some(raw) = parts.get(1) {
                                    let mut merge = false;
                                    let mut name_tokens: Vec<&str> = Vec::new();
                                    for token in raw.split_whitespace() {
                                        if token == "--merge" || token == "-m" {
                                            merge = true;
                                        } else {
                                            name_tokens.push(token);
                                        }
                                    }
                                    if name_tokens.is_empty() {
                                        eprintln!(
                                            "error: usage: :restore <name|file> [--merge|-m]"
                                        );
                                    } else {
                                        session.restore(&name_tokens.join(" "), merge);
                                    }
                                } else {
                                    eprintln!("error: usage: :restore <name|file> [--merge|-m]");
                                }
                            }
                            "delete" => {
                                if let Some(name) = parts.get(1) {
                                    session.delete(name);
                                } else {
                                    eprintln!("error: usage: :delete <name>");
                                }
                            }
                            "reset" => {
                                session.reset();
                                println!("Session reset.");
                            }
                            "exit" | "quit" | "q" => return,
                            _ => eprintln!(
                                "error: unknown command `:{cmd}`\nRun :help for available commands.",
                                cmd = parts[0]
                            ),
                        }
                        break;
                    }

                    if trimmed.is_empty() && !multi.is_empty() {
                        if let Err(stderr) = session.eval(multi.trim_end()) {
                            eprint!("{stderr}");
                        }
                        break;
                    }

                    if trimmed.is_empty() {
                        break;
                    }

                    multi.push_str(&line);
                    multi.push('\n');

                    match input_status(&multi) {
                        InputStatus::Ready => {
                            if let Err(stderr) = session.eval(multi.trim_end()) {
                                eprint!("{stderr}");
                            }
                            break;
                        }
                        InputStatus::Incomplete => prompt.continuation = true,
                        InputStatus::Error(message) => {
                            eprintln!("error: {message}");
                            break;
                        }
                    }
                }
                Ok(_) => continue,
                Err(e) => {
                    eprintln!("error reading input: {e}");
                    break;
                }
            }
        }
    }
}
