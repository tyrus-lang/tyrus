use super::*;
use tyrus_core::analysis::Symbol;
use tyrus_core::ast::{Expr, ExprKind, ParamPattern, Program, Type, ValidatorDecorator};
use tyrus_core::collections::HashMap;
use tyrus_core::collections::HashSet;
use tyrus_core::doc_comments::{
    declaration_doc_anchor, doc_comment_before_decl, extract_doc_comments,
};
use tyrus_core::span::Span;
use tyrus_core::validators::{ValidatorInfo, validator_info, validator_infos, validator_message};
use tyrus_core::{ast::Item, lexer::Token, parser::Parser};

pub(super) fn completion_kind(kind: SymbolKind) -> CompletionItemKind {
    match kind {
        SymbolKind::Function => CompletionItemKind::FUNCTION,
        SymbolKind::Variable => CompletionItemKind::VARIABLE,
        SymbolKind::Parameter => CompletionItemKind::VALUE,
        SymbolKind::Import => CompletionItemKind::REFERENCE,
        SymbolKind::Interface => CompletionItemKind::INTERFACE,
        SymbolKind::TypeAlias => CompletionItemKind::STRUCT,
        SymbolKind::Property => CompletionItemKind::PROPERTY,
        SymbolKind::Enum => CompletionItemKind::ENUM,
        SymbolKind::EnumMember => CompletionItemKind::ENUM_MEMBER,
    }
}

fn enum_member_names(source: &str, name: &str) -> Option<Vec<String>> {
    let tokens = tyrus_core::lexer::lex(source).unwrap_or_default();
    let mut index = 0;
    while index < tokens.len() {
        let is_enum_header = matches!(&tokens[index], Token::Enum)
            && matches!(tokens.get(index + 1), Some(Token::Ident(n)) if n == name);
        if !is_enum_header {
            index += 1;
            continue;
        }
        let mut cursor = index + 2;
        while cursor < tokens.len() && !matches!(tokens[cursor], Token::LBrace) {
            cursor += 1;
        }
        cursor += 1;
        let mut members = Vec::new();
        let mut expect_name = true;
        while cursor < tokens.len() && !matches!(tokens[cursor], Token::RBrace) {
            match &tokens[cursor] {
                Token::Ident(member) if expect_name => {
                    members.push(member.clone());
                    expect_name = false;
                }
                Token::Comma => expect_name = true,
                _ => {}
            }
            cursor += 1;
        }
        return Some(members);
    }
    None
}

pub(super) fn style_text_completion_items(
    source: &str,
    offset: usize,
) -> Option<Vec<CompletionItem>> {
    let needs_quotes = style_text_style_context(source, offset)?;
    Some(
        tyrus_core::style_text::STYLE_NAMES
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let insert = if needs_quotes {
                    format!("\"{name}\"")
                } else {
                    name.to_string()
                };
                CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                    detail: Some("text style".to_string()),
                    insert_text: Some(insert),
                    sort_text: Some(format!("0_style_{index:03}")),
                    ..CompletionItem::default()
                }
            })
            .collect(),
    )
}

pub(super) fn figlet_font_completion_items(
    source: &str,
    offset: usize,
) -> Option<Vec<CompletionItem>> {
    let needs_quotes = figlet_font_context(source, offset)?;
    Some(
        tyrus_core::figlet::FONT_NAMES
            .iter()
            .enumerate()
            .map(|(index, name)| {
                let insert = if needs_quotes {
                    format!("\"{name}\"")
                } else {
                    name.to_string()
                };
                CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                    detail: Some("figlet font".to_string()),
                    insert_text: Some(insert),
                    sort_text: Some(format!("0_figlet_font_{index:03}")),
                    ..CompletionItem::default()
                }
            })
            .collect(),
    )
}

const RESPONSE_HEADER_NAMES: &[(&str, &str, &str)] = &[
    (
        "x-content-type-options",
        "nosniff",
        "Stops content-type sniffing, which is what turns a JSON response into executed script.",
    ),
    (
        "referrer-policy",
        "no-referrer",
        "Keeps request paths out of the `Referer` sent to whatever the response links to.",
    ),
    (
        "x-frame-options",
        "DENY",
        "Refuses framing, so the response cannot be the invisible layer of a clickjacking page.",
    ),
    (
        "cross-origin-resource-policy",
        "same-origin",
        "Refuses embedding by another origin: the modern form of the same refusal.",
    ),
    (
        "content-security-policy",
        "",
        "What a page may load and run. The value is the application's to write: a policy generic enough for every application either protects nothing or breaks the page.",
    ),
    (
        "strict-transport-security",
        "max-age=31536000",
        "A promise that this origin is reachable over TLS. Send it only from behind TLS: over plain HTTP it asks a browser to refuse the connection that works.",
    ),
    (
        "cache-control",
        "no-store",
        "How caches may keep the response. `no-store` is what a response carrying anything private needs.",
    ),
    (
        "content-type",
        "application/json; charset=utf-8",
        "The media type of the response body.",
    ),
    (
        "content-disposition",
        "attachment; filename=\"report.csv\"",
        "Whether the body is shown or downloaded, and under what filename.",
    ),
    (
        "retry-after",
        "60",
        "How long the client should wait before trying again.",
    ),
    (
        "location",
        "/",
        "Where a redirect points, or where a created resource now lives.",
    ),
    (
        "etag",
        "",
        "A version tag a later request can revalidate against.",
    ),
    (
        "vary",
        "accept-encoding",
        "Which request headers change this response, so a cache does not serve one variant for another.",
    ),
];

pub(super) fn response_header_completion_items(
    source: &str,
    offset: usize,
) -> Option<Vec<CompletionItem>> {
    let needs_quotes = response_header_context(source, offset)?;
    Some(
        RESPONSE_HEADER_NAMES
            .iter()
            .enumerate()
            .map(|(index, (name, value, documentation))| {
                let insert = match (needs_quotes, value.is_empty()) {
                    (true, true) => format!("\"{name}\""),
                    (true, false) => format!("\"{name}\", \"{value}\""),
                    (false, _) => (*name).to_string(),
                };
                CompletionItem {
                    label: (*name).to_string(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    detail: Some(if value.is_empty() {
                        "response header".to_string()
                    } else {
                        format!("response header: {value}")
                    }),
                    documentation: Some(Documentation::String((*documentation).to_string())),
                    insert_text: Some(insert),
                    sort_text: Some(format!("0_response_header_{index:03}")),
                    ..CompletionItem::default()
                }
            })
            .collect(),
    )
}

fn response_header_context(source: &str, offset: usize) -> Option<bool> {
    let prefix = source.get(..offset)?;
    let rest = ["header(", "appendHeader(", "removeHeader("]
        .iter()
        .filter_map(|method| prefix.rsplit_once(method).map(|(_, rest)| rest))
        .min_by_key(|rest| rest.len())?;
    if rest.contains(')') || top_level_comma(rest).is_some() {
        return None;
    }
    let odd_quotes = rest.matches(['"', '\'']).count() % 2 == 1;
    Some(!odd_quotes)
}

fn figlet_font_context(source: &str, offset: usize) -> Option<bool> {
    let prefix = source.get(..offset)?;
    let (_, rest) = prefix.rsplit_once("figlet(")?;
    let comma = top_level_comma(rest)?;
    let after_comma = &rest[comma + 1..];
    if after_comma.contains(')') {
        return None;
    }
    let odd_quotes = after_comma.matches(['"', '\'']).count() % 2 == 1;
    Some(!odd_quotes)
}

fn top_level_comma(text: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut depth = 0_i32;
    for (index, character) in text.char_indices() {
        match quote {
            Some(open) => {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == open {
                    quote = None;
                }
            }
            None if character == '"' || character == '\'' => quote = Some(character),
            None if matches!(character, '(' | '[' | '{') => depth += 1,
            None if matches!(character, ')' | ']' | '}') => depth -= 1,
            None if character == ',' && depth == 0 => return Some(index),
            None => {}
        }
    }
    None
}

fn style_text_style_context(source: &str, offset: usize) -> Option<bool> {
    let prefix = source.get(..offset)?;
    let without_partial = prefix.trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_');
    if let Some(chain) = without_partial.strip_suffix('.') {
        let chain = trailing_dotted_chain(chain);
        if chain == "styleText" || chain.starts_with("styleText.") {
            return Some(false);
        }
    }
    let (_, rest) = prefix.rsplit_once("styleText(")?;
    let odd_quotes = rest.matches(['"', '\'']).count() % 2 == 1;
    if rest.trim_start().starts_with('[') {
        if rest.contains(']') {
            return None;
        }
        return Some(!odd_quotes);
    }
    if rest.contains(',') {
        return None;
    }
    if odd_quotes {
        return Some(false);
    }
    rest.trim_start().is_empty().then_some(true)
}

pub(super) fn in_string_literal(source: &str, offset: usize) -> bool {
    let Some(prefix) = source.get(..offset) else {
        return false;
    };
    let line = prefix.rsplit('\n').next().unwrap_or(prefix);
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for character in line.chars() {
        match quote {
            Some(open) => {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == open {
                    quote = None;
                }
            }
            None if character == '"' || character == '\'' => quote = Some(character),
            None => {}
        }
    }
    quote.is_some()
}

fn trailing_dotted_chain(text: &str) -> &str {
    let start = text
        .char_indices()
        .rev()
        .find(|(_, character)| {
            !(character.is_ascii_alphanumeric() || *character == '_' || *character == '.')
        })
        .map_or(0, |(index, character)| index + character.len_utf8());
    &text[start..]
}

pub(super) fn member_completion_receiver(source: &str, offset: usize) -> Option<String> {
    let prefix = source.get(..offset)?.trim_end();
    let receiver = prefix
        .strip_suffix("?.")
        .or_else(|| prefix.strip_suffix('.'))?
        .trim_end();
    let start = receiver
        .char_indices()
        .rev()
        .find(|(_, character)| !character.is_ascii_alphanumeric() && *character != '_')
        .map_or(0, |(index, character)| index + character.len_utf8());
    let receiver = &receiver[start..];
    (!receiver.is_empty()).then(|| receiver.to_string())
}

#[cfg(test)]
pub(super) fn member_completion_items(
    source: &str,
    offset: usize,
    analysis: &DocumentAnalysis,
) -> Option<Vec<CompletionItem>> {
    member_completion_items_with_behaviors(source, offset, analysis, &HashMap::default())
}

pub(super) fn member_completion_items_with_behaviors(
    source: &str,
    offset: usize,
    analysis: &DocumentAnalysis,
    import_behaviors: &HashMap<String, tyrus_core::ast::BehaviorInterfaceInfo>,
) -> Option<Vec<CompletionItem>> {
    if let Some(items) = test_framework_completion_items(source, offset) {
        return Some(items);
    }
    let receiver = member_completion_receiver(source, offset)?;
    if receiver == "console" {
        return Some(console_completion_items());
    }
    if let Some(members) = enum_member_names(source, &receiver) {
        return Some(
            members
                .into_iter()
                .map(|member| CompletionItem {
                    detail: Some(format!("({receiver}) {member}")),
                    label: member,
                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                    ..CompletionItem::default()
                })
                .collect(),
        );
    }
    for import in scan_imports(source) {
        for (imported, local) in import.specifiers {
            if local == receiver {
                if let Some(members) = native_enum_member_names(&import.source, &imported) {
                    return Some(
                        members
                            .into_iter()
                            .map(|member| CompletionItem {
                                detail: Some(format!("({receiver}) {member}")),
                                label: member,
                                kind: Some(CompletionItemKind::ENUM_MEMBER),
                                ..CompletionItem::default()
                            })
                            .collect(),
                    );
                }
                if let Some(members) = native_static_member_names(&import.source, &imported) {
                    return Some(native_static_completion_items(
                        receiver.as_str(),
                        members,
                        Some((&import.source, &imported)),
                    ));
                }
            }
        }
    }
    if let Some(symbol) = analysis
        .symbols
        .iter()
        .find(|symbol| symbol.name == receiver && symbol.kind == SymbolKind::Import)
    {
        if let Some(members) = symbol
            .import_source
            .as_deref()
            .zip(symbol.imported_name.as_deref())
            .and_then(|(module, imported)| native_enum_member_names(module, imported))
        {
            return Some(
                members
                    .into_iter()
                    .map(|member| CompletionItem {
                        detail: Some(format!("({receiver}) {member}")),
                        label: member,
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        ..CompletionItem::default()
                    })
                    .collect(),
            );
        }
        if let Some((module, imported)) = symbol
            .import_source
            .as_deref()
            .zip(symbol.imported_name.as_deref())
            && let Some(members) = native_static_member_names(module, imported)
        {
            return Some(native_static_completion_items(
                receiver.as_str(),
                members,
                Some((module, imported)),
            ));
        }
    }
    let declared_type = declared_receiver_type(source, offset, &receiver);
    if receiver == "Array"
        && !analysis.symbols.iter().any(|symbol| symbol.name == "Array")
        && declared_type.is_none()
    {
        return Some(array_static_completion_items());
    }
    if receiver == "Promise"
        && !analysis
            .symbols
            .iter()
            .any(|symbol| symbol.name == "Promise")
        && declared_type.is_none()
    {
        return Some(promise_static_completion_items());
    }
    if let Some(items) = automatic_data_static_completion_items(source, offset, &receiver) {
        return Some(items);
    }
    let receiver_type = analysis
        .symbols
        .iter()
        .find(|symbol| symbol.name == receiver)
        .and_then(|symbol| symbol.semantic_type.clone())
        .or(declared_type);
    let object_fields = receiver_type
        .as_ref()
        .and_then(|ty| object_fields_for_type(analysis, ty))
        .or_else(|| {
            let prefix = source.get(..offset)?.trim_end();
            let prefix = prefix
                .strip_suffix("?.")
                .or_else(|| prefix.strip_suffix('.'))?;
            object_fields_for_receiver(&analyze(prefix), &receiver)
        });
    let mut items = object_fields
        .as_ref()
        .map(|fields| object_member_completion_items(fields))
        .or_else(|| {
            receiver_type
                .as_ref()
                .and_then(member_completion_items_for_type)
        })
        .unwrap_or_default();
    if let Some(behavior_items) =
        behavior_member_completion_items(source, offset, &receiver_type, import_behaviors)
    {
        items.extend(behavior_items);
    }
    for automatic in automatic_data_method_completion_items(source, offset, &receiver_type) {
        if !items.iter().any(|item| item.label == automatic.label) {
            items.push(automatic);
        }
    }
    Some(items)
}

fn strip_chain_modifiers(prefix: &str) -> (&str, Vec<&'static str>) {
    use tyrus_core::test::{NOT_MODIFIER, REJECTS_MODIFIER, RESOLVES_MODIFIER};
    let mut rest = prefix.trim_end();
    let mut used = Vec::new();
    loop {
        let Some(modifier) = [NOT_MODIFIER, REJECTS_MODIFIER, RESOLVES_MODIFIER]
            .into_iter()
            .find(|modifier| {
                rest.strip_suffix(modifier)
                    .is_some_and(|head| head.ends_with('.'))
            })
        else {
            return (rest, used);
        };
        used.push(modifier);
        rest = rest[..rest.len() - modifier.len() - 1].trim_end();
    }
}

fn test_framework_completion_items(source: &str, offset: usize) -> Option<Vec<CompletionItem>> {
    use tyrus_core::test::{
        MATCHERS, MODULE, NOT_MODIFIER, REJECTS_MODIFIER, RESOLVES_MODIFIER, TestModifier,
        chain_modifier_summary, matcher_signature, matcher_summary, modifier_help,
    };
    let prefix = source.get(..offset)?.trim_end();
    let (before_dot, partial) = if let Some(before_dot) = prefix.strip_suffix('.') {
        (before_dot.trim_end(), "")
    } else {
        let partial = trailing_identifier(prefix);
        let before_dot = prefix[..prefix.len() - partial.len()]
            .strip_suffix('.')?
            .trim_end();
        (before_dot, partial)
    };
    let specifiers: Vec<(String, String)> = scan_imports(source)
        .into_iter()
        .filter(|import| import.source == MODULE)
        .flat_map(|import| import.specifiers)
        .collect();
    if specifiers.is_empty() {
        return None;
    }
    let imported_as = |local: &str| {
        specifiers
            .iter()
            .find(|(_, candidate)| candidate == local)
            .map(|(imported, _)| imported.clone())
    };

    let (chain, used) = strip_chain_modifiers(before_dot);
    if chain.ends_with(')') {
        let open = final_call_open_paren(chain)?;
        let callee = trailing_identifier(&chain[..open]);
        if imported_as(callee).as_deref() != Some("expect") {
            return None;
        }
        let mut items: Vec<CompletionItem> = MATCHERS
            .iter()
            .map(|def| CompletionItem {
                label: def.name.to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some(format!("(matcher) {}", matcher_signature(def))),
                documentation: Some(Documentation::String(matcher_summary(def.name).to_string())),
                ..CompletionItem::default()
            })
            .collect();
        let settled = used
            .iter()
            .any(|modifier| *modifier == REJECTS_MODIFIER || *modifier == RESOLVES_MODIFIER);
        for modifier in [NOT_MODIFIER, REJECTS_MODIFIER, RESOLVES_MODIFIER] {
            let already = used.contains(&modifier);
            if already || (settled && modifier != NOT_MODIFIER) {
                continue;
            }
            items.push(CompletionItem {
                label: modifier.to_string(),
                kind: Some(CompletionItemKind::PROPERTY),
                detail: Some(format!("(modifier) {modifier}")),
                documentation: chain_modifier_summary(modifier)
                    .map(|summary| Documentation::String(summary.to_string())),
                ..CompletionItem::default()
            });
        }
        return Some(items);
    }

    let receiver = trailing_identifier(before_dot);
    if receiver.is_empty()
        || receiver.len()
            != before_dot.len()
                - before_dot
                    .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || c == '_')
                    .len()
    {
        return None;
    }
    let modifiers: &[TestModifier] = match imported_as(receiver).as_deref() {
        Some("test" | "it") => &[
            TestModifier::Skip,
            TestModifier::Only,
            TestModifier::Todo,
            TestModifier::Each,
        ],
        Some("describe") => &[TestModifier::Skip, TestModifier::Only],
        _ => return None,
    };
    Some(
        modifiers
            .iter()
            .filter(|modifier| {
                modifier_help(receiver, **modifier)
                    .0
                    .split('.')
                    .nth(1)
                    .and_then(|rest| rest.split('(').next())
                    .is_some_and(|name| name.starts_with(partial))
            })
            .map(|modifier| {
                let (signature, summary) = modifier_help(receiver, *modifier);
                let label = signature
                    .split('.')
                    .nth(1)
                    .and_then(|rest| rest.split('(').next())
                    .unwrap_or_default()
                    .to_string();
                CompletionItem {
                    label,
                    kind: Some(CompletionItemKind::METHOD),
                    detail: Some(format!("(modifier) {signature}")),
                    documentation: Some(Documentation::String(summary.to_string())),
                    ..CompletionItem::default()
                }
            })
            .collect(),
    )
}

fn identifier_at(source: &str, offset: usize) -> Option<(std::ops::Range<usize>, &str)> {
    let is_word = |character: char| character.is_ascii_alphanumeric() || character == '_';
    if offset > source.len() {
        return None;
    }
    let start = source[..offset]
        .char_indices()
        .rev()
        .find(|(_, character)| !is_word(*character))
        .map_or(0, |(index, character)| index + character.len_utf8());
    let end = source[offset..]
        .char_indices()
        .find(|(_, character)| !is_word(*character))
        .map_or(source.len(), |(index, _)| offset + index);
    (start < end).then(|| (start..end, &source[start..end]))
}

pub(super) fn test_framework_hover_at(
    source: &str,
    offset: usize,
) -> Option<(std::ops::Range<usize>, String)> {
    use tyrus_core::test::{
        DESCRIBE_TARGET, IT_TARGET, MODULE, TEST_TARGET, chain_modifier_summary, matcher_def,
        matcher_signature, matcher_summary, modifier_help, test_modifier,
    };
    let (range, member) = identifier_at(source, offset)?;
    let before = source[..range.start].trim_end();
    let before = before.strip_suffix('.')?.trim_end();
    let specifiers: Vec<(String, String)> = scan_imports(source)
        .into_iter()
        .filter(|import| import.source == MODULE)
        .flat_map(|import| import.specifiers)
        .collect();
    if specifiers.is_empty() {
        return None;
    }
    let imported_as = |local: &str| {
        specifiers
            .iter()
            .find(|(_, candidate)| candidate == local)
            .map(|(imported, _)| imported.clone())
    };

    let receiver = trailing_identifier(before);
    if let Some(imported) = imported_as(receiver) {
        let target = match imported.as_str() {
            "describe" => DESCRIBE_TARGET,
            "it" => IT_TARGET,
            "test" => TEST_TARGET,
            _ => "",
        };
        if let Some(modifier) = test_modifier(target, member) {
            let (signature, summary) = modifier_help(&imported, modifier);
            return Some((
                range,
                format!("```tyrus\n{signature}\n```\n\nTest modifier. {summary}"),
            ));
        }
    }

    let (chain, _) = strip_chain_modifiers(before);
    if !chain.ends_with(')') {
        return None;
    }
    let open = final_call_open_paren(chain)?;
    if imported_as(trailing_identifier(&chain[..open])).as_deref() != Some("expect") {
        return None;
    }
    if let Some(summary) = chain_modifier_summary(member) {
        return Some((
            range,
            format!("```tyrus\n{member}\n```\n\nTest modifier. {summary}"),
        ));
    }
    let matcher = matcher_def(member)?;
    Some((
        range,
        format!(
            "```tyrus\n{}\n```\n\nTest matcher. {}",
            matcher_signature(matcher),
            matcher_summary(matcher.name)
        ),
    ))
}

fn final_call_open_paren(text: &str) -> Option<usize> {
    let mut stack = Vec::new();
    let mut last_open = None;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, character) in text.char_indices() {
        if let Some(open_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == open_quote {
                quote = None;
            }
            continue;
        }
        match character {
            '"' | '\'' | '`' => quote = Some(character),
            '(' => stack.push(index),
            ')' => last_open = stack.pop(),
            _ => {}
        }
    }
    last_open
}

fn trailing_identifier(text: &str) -> &str {
    let text = text.trim_end();
    let start = text
        .char_indices()
        .rev()
        .find(|(_, character)| !character.is_ascii_alphanumeric() && *character != '_')
        .map_or(0, |(index, character)| index + character.len_utf8());
    &text[start..]
}

/// A completion request cuts the file at the cursor, so the statement being typed
/// has no `;` yet and the parser, which requires one, would refuse the whole
/// prefix. Closing it for the parse alone keeps every declaration before the
/// cursor available.
fn terminated_for_parse(prefix: &str) -> String {
    let trimmed = prefix.trim_end();
    if trimmed.is_empty()
        || trimmed.ends_with(';')
        || trimmed.ends_with('}')
        || trimmed.ends_with('{')
    {
        return trimmed.to_string();
    }
    format!("{trimmed};")
}

fn automatic_data_static_completion_items(
    source: &str,
    offset: usize,
    receiver: &str,
) -> Option<Vec<CompletionItem>> {
    let prefix = source.get(..offset)?.trim_end();
    let parse_source = prefix
        .strip_suffix("?.")
        .or_else(|| prefix.strip_suffix('.'))
        .unwrap_or(prefix);
    let parse_source = terminated_for_parse(parse_source);
    let parse_source = parse_source.as_str();
    let Ok(mut parser) = Parser::from_source(parse_source) else {
        return None;
    };
    let Ok(program) = parser.parse_program() else {
        return None;
    };
    let mut interfaces = HashMap::default();
    let mut data_interfaces = HashSet::default();
    for item in &program.items {
        let Item::InterfaceDecl {
            name,
            fields,
            from_type_alias,
            ..
        } = item
        else {
            continue;
        };
        if !from_type_alias {
            data_interfaces.insert(name.clone());
            interfaces.insert(name.clone(), fields.clone());
        }
    }
    if !data_interfaces.contains(receiver) {
        return None;
    }
    let ty = Type::Named(receiver.to_string());
    if !tyrus_core::automatic_data_methods::supports_json(&ty, &interfaces, &data_interfaces) {
        return Some(Vec::new());
    }
    Some(vec![CompletionItem {
        label: "fromJson".to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(format!(
            "({receiver}) fromJson(text: string): Result<{receiver}, JsonError>"
        )),
        documentation: Some(Documentation::String(
            "Automatic data method: validates JSON and decodes a new value of this interface."
                .to_string(),
        )),
        insert_text: Some("fromJson(${1:text})".to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some("2_automatic_fromJson".to_string()),
        ..CompletionItem::default()
    }])
}

fn automatic_data_method_completion_items(
    source: &str,
    offset: usize,
    receiver_type: &Option<Type>,
) -> Vec<CompletionItem> {
    let Some(Type::Named(receiver)) = receiver_type else {
        return Vec::new();
    };
    let prefix = source.get(..offset).unwrap_or(source).trim_end();
    let parse_source = prefix
        .strip_suffix("?.")
        .or_else(|| prefix.strip_suffix('.'))
        .unwrap_or(prefix);
    let parse_source = terminated_for_parse(parse_source);
    let parse_source = parse_source.as_str();
    let Ok(mut parser) = Parser::from_source(parse_source) else {
        return Vec::new();
    };
    let Ok(program) = parser.parse_program() else {
        return Vec::new();
    };
    let mut interfaces = HashMap::default();
    let mut data_interfaces = HashSet::default();
    for item in &program.items {
        let Item::InterfaceDecl {
            name,
            fields,
            from_type_alias,
            ..
        } = item
        else {
            continue;
        };
        if !from_type_alias {
            data_interfaces.insert(name.clone());
            interfaces.insert(name.clone(), fields.clone());
        }
    }
    let ty = Type::Named(receiver.clone());
    let mut items = Vec::new();
    if tyrus_core::automatic_data_methods::supports_copy(&ty, &interfaces, &data_interfaces) {
        items.push(automatic_data_method_completion(
            receiver,
            "copy",
            &format!("copy(): {receiver}"),
            "Returns an independent structural copy of this value.",
        ));
    }
    if tyrus_core::automatic_data_methods::supports_debug(&ty, &interfaces, &data_interfaces) {
        items.push(automatic_data_method_completion(
            receiver,
            "toString",
            "toString(): string",
            "Returns a readable representation of this value.",
        ));
    }
    if tyrus_core::automatic_data_methods::supports_eq(&ty, &interfaces, &data_interfaces) {
        items.push(automatic_data_method_completion(
            receiver,
            "equals",
            &format!("equals(other: {receiver}): boolean"),
            "Compares this value structurally with another value of the same type.",
        ));
    }
    if tyrus_core::automatic_data_methods::supports_json_encode(&ty, &interfaces, &data_interfaces)
    {
        items.push(automatic_data_method_completion(
            receiver,
            "toJson",
            "toJson(options?: { pretty: boolean }): string",
            "Encodes this value as canonical JSON, with optional four-space indentation.",
        ));
    }
    items
}

fn automatic_data_method_completion(
    receiver: &str,
    name: &str,
    signature: &str,
    documentation: &str,
) -> CompletionItem {
    CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(format!("({receiver}) {signature}")),
        documentation: Some(Documentation::String(format!(
            "Automatic data method: {documentation}"
        ))),
        insert_text: Some(match name {
            "equals" => "equals(${1:other})".to_string(),
            _ => format!("{name}()"),
        }),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("2_automatic_{name}")),
        ..CompletionItem::default()
    }
}

fn behavior_member_completion_items(
    source: &str,
    offset: usize,
    receiver_type: &Option<Type>,
    import_behaviors: &HashMap<String, tyrus_core::ast::BehaviorInterfaceInfo>,
) -> Option<Vec<CompletionItem>> {
    let receiver_type = receiver_type.as_ref()?;
    let receiver_name = receiver_type.to_string();
    let prefix = source.get(..offset)?.trim_end();
    let parse_source = prefix
        .strip_suffix("?.")
        .or_else(|| prefix.strip_suffix('.'))
        .unwrap_or(prefix);
    let parse_source = terminated_for_parse(parse_source);
    let parse_source = parse_source.as_str();
    let program = Parser::from_source(parse_source)
        .ok()?
        .parse_program()
        .ok()?;
    let mut behaviors = HashMap::default();
    for item in &program.items {
        if let Item::BehaviorInterfaceDecl { name, .. } = item
            && let Some(behavior) =
                tyrus_core::module_loader::behavior_interface_info(item, &program.items)
        {
            behaviors.insert(
                name.clone(),
                (
                    behavior.targets,
                    behavior.extends,
                    behavior.methods,
                    behavior.binding_methods,
                ),
            );
        }
    }
    for (name, behavior) in import_behaviors {
        behaviors.insert(
            name.clone(),
            (
                behavior.targets.clone(),
                behavior.extends.clone(),
                behavior.methods.clone(),
                behavior.binding_methods.clone(),
            ),
        );
    }
    let mut matching = Vec::new();
    let mut pending = behaviors
        .iter()
        .filter(|(_, (targets, _, _, _))| targets.iter().any(|target| target == receiver_type))
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    while let Some(name) = pending.pop() {
        if matching.contains(&name) {
            continue;
        }
        matching.push(name.clone());
        for (child, (_, extends, _, _)) in &behaviors {
            if extends
                .iter()
                .any(|ty| matches!(ty, Type::Named(parent) if parent == &name))
            {
                pending.push(child.clone());
            }
        }
    }
    let mut items = Vec::new();
    for name in matching {
        let Some((_, _, methods, binding_methods)) = behaviors.get(&name) else {
            continue;
        };
        for method in methods.iter().chain(binding_methods) {
            let parameters = method
                .params
                .iter()
                .skip(1)
                .map(|param| {
                    let name = match &param.pattern {
                        tyrus_core::ast::ParamPattern::Name(name) => name.as_str(),
                        _ => "value",
                    };
                    format!("{name}: {}", param.ty)
                })
                .collect::<Vec<_>>();
            items.push(CompletionItem {
                label: method.name.clone(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some(format!(
                    "({receiver_name}) {}({}): {}",
                    method.name,
                    parameters.join(", "),
                    method.ret_type
                )),
                insert_text: Some(format!("{}($0)", method.name)),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..CompletionItem::default()
            });
        }
    }
    Some(items)
}

fn native_static_completion_items(
    receiver: &str,
    members: Vec<(String, String)>,
    native: Option<(&str, &str)>,
) -> Vec<CompletionItem> {
    members
        .into_iter()
        .map(|(member, detail)| {
            let documentation = native
                .and_then(|(module, namespace)| {
                    (module == "tyrus:crypto" || module == "tyrus:encoding")
                        .then(|| native_namespace_member_documentation(module, namespace, &member))
                        .flatten()
                })
                .map(|value| Documentation::String(value.to_string()));
            CompletionItem {
                detail: Some(format!("({receiver}) {detail}")),
                documentation,
                label: member,
                kind: Some(CompletionItemKind::METHOD),
                ..CompletionItem::default()
            }
        })
        .collect()
}

pub(super) fn array_static_completion_items() -> Vec<CompletionItem> {
    vec![CompletionItem {
        label: "isArray".to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some("(value) => boolean".to_string()),
        insert_text: Some("isArray(${1:value})".to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..CompletionItem::default()
    }]
}

pub(super) fn promise_static_completion_items() -> Vec<CompletionItem> {
    [
        (
            "resolve",
            "<T>(value: T) => Promise<T>",
            "resolve(${1:value})",
        ),
        (
            "reject",
            "<T>(reason: Error) => Promise<T>",
            "reject(${1:reason})",
        ),
        (
            "all",
            "<T>(values: Promise<T>[]) => Promise<T[]>",
            "all(${1:values})",
        ),
        (
            "allSettled",
            "<T>(values: Promise<T>[]) => Promise<PromiseSettledResult<T>[]>",
            "allSettled(${1:values})",
        ),
        (
            "any",
            "<T>(values: Promise<T>[]) => Promise<T>",
            "any(${1:values})",
        ),
        (
            "race",
            "<T>(values: Promise<T>[]) => Promise<T>",
            "race(${1:values})",
        ),
        (
            "withResolvers",
            "<T>() => PromiseWithResolvers<T>",
            "withResolvers<${1:T}>()",
        ),
    ]
    .into_iter()
    .map(|(label, detail, insert_text)| CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(detail.to_string()),
        documentation: Some(Documentation::String(
            "Native Promise API. Availability is gated by the documented language subset."
                .to_string(),
        )),
        insert_text: Some(insert_text.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_promise_static_{label}")),
        ..CompletionItem::default()
    })
    .collect()
}

pub(super) fn console_completion_items() -> Vec<CompletionItem> {
    tyrus_core::completion_common::CONSOLE_MEMBERS
        .iter()
        .map(|member| CompletionItem {
            label: member.name.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(member.description.to_string()),
            insert_text: Some(format!("{}($0);", member.name)),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..CompletionItem::default()
        })
        .collect()
}

pub(super) fn member_completion_items_for_type(ty: &Type) -> Option<Vec<CompletionItem>> {
    match ty {
        Type::Object(fields) => Some(object_member_completion_items(fields)),
        Type::Named(name) if name == "HttpRouter" => Some(http_router_completion_items()),
        Type::Array(_) => Some(array_completion_items()),
        Type::Tuple(elements) => tyrus_core::type_checker::tuple_sequence(elements)
            .is_uniform()
            .then(non_mutating_array_completion_items),
        Type::Readonly(inner) => match inner.as_ref() {
            Type::Array(_) => Some(non_mutating_array_completion_items()),
            other => member_completion_items_for_type(other),
        },
        Type::Map(_, _) => Some(map_completion_items()),
        Type::Set(_) => Some(set_completion_items()),
        Type::BinaryHeap(_) => Some(binary_heap_completion_items()),
        Type::BinomialHeap(_) => Some(binomial_heap_completion_items()),
        Type::UnionFind(_) => Some(union_find_completion_items()),
        Type::Counter(_) => Some(counter_completion_items()),
        Type::Deque(_) => Some(deque_completion_items()),
        Type::FenwickTree => Some(fenwick_tree_completion_items()),
        Type::SegmentTree => Some(segment_tree_completion_items()),
        Type::SparseTable => Some(sparse_table_completion_items()),
        Type::SegmentTreeLazy => Some(segment_tree_lazy_completion_items()),
        Type::BitSet => Some(bit_set_completion_items()),
        Type::LinkedList(_) => Some(linked_list_completion_items()),
        Type::DoubleLinkedList(_) => Some(double_linked_list_completion_items()),
        Type::AvlTree(_) => Some(avl_tree_completion_items()),
        Type::RedBlackTree(_) => Some(red_black_tree_completion_items()),
        Type::SplayTree(_) => Some(splay_tree_completion_items()),
        Type::Treap(_) => Some(treap_completion_items()),
        Type::SortedSet(_) => Some(sorted_set_completion_items()),
        Type::SkipList(_) => Some(skip_list_completion_items()),
        Type::SortedMap(_, _) => Some(sorted_map_completion_items()),
        Type::Trie => Some(trie_completion_items()),
        Type::BloomFilter => Some(bloom_filter_completion_items()),
        Type::SuffixTree => Some(suffix_tree_completion_items()),
        Type::Lca => Some(lca_completion_items()),
        Type::Hld => Some(hld_completion_items()),
        Type::Centroid => Some(centroid_completion_items()),
        Type::Str => Some(string_completion_items()),
        Type::Char => Some(char_completion_items()),
        Type::Regex => Some(regex_completion_items()),
        Type::Apply { name, args } if name == "Promise" && args.len() == 1 => {
            Some(promise_completion_items(&args[0]))
        }
        Type::Apply { name, args } if name == "JoinHandle" && args.len() == 1 => {
            Some(join_handle_completion_items(&args[0]))
        }
        Type::Apply { name, args } if name == "Sender" && args.len() == 1 => {
            Some(sender_completion_items(&args[0]))
        }
        Type::Apply { name, args } if name == "Receiver" && args.len() == 1 => {
            Some(receiver_completion_items(&args[0]))
        }
        Type::Apply { name, args } if name == "ReadableStream" && args.len() == 1 => {
            Some(readable_stream_completion_items(&args[0]))
        }
        Type::Apply { name, args } if name == "WritableStream" && args.len() == 1 => {
            Some(writable_stream_completion_items(&args[0]))
        }
        Type::Apply { name, args }
            if matches!(name.as_str(), "DuplexStream" | "TransformStream") && args.len() == 2 =>
        {
            Some(composite_stream_completion_items(name, &args[0], &args[1]))
        }
        Type::Apply { name, args } if name == "Mutex" && args.len() == 1 => {
            Some(mutex_completion_items(&args[0]))
        }
        Type::Apply { name, args } if name == "MutexGuard" && args.len() == 1 => {
            Some(mutex_guard_completion_items(&args[0]))
        }
        Type::Apply { name, args } if name == "RwLock" && args.len() == 1 => {
            Some(rwlock_completion_items(&args[0]))
        }
        Type::Apply { name, args } if name == "RwReadGuard" && args.len() == 1 => {
            Some(rwlock_read_guard_completion_items(&args[0]))
        }
        Type::Apply { name, args } if name == "RwWriteGuard" && args.len() == 1 => {
            Some(rwlock_write_guard_completion_items(&args[0]))
        }
        Type::Named(name) if name == "Semaphore" => Some(semaphore_completion_items()),
        Type::Named(name) if name == "WebSocket" => Some(websocket_completion_items()),
        Type::Lit(expr) if matches!(expr.kind(), tyrus_core::ast::ExprKind::StrLit(_)) => {
            Some(string_completion_items())
        }
        _ => None,
    }
}

pub(super) fn object_fields_for_receiver(
    analysis: &DocumentAnalysis,
    receiver: &str,
) -> Option<Vec<Field>> {
    let ty = analysis.symbol_named(receiver)?.semantic_type.as_ref()?;
    object_fields_for_type(analysis, ty)
}

fn object_fields_for_type(analysis: &DocumentAnalysis, ty: &Type) -> Option<Vec<Field>> {
    match ty {
        Type::Object(fields) => Some(fields.clone()),
        Type::Readonly(inner) => object_fields_for_type(analysis, inner),
        Type::Named(name) => analysis
            .symbol_named(name)
            .and_then(|symbol| symbol.semantic_type.as_ref())
            .and_then(|target| match target {
                Type::Named(target_name) if target_name == name => None,
                target => object_fields_for_type(analysis, target),
            }),
        _ => None,
    }
}

pub(super) fn object_member_completion_items(fields: &[Field]) -> Vec<CompletionItem> {
    let mut items = fields
        .iter()
        .map(|field| {
            let optional = if field.optional { "?" } else { "" };
            let readonly = if field.readonly { "readonly " } else { "" };
            CompletionItem {
                label: field.name.clone(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some(format!("{readonly}{}{optional}", field.ty)),
                sort_text: Some(format!("0_field_{}", field.name)),
                ..CompletionItem::default()
            }
        })
        .collect::<Vec<_>>();
    let keys = fields
        .iter()
        .map(|field| format!("\"{}\"", field.name))
        .collect::<Vec<_>>()
        .join(" | ");
    for (method, description) in [
        (
            "pick",
            "Returns a new structural object containing only the selected static keys.",
        ),
        (
            "omit",
            "Returns a new structural object without the selected static keys.",
        ),
    ] {
        items.push(CompletionItem {
            label: method.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(format!("(keys: ({keys})[]) => object")),
            documentation: Some(Documentation::String(description.to_string())),
            insert_text: Some(format!(
                "{method}([${{1:\"{}\"}}])",
                fields.first().map_or("key", |field| field.name.as_str())
            )),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            sort_text: Some(format!("1_object_projection_{method}")),
            ..CompletionItem::default()
        });
    }
    items
}

pub(super) fn object_projection_key_completion_receiver(
    source: &str,
    offset: usize,
) -> Option<String> {
    let prefix = source.get(..offset)?;
    for method in ["pick", "omit"] {
        let marker = format!(".{method}([");
        let Some((receiver, argument)) = prefix.rsplit_once(&marker) else {
            continue;
        };
        if argument.contains("])") {
            continue;
        }
        let receiver = receiver.trim_end();
        let start = receiver
            .char_indices()
            .rev()
            .find(|(_, character)| !character.is_ascii_alphanumeric() && *character != '_')
            .map_or(0, |(index, character)| index + character.len_utf8());
        let receiver = &receiver[start..];
        if !receiver.is_empty() {
            return Some(receiver.to_string());
        }
    }
    None
}

pub(super) fn object_projection_type_key_completion_receiver(
    source: &str,
    offset: usize,
) -> Option<String> {
    let prefix = source.get(..offset)?;
    for utility in ["Pick", "Omit", "Optional"] {
        let marker = format!("{utility}<");
        let Some((_, arguments)) = prefix.rsplit_once(&marker) else {
            continue;
        };
        if arguments.contains('>') {
            continue;
        }
        let Some((source_type, _)) = arguments.split_once(',') else {
            continue;
        };
        let source_type = source_type.trim();
        if !source_type.is_empty()
            && source_type
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return Some(source_type.to_string());
        }
    }
    None
}

pub(super) fn object_projection_key_completion_items(
    fields: &[Field],
    quoted: bool,
) -> Vec<CompletionItem> {
    fields
        .iter()
        .map(|field| CompletionItem {
            label: field.name.clone(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some(field.ty.to_string()),
            documentation: Some(Documentation::String(
                "Static key for object projection.".to_string(),
            )),
            insert_text: Some(if quoted {
                field.name.clone()
            } else {
                format!("\"{}\"", field.name)
            }),
            sort_text: Some(format!("0_projection_key_{}", field.name)),
            ..CompletionItem::default()
        })
        .collect()
}

pub(super) fn promise_completion_items(payload: &Type) -> Vec<CompletionItem> {
    [
        (
            "then",
            format!("(onFulfilled: (value: {payload}) => U) => Promise<U>"),
            "then(${1:onFulfilled})",
        ),
        (
            "catch",
            "catch(onRejected: (reason: Error) => T) => Promise<T>".to_string(),
            "catch(${1:onRejected})",
        ),
        (
            "finally",
            "finally(onFinally: () => void) => Promise<T>".to_string(),
            "finally(${1:onFinally})",
        ),
    ]
    .into_iter()
    .map(|(label, detail, insert_text)| CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(detail),
        documentation: Some(Documentation::String(
            "Native Promise continuation. The callback is scheduled and never runs inline during settlement."
                .to_string(),
        )),
        insert_text: Some(insert_text.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_promise_instance_{label}")),
        ..CompletionItem::default()
    })
    .collect()
}

pub(super) fn join_handle_completion_items(payload: &Type) -> Vec<CompletionItem> {
    [
        (
            "join",
            format!("() => Promise<{payload}>"),
            "join()",
            "Resolves to the task's result once it completes. Consumes the handle, like Rust's own `JoinHandle::join(self)`, joining it again, or calling `abort()` afterward, is a compile error.",
        ),
        (
            "abort",
            "() => void".to_string(),
            "abort()",
            "Cooperatively cancels the task, reusing `tyrus:task.cancel`'s mechanism. Does not consume the handle, may be called any number of times before `join()`, which afterward observes the cancellation through its rejection.",
        ),
    ]
    .into_iter()
    .map(|(label, detail, insert_text, documentation)| CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(detail),
        documentation: Some(Documentation::String(documentation.to_string())),
        insert_text: Some(insert_text.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_join_handle_instance_{label}")),
        ..CompletionItem::default()
    })
    .collect()
}

pub(super) fn sender_completion_items(payload: &Type) -> Vec<CompletionItem> {
    [
        (
            "send",
            format!("(value: {payload}) => Promise<void>"),
            "send(${1:value})",
            "Waits for room (a bounded channel that is full) and delivers `value`. Rejects once every `Receiver` has been dropped.",
        ),
        (
            "trySend",
            format!("(value: {payload}) => boolean"),
            "trySend(${1:value})",
            "Never waits: `true` once `value` is accepted immediately, `false` if the channel is full or every `Receiver` has been dropped.",
        ),
    ]
    .into_iter()
    .map(|(label, detail, insert_text, documentation)| CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(detail),
        documentation: Some(Documentation::String(documentation.to_string())),
        insert_text: Some(insert_text.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_sender_instance_{label}")),
        ..CompletionItem::default()
    })
    .collect()
}

pub(super) fn receiver_completion_items(payload: &Type) -> Vec<CompletionItem> {
    [
        (
            "recv",
            format!("() => Promise<{payload}?>"),
            "recv()",
            "Waits for the next message, or resolves to `None` once every `Sender` has been dropped and no message remains buffered.",
        ),
        (
            "tryRecv",
            format!("() => {payload}?"),
            "tryRecv()",
            "Never waits: the next message if one is already buffered, or `None` if the channel is empty right now.",
        ),
        (
            "close",
            "() => void".to_string(),
            "close()",
            "Closes the channel: `send`/`trySend` start failing, and `recv`/`tryRecv` still drain whatever was already buffered before settling to `None`.",
        ),
    ]
    .into_iter()
    .map(|(label, detail, insert_text, documentation)| CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(detail),
        documentation: Some(Documentation::String(documentation.to_string())),
        insert_text: Some(insert_text.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_receiver_instance_{label}")),
        ..CompletionItem::default()
    })
    .collect()
}

pub(super) fn readable_stream_completion_items(payload: &Type) -> Vec<CompletionItem> {
    stream_completion_items(&[
        ("read", format!("() => Promise<{payload}?>"), "read()"),
        (
            "pipeTo",
            format!("(destination: WritableStream<{payload}>) => Promise<void>"),
            "pipeTo(${1:destination})",
        ),
        ("close", "() => Promise<void>".to_string(), "close()"),
    ])
}

pub(super) fn writable_stream_completion_items(payload: &Type) -> Vec<CompletionItem> {
    stream_completion_items(&[
        (
            "write",
            format!("(chunk: {payload}) => Promise<void>"),
            "write(${1:chunk})",
        ),
        ("close", "() => Promise<void>".to_string(), "close()"),
    ])
}

pub(super) fn composite_stream_completion_items(
    name: &str,
    first: &Type,
    second: &Type,
) -> Vec<CompletionItem> {
    let (readable, writable) = if name == "DuplexStream" {
        (first, second)
    } else {
        (second, first)
    };
    let mut items =
        stream_completion_items(&[("close", "() => Promise<void>".to_string(), "close()")]);
    items.push(CompletionItem {
        label: "readable".to_string(),
        kind: Some(CompletionItemKind::FIELD),
        detail: Some(format!("ReadableStream<{readable}>")),
        sort_text: Some("0_stream_field_readable".to_string()),
        ..CompletionItem::default()
    });
    items.push(CompletionItem {
        label: "writable".to_string(),
        kind: Some(CompletionItemKind::FIELD),
        detail: Some(format!("WritableStream<{writable}>")),
        sort_text: Some("0_stream_field_writable".to_string()),
        ..CompletionItem::default()
    });
    items
}

fn stream_completion_items(entries: &[(&str, String, &str)]) -> Vec<CompletionItem> {
    entries
        .iter()
        .map(|(label, detail, insert_text)| CompletionItem {
            label: (*label).to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(detail.clone()),
            documentation: Some(Documentation::String(
                "Uses the bounded native stream lifecycle and propagates backpressure, closure, cancellation, and structured errors.".to_string(),
            )),
            insert_text: Some((*insert_text).to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            sort_text: Some(format!("0_stream_instance_{label}")),
            ..CompletionItem::default()
        })
        .collect()
}

pub(super) fn mutex_completion_items(payload: &Type) -> Vec<CompletionItem> {
    [
        (
            "lock",
            format!("() => Promise<MutexGuard<{payload}>>"),
            "lock()",
            "Waits for the lock, then resolves with a guard giving exclusive access to the protected value through `guard.value`.",
        ),
        (
            "tryLock",
            format!("() => MutexGuard<{payload}>?"),
            "tryLock()",
            "Never waits: the acquired guard if the lock was free, or `None` if it is currently held elsewhere.",
        ),
    ]
    .into_iter()
    .map(|(label, detail, insert_text, documentation)| CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(detail),
        documentation: Some(Documentation::String(documentation.to_string())),
        insert_text: Some(insert_text.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_mutex_instance_{label}")),
        ..CompletionItem::default()
    })
    .collect()
}

pub(super) fn mutex_guard_completion_items(payload: &Type) -> Vec<CompletionItem> {
    vec![CompletionItem {
        label: "value".to_string(),
        kind: Some(CompletionItemKind::FIELD),
        detail: Some(payload.to_string()),
        documentation: Some(Documentation::String(
            "The `Mutex<T>` value this guard holds the lock for. Reads and writes go straight through to the protected storage; the lock releases automatically when the guard goes out of scope.".to_string(),
        )),
        sort_text: Some("0_mutex_guard_instance_value".to_string()),
        ..CompletionItem::default()
    }]
}

pub(super) fn rwlock_completion_items(payload: &Type) -> Vec<CompletionItem> {
    [
        (
            "read",
            format!("() => Promise<RwReadGuard<{payload}>>"),
            "read()",
            "Waits for read access, then resolves with a guard giving shared, read-only access to the protected value through `guard.value`. Multiple readers may hold the lock at once.",
        ),
        (
            "write",
            format!("() => Promise<RwWriteGuard<{payload}>>"),
            "write()",
            "Waits for exclusive write access, then resolves with a guard giving read-write access to the protected value through `guard.value`.",
        ),
        (
            "tryRead",
            format!("() => RwReadGuard<{payload}>?"),
            "tryRead()",
            "Never waits: the acquired read guard if a writer does not currently hold the lock, or `None` otherwise.",
        ),
        (
            "tryWrite",
            format!("() => RwWriteGuard<{payload}>?"),
            "tryWrite()",
            "Never waits: the acquired write guard if the lock is currently free, or `None` if a reader or writer already holds it.",
        ),
    ]
    .into_iter()
    .map(|(label, detail, insert_text, documentation)| CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(detail),
        documentation: Some(Documentation::String(documentation.to_string())),
        insert_text: Some(insert_text.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_rwlock_instance_{label}")),
        ..CompletionItem::default()
    })
    .collect()
}

pub(super) fn rwlock_read_guard_completion_items(payload: &Type) -> Vec<CompletionItem> {
    vec![CompletionItem {
        label: "value".to_string(),
        kind: Some(CompletionItemKind::FIELD),
        detail: Some(payload.to_string()),
        documentation: Some(Documentation::String(
            "The `RwLock<T>` value this guard holds a read lock for. Read-only: this guard cannot write through it. Reads go straight through to the protected storage; the lock releases automatically when the guard goes out of scope.".to_string(),
        )),
        sort_text: Some("0_rwlock_read_guard_instance_value".to_string()),
        ..CompletionItem::default()
    }]
}

pub(super) fn rwlock_write_guard_completion_items(payload: &Type) -> Vec<CompletionItem> {
    vec![CompletionItem {
        label: "value".to_string(),
        kind: Some(CompletionItemKind::FIELD),
        detail: Some(payload.to_string()),
        documentation: Some(Documentation::String(
            "The `RwLock<T>` value this guard holds the exclusive write lock for. Reads and writes go straight through to the protected storage; the lock releases automatically when the guard goes out of scope.".to_string(),
        )),
        sort_text: Some("0_rwlock_write_guard_instance_value".to_string()),
        ..CompletionItem::default()
    }]
}

pub(super) fn semaphore_completion_items() -> Vec<CompletionItem> {
    [
        (
            "acquire",
            "() => Promise<SemaphorePermit>",
            "acquire()",
            "Waits for a permit, then resolves with one giving access to the limited region. The permit is released automatically when it goes out of scope, unlike a lock guard, holding it across an `await` is expected and allowed.",
        ),
        (
            "tryAcquire",
            "() => SemaphorePermit?",
            "tryAcquire()",
            "Never waits: the acquired permit if one is currently available, or `None` if every permit is already held.",
        ),
    ]
    .into_iter()
    .map(|(label, detail, insert_text, documentation)| CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(detail.to_string()),
        documentation: Some(Documentation::String(documentation.to_string())),
        insert_text: Some(insert_text.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_semaphore_instance_{label}")),
        ..CompletionItem::default()
    })
    .collect()
}

pub(super) fn websocket_completion_items() -> Vec<CompletionItem> {
    [
        (
            "stream",
            "() => Promise<DuplexStream<u8[], u8[]>>",
            "stream()",
            "Transfers binary message I/O to a bounded duplex. Each read and write is exactly one complete binary WebSocket message; receiving a text message fails its readable side with `StreamError`.",
        ),
        (
            "receive",
            "() => Promise<WebSocketMessage?>",
            "receive()",
            "Waits for the next whole message. Resolves with no message once the peer has closed the connection; rejects with `HttpError` when a message exceeds the size limit or the connection fails.",
        ),
        (
            "sendText",
            "(text: string) => Promise<void>",
            "sendText(${1:text})",
            "Sends one text message and resolves once it has been written, so a fast sender waits for a slow peer.",
        ),
        (
            "sendBytes",
            "(data: u8[]) => Promise<void>",
            "sendBytes(${1:data})",
            "Sends one binary message and resolves once it has been written.",
        ),
        (
            "close",
            "(code?: i32, reason?: string) => Promise<void>",
            "close()",
            "Sends a close frame and ends the connection. `code` is 1000 by default or between 3000 and 4999, and `reason` is at most 123 bytes; anything else rejects with `HttpError`. Closing twice, or after the peer left, is not an error.",
        ),
        (
            "protocol",
            "() => string",
            "protocol()",
            "The subprotocol both ends agreed on, or an empty string when none was negotiated.",
        ),
        (
            "closeCode",
            "() => i32?",
            "closeCode()",
            "The close code received from the peer after `receive()` observes its close frame, or no value before then or after an unclean transport end.",
        ),
    ]
    .into_iter()
    .map(|(label, detail, insert_text, documentation)| CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(detail.to_string()),
        documentation: Some(Documentation::String(documentation.to_string())),
        insert_text: Some(insert_text.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_websocket_instance_{label}")),
        ..CompletionItem::default()
    })
    .collect()
}

fn declared_receiver_type(source: &str, offset: usize, receiver: &str) -> Option<Type> {
    let prefix = source.get(..offset)?;
    for line in prefix.lines().rev() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed
            .strip_prefix("const ")
            .or_else(|| trimmed.strip_prefix("let "))
        else {
            continue;
        };
        let name_end = rest
            .char_indices()
            .find(|(_, character)| !character.is_ascii_alphanumeric() && *character != '_')
            .map(|(index, _)| index)?;
        if &rest[..name_end] != receiver {
            continue;
        }
        let after_name = rest[name_end..].trim_start();
        if after_name
            .strip_prefix('=')
            .is_some_and(|initializer| initializer.trim_start().starts_with('['))
        {
            return Some(Type::Array(Box::new(Type::Unknown)));
        }
        let declared = after_name.strip_prefix(':')?.trim_start();
        let mut depth = 0i32;
        let type_end = declared
            .char_indices()
            .find(|(_, character)| match character {
                '<' => {
                    depth += 1;
                    false
                }
                '>' => {
                    depth -= 1;
                    false
                }
                '=' | ';' | ')' => depth <= 0,
                ',' => depth <= 0,
                _ => false,
            })
            .map_or(declared.len(), |(index, _)| index);
        return parse_completion_type(declared[..type_end].trim());
    }
    None
}

fn parse_completion_type(source: &str) -> Option<Type> {
    let source = source.trim();
    if let Some(inner) = source.strip_suffix("[]") {
        return parse_completion_type(inner).map(|ty| Type::Array(Box::new(ty)));
    }
    if let Some(inner) = source
        .strip_prefix("Array<")
        .and_then(|value| value.strip_suffix('>'))
    {
        return parse_completion_type(inner).map(|ty| Type::Array(Box::new(ty)));
    }
    if let Some(inner) = source
        .strip_prefix("Record<")
        .and_then(|value| value.strip_suffix('>'))
        && let Some((key, value)) = inner.split_once(',')
        && key.trim() == "string"
    {
        let value_ty = parse_completion_type(value.trim()).unwrap_or(Type::Unknown);
        return Some(Type::Map(Box::new(Type::Str), Box::new(value_ty)));
    }
    if let Some(inner) = source
        .strip_prefix("Map<")
        .and_then(|value| value.strip_suffix('>'))
        && let Some((key, value)) = inner.split_once(',')
    {
        let key_ty = parse_completion_type(key.trim()).unwrap_or(Type::Unknown);
        let value_ty = parse_completion_type(value.trim()).unwrap_or(Type::Unknown);
        return Some(Type::Map(Box::new(key_ty), Box::new(value_ty)));
    }
    if let Some(inner) = source
        .strip_prefix("Set<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::Set(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("BinaryHeap<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::BinaryHeap(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("BinomialHeap<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::BinomialHeap(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("UnionFind<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::UnionFind(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("Counter<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::Counter(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("Deque<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::Deque(Box::new(element)));
    }
    if source.trim() == "FenwickTree" {
        return Some(Type::FenwickTree);
    }
    if source.trim() == "SegmentTree" {
        return Some(Type::SegmentTree);
    }
    if source.trim() == "SparseTable" {
        return Some(Type::SparseTable);
    }
    if source.trim() == "SegmentTreeLazy" {
        return Some(Type::SegmentTreeLazy);
    }
    if source.trim() == "BitSet" {
        return Some(Type::BitSet);
    }
    if source.trim() == "Trie" {
        return Some(Type::Trie);
    }
    if source.trim() == "SuffixTree" {
        return Some(Type::SuffixTree);
    }
    if source.trim() == "Lca" {
        return Some(Type::Lca);
    }
    if source.trim() == "Hld" {
        return Some(Type::Hld);
    }
    if source.trim() == "Centroid" {
        return Some(Type::Centroid);
    }
    if source.trim() == "BloomFilter" {
        return Some(Type::BloomFilter);
    }
    if let Some(inner) = source
        .strip_prefix("LinkedList<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::LinkedList(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("DoubleLinkedList<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::DoubleLinkedList(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("AvlTree<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::AvlTree(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("RedBlackTree<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::RedBlackTree(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("SplayTree<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::SplayTree(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("Treap<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::Treap(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("SortedSet<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::SortedSet(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("SkipList<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let element = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::SkipList(Box::new(element)));
    }
    if let Some(inner) = source
        .strip_prefix("SortedMap<")
        .and_then(|value| value.strip_suffix('>'))
        && let Some((key, value)) = inner.split_once(',')
    {
        let key_ty = parse_completion_type(key.trim()).unwrap_or(Type::Unknown);
        let value_ty = parse_completion_type(value.trim()).unwrap_or(Type::Unknown);
        return Some(Type::SortedMap(Box::new(key_ty), Box::new(value_ty)));
    }
    if let Some(inner) = source
        .strip_prefix("Promise<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let payload = parse_completion_type(inner.trim()).unwrap_or(Type::Unknown);
        return Some(Type::Apply {
            name: "Promise".to_string(),
            args: vec![payload],
        });
    }
    if let Some(inner) = source.strip_suffix('?') {
        return parse_completion_type(inner).map(|ty| Type::Optional(Box::new(ty)));
    }
    Some(match source {
        "char" => Type::Char,
        "i8" => Type::I8,
        "i16" => Type::I16,
        "i32" => Type::I32,
        "i64" => Type::I64,
        "i128" => Type::I128,
        "u8" => Type::U8,
        "u16" => Type::U16,
        "u32" => Type::U32,
        "u64" => Type::U64,
        "u128" => Type::U128,
        "f32" => Type::F32,
        "f64" => Type::F64,
        "f128" => Type::F128,
        "decimal" => Type::Decimal,
        "boolean" => Type::Bool,
        "Regex" => Type::Regex,
        "string" => Type::Str,
        "void" => Type::Void,
        "unknown" => Type::Unknown,
        "never" => Type::Never,
        name if !name.is_empty() => Type::Named(name.to_string()),
        _ => return None,
    })
}

pub(super) fn keyword_completion_items() -> Vec<CompletionItem> {
    let mut items = vec![
        snippet_completion_item(
            "if",
            "conditional block",
            "if (${1:condition}) {\n    $0\n}",
        ),
        snippet_completion_item(
            "ifelse",
            "conditional block with else",
            "if (${1:condition}) {\n    ${2}\n} else {\n    $0\n}",
        ),
        snippet_completion_item("else", "conditional branch", "else {\n    $0\n}"),
        snippet_completion_item(
            "asyncfn",
            "async function declaration",
            "async function ${1:name}(${2}): Promise<${3:void}> {\n    $0\n}",
        ),
        snippet_completion_item(
            "asyncconst",
            "async arrow function",
            "const ${1:name} = async (${2}): Promise<${3:void}> => {\n    $0\n};",
        ),
        snippet_completion_item("while", "while loop", "while (${1:condition}) {\n    $0\n}"),
        snippet_completion_item(
            "for",
            "for loop",
            "for (let ${1:i}: i32 = 0; ${1:i} < ${2:count}; ${1:i}++) {\n    $0\n}",
        ),
        snippet_completion_item(
            "forof",
            "for...of loop",
            "for (const ${1:item} of ${2:items}) {\n    $0\n}",
        ),
        snippet_completion_item(
            "forin",
            "for...in loop",
            "for (const ${1:key} in ${2:object}) {\n    $0\n}",
        ),
        snippet_completion_item(
            "do",
            "do...while loop",
            "do {\n    $0\n} while (${1:condition});",
        ),
        snippet_completion_item(
            "switch",
            "switch statement",
            "switch (${1:value}) {\n    case ${2:match}:\n        $0\n        break;\n    default:\n        break;\n}",
        ),
        snippet_completion_item(
            "match",
            "pattern match statement",
            "match (${1:value}) {\n    ${2:case} => {\n        ${3}\n    }\n    _ => {\n        $0\n    }\n}",
        ),
        snippet_completion_item(
            "try",
            "try/catch block",
            "try {\n    ${1}\n} catch (${2:error}: unknown) {\n    $0\n}",
        ),
        snippet_completion_item(
            "trycatch",
            "try/catch block",
            "try {\n    ${1}\n} catch (${2:error}: unknown) {\n    $0\n}",
        ),
        snippet_completion_item(
            "tryfinally",
            "try/catch/finally block",
            "try {\n    ${1}\n} catch (${2:error}: unknown) {\n    ${3}\n} finally {\n    $0\n}",
        ),
        snippet_completion_item(
            "interface",
            "structural object type",
            "interface ${1:Name} {\n    $0\n}",
        ),
        snippet_completion_item("type", "type alias", "type ${1:Name} = ${2:Type};$0"),
        snippet_completion_item("enum", "enumeration", "enum ${1:Name} {\n    $0\n}"),
        snippet_completion_item(
            "import",
            "import named exports",
            "import { ${1:name} } from \"${2:module}\";$0",
        ),
        snippet_completion_item("export", "export declaration", "export $0"),
    ];

    for member in tyrus_core::completion_common::CONSOLE_MEMBERS {
        items.push(function_completion_item(
            &format!("console.{}", member.name),
            member.description,
            &format!("console.{name}($0);", name = member.name),
        ));
    }

    items.extend(
        [
            ("const", "immutable declaration"),
            ("let", "mutable declaration"),
            ("of", "for...of iterator keyword"),
            ("case", "switch case"),
            ("default", "switch default case"),
            ("return", "return from function"),
            ("break", "exit the current loop"),
            ("continue", "continue the current loop"),
            ("readonly", "readonly object field"),
            ("extends", "generic constraint"),
            ("from", "import source"),
            ("as", "import alias"),
            ("async", "async function marker"),
            ("await", "await a Promise<T>"),
            ("typeof", "inspect a value's runtime type"),
            ("new", "constructor expression"),
            ("None", "explicit optional absence"),
            ("true", "boolean literal"),
            ("false", "boolean literal"),
        ]
        .into_iter()
        .map(|(label, detail)| keyword_completion_item(label, detail)),
    );
    items
}

fn keyword_completion_item(label: &str, detail: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: Some(detail.to_string()),
        ..CompletionItem::default()
    }
}

fn snippet_completion_item(label: &str, detail: &str, insert_text: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: Some(detail.to_string()),
        insert_text: Some(insert_text.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_snippet_{label}")),
        ..CompletionItem::default()
    }
}

fn function_completion_item(label: &str, detail: &str, insert_text: &str) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(detail.to_string()),
        insert_text: Some(insert_text.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_function_{label}")),
        ..CompletionItem::default()
    }
}

pub(super) fn type_completion_items(
    source: &str,
    program: Option<&Program>,
) -> Vec<CompletionItem> {
    let mut items = [
        ("char", "a single Unicode character"),
        ("i8", "8-bit signed integer"),
        ("i16", "16-bit signed integer"),
        ("i32", "32-bit signed integer"),
        ("i64", "64-bit signed integer"),
        ("i128", "128-bit signed integer"),
        ("u8", "8-bit unsigned integer"),
        ("u16", "16-bit unsigned integer"),
        ("u32", "32-bit unsigned integer"),
        ("u64", "64-bit unsigned integer"),
        ("u128", "128-bit unsigned integer"),
        ("f32", "32-bit floating-point number"),
        ("f64", "64-bit floating-point number"),
        ("f128", "128-bit floating-point number"),
        ("decimal", "bounded base-ten decimal number"),
        ("boolean", "boolean type"),
        ("string", "UTF-8 string"),
        ("void", "no return value"),
        ("unknown", "top type"),
        ("never", "bottom type"),
        ("Array", "generic array type"),
        ("Result", "success-or-error result type"),
        ("Promise", "native async result type"),
        ("Error", "native asynchronous error type"),
        (
            "AggregateError",
            "ordered asynchronous aggregate error type",
        ),
        ("HttpRouter", "native HTTP router type"),
        ("CorsOptions", "native CORS options type"),
    ]
    .into_iter()
    .map(|(label, detail)| CompletionItem {
        label: label.to_string(),
        kind: Some(CompletionItemKind::TYPE_PARAMETER),
        detail: Some(detail.to_string()),
        sort_text: Some(format!("0_type_{label}")),
        ..CompletionItem::default()
    })
    .collect::<Vec<_>>();

    if let Some(program) = program {
        items.extend(declared_type_completion_items(source, program));
    }
    items
}

pub(super) fn global_builtin_completion_items() -> Vec<CompletionItem> {
    let mut items = vec![CompletionItem {
        label: "parseDecimal".to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some("(text: string) => decimal?".to_string()),
        documentation: Some(Documentation::String(
            "Parses a complete base-ten decimal string and returns absence when the text is invalid or outside the decimal range.".to_string(),
        )),
        insert_text: Some("parseDecimal(${1:text})".to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some("0_function_parseDecimal".to_string()),
        ..CompletionItem::default()
    }, CompletionItem {
        label: "isNaN".to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some("(value: f32 | f64 | f128) => boolean".to_string()),
        documentation: Some(Documentation::String(
            "Returns true when a floating-point value is NaN.".to_string(),
        )),
        insert_text: Some("isNaN(${1:value})".to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some("0_function_isNaN".to_string()),
        ..CompletionItem::default()
    }];
    items.extend(
        tyrus_core::completion_common::PRINT_FUNCTIONS
            .iter()
            .map(|function| CompletionItem {
                label: function.name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(function.description.to_string()),
                documentation: Some(Documentation::String(function.description.to_string())),
                insert_text: Some(format!("{}($0);", function.name)),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                sort_text: Some(format!("0_function_{}", function.name)),
                ..CompletionItem::default()
            }),
    );
    items
}

pub(super) fn validator_completion_items(
    source: &str,
    offset: usize,
    analysis: &tyrus_core::analysis::DocumentAnalysis,
) -> Option<Vec<CompletionItem>> {
    let prefix = validator_completion_prefix(source, offset)?;
    let mut items = validator_infos()
        .into_iter()
        .map(validator_completion_item)
        .collect::<Vec<_>>();
    items.extend(
        analysis
            .symbols
            .iter()
            .filter(|symbol| symbol.is_validator && symbol.name.starts_with(prefix))
            .map(user_validator_completion_item),
    );
    Some(items)
}

fn user_validator_completion_item(symbol: &Symbol) -> CompletionItem {
    let parameter_types = match symbol.semantic_type.as_ref() {
        Some(Type::Function { params, .. }) => params.iter().skip(1).collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let config_parameters = if symbol.parameters.len() > 1 {
        symbol
            .parameters
            .iter()
            .skip(1)
            .map(|param| {
                let name = match &param.pattern {
                    ParamPattern::Name(name) => name.as_str(),
                    ParamPattern::Array(_) => "constraint",
                    ParamPattern::Object(_) => "constraint",
                };
                format!("{name}: {}", param.ty)
            })
            .collect::<Vec<_>>()
    } else {
        parameter_types
            .iter()
            .enumerate()
            .map(|(index, ty)| format!("constraint{}: {ty}", index + 1))
            .collect()
    };
    let mut signature_parameters = vec![format!(
        "value: {}",
        parameter_types
            .first()
            .map_or("unknown".to_string(), |ty| ty.to_string())
    )];
    signature_parameters.extend(config_parameters.iter().cloned());
    let signature = format!(
        "validator function {}({}): Validator",
        symbol.name,
        signature_parameters.join(", ")
    );
    let insert_text = if config_parameters.is_empty() {
        format!("{}()", symbol.name)
    } else {
        format!(
            "{}({})",
            symbol.name,
            (1..=config_parameters.len())
                .map(|index| format!("${{{index}}}"))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    CompletionItem {
        label: symbol.name.clone(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(signature),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: "Custom validator. It receives the decorated value first and returns `Validator`; configuration arguments are passed in decorator order.".to_string(),
        })),
        insert_text: Some(insert_text),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_validator_{}", symbol.name)),
        ..CompletionItem::default()
    }
}

const ENV_TYPE_COMPLETIONS: &[&str] = &[
    "string", "boolean", "i32", "i64", "u32", "u64", "f32", "f64",
];

pub(super) fn env_completion_items(source: &str, offset: usize) -> Option<Vec<CompletionItem>> {
    if env_type_argument_prefix(source, offset) {
        return Some(
            ENV_TYPE_COMPLETIONS
                .iter()
                .enumerate()
                .map(|(index, name)| CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::TYPE_PARAMETER),
                    detail: Some("env variable type".to_string()),
                    sort_text: Some(format!("0_type_{index:02}")),
                    ..CompletionItem::default()
                })
                .collect(),
        );
    }
    validator_completion_prefix(source, offset)?;
    let mut items = vec![CompletionItem {
        label: "type".to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some("@type(T, message?)".to_string()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: "Declares the static type of the environment variable. Required for every variable.\n\nAccepts `string`, `boolean`, `i32`, `i64`, `u32`, `u64`, `f32`, `f64`.".to_string(),
        })),
        insert_text: Some("type(${1:string})".to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some("0_validator_@type".to_string()),
        ..CompletionItem::default()
    }];
    items.extend(validator_infos().into_iter().map(validator_completion_item));
    Some(items)
}

fn env_type_argument_prefix(source: &str, offset: usize) -> bool {
    let Some(prefix) = source.get(..offset) else {
        return false;
    };
    let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
    let line = &prefix[line_start..];
    let Some(paren) = line.rfind('(') else {
        return false;
    };
    line[..paren].trim_end().ends_with("@type") && !line[paren + 1..].contains([',', ')'])
}

fn validator_completion_prefix(source: &str, offset: usize) -> Option<&str> {
    let prefix = source.get(..offset)?;
    let at = prefix.rfind('@')?;
    let after_at = &prefix[at + 1..];
    if !after_at
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '_')
    {
        return None;
    }
    let before_at = prefix[..at].chars().next_back();
    if before_at.is_some_and(|character| {
        character.is_ascii_alphanumeric() || character == '_' || character == ')'
    }) {
        return None;
    }
    Some(after_at)
}

fn validator_completion_item(info: ValidatorInfo) -> CompletionItem {
    CompletionItem {
        label: info.name.to_string(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(info.signature.clone()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: validator_markdown(&info, None),
        })),
        insert_text: Some(info.insert_text),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        sort_text: Some(format!("0_validator_{}", info.name)),
        ..CompletionItem::default()
    }
}

pub(super) fn fn_return_type_hints(
    source: &str,
    analysis: &DocumentAnalysis,
) -> Vec<(usize, String)> {
    let tokens = analysis_tokens(source, analysis);
    let mut hints = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        if matches!(tokens[index].0, Token::Const | Token::Let)
            && let Some((name, name_range)) = tokens.get(index + 1)
            && matches!(name, Token::Ident(_))
            && matches!(tokens.get(index + 2).map(|token| &token.0), Some(Token::Eq))
            && let Some((offset, has_return_annotation)) =
                arrow_function_return_hint(&tokens, index + 3)
        {
            if !has_return_annotation {
                let symbol = analysis.symbol_by_declaration(name_range).filter(|symbol| {
                    matches!(symbol.kind, SymbolKind::Variable | SymbolKind::Function)
                });
                if let Some(Type::Function { ret, .. }) =
                    symbol.and_then(|symbol| symbol.semantic_type.as_ref())
                {
                    hints.push((offset, ret.to_string()));
                }
            }
            index += 1;
            continue;
        }
        if !matches!(tokens[index].0, Token::Function) {
            index += 1;
            continue;
        }
        let name_index = index + 1;
        let Some((Token::Ident(_), name_range)) = tokens.get(name_index) else {
            index += 1;
            continue;
        };
        if !matches!(
            tokens.get(name_index + 1).map(|t| &t.0),
            Some(Token::LParen)
        ) {
            index += 1;
            continue;
        }
        let mut depth = 0i32;
        let mut cursor = name_index + 1;
        let close_index = loop {
            match tokens.get(cursor).map(|t| &t.0) {
                Some(Token::LParen) => depth += 1,
                Some(Token::RParen) => {
                    depth -= 1;
                    if depth == 0 {
                        break Some(cursor);
                    }
                }
                Some(_) => {}
                None => break None,
            }
            cursor += 1;
        };
        let Some(close_index) = close_index else {
            break;
        };
        if matches!(
            tokens.get(close_index + 1).map(|t| &t.0),
            Some(Token::Colon)
        ) {
            index = close_index + 1;
            continue;
        }
        let close_range = tokens[close_index].1.clone();
        let symbol = analysis
            .symbol_by_declaration(name_range)
            .filter(|symbol| matches!(symbol.kind, SymbolKind::Function));
        if let Some(Type::Function { ret, .. }) =
            symbol.and_then(|symbol| symbol.semantic_type.as_ref())
        {
            hints.push((close_range.end, ret.to_string()));
        }
        index = close_index + 1;
    }
    hints
}

fn arrow_function_return_hint(
    tokens: &[(Token, std::ops::Range<usize>)],
    start: usize,
) -> Option<(usize, bool)> {
    let mut index = start;
    if matches!(tokens.get(index).map(|token| &token.0), Some(Token::Async)) {
        index += 1;
    }
    if matches!(tokens.get(index).map(|token| &token.0), Some(Token::Lt)) {
        let mut depth = 0i32;
        loop {
            match tokens.get(index).map(|token| &token.0) {
                Some(Token::Lt) => depth += 1,
                Some(Token::Gt) => {
                    depth -= 1;
                    if depth == 0 {
                        index += 1;
                        break;
                    }
                }
                Some(_) => {}
                None => return None,
            }
            index += 1;
        }
    }
    if !matches!(tokens.get(index).map(|token| &token.0), Some(Token::LParen)) {
        return None;
    }
    let mut depth = 0i32;
    let close_index = loop {
        match tokens.get(index).map(|token| &token.0) {
            Some(Token::LParen) => depth += 1,
            Some(Token::RParen) => {
                depth -= 1;
                if depth == 0 {
                    break index;
                }
            }
            Some(_) => {}
            None => return None,
        }
        index += 1;
    };
    if !matches!(
        tokens.get(close_index + 1).map(|token| &token.0),
        Some(Token::Arrow | Token::Colon)
    ) {
        return None;
    }
    let has_return_annotation = matches!(
        tokens.get(close_index + 1).map(|token| &token.0),
        Some(Token::Colon)
    );
    Some((tokens[close_index].1.end, has_return_annotation))
}

pub(super) fn inferred_type_hints(
    source: &str,
    analysis: &DocumentAnalysis,
) -> Vec<(usize, String)> {
    let tokens = analysis_tokens(source, analysis);
    let mut hints = Vec::new();
    for (index, window) in tokens.windows(3).enumerate() {
        let [(keyword, _), (name, name_range), (eq, _)] = window else {
            continue;
        };
        if !matches!(keyword, Token::Const | Token::Let)
            || !matches!(eq, Token::Eq)
            || !matches!(name, Token::Ident(_))
        {
            continue;
        }
        let Some(symbol) = analysis
            .symbol_by_declaration(name_range)
            .filter(|symbol| matches!(symbol.kind, SymbolKind::Variable | SymbolKind::Function))
        else {
            continue;
        };
        if initializer_is_arrow_function(&tokens, index + 3) {
            continue;
        }
        let has_error_diagnostic = analysis.diagnostics.iter().any(|diagnostic| {
            diagnostic.severity == tyrus_core::analysis::DiagnosticSeverity::Error
                && diagnostic.range == *name_range
        });
        if has_error_diagnostic {
            continue;
        }
        let ty = if symbol.ty.is_empty() || symbol.ty == "<inferred>" || symbol.ty == "unknown" {
            initializer_type_hint(&tokens, index + 2, analysis)
        } else {
            Some(symbol.ty.clone())
        };
        let Some(ty) = ty else {
            continue;
        };
        if ty == "<inferred>" || ty == "unknown" || ty == "void" {
            continue;
        }
        hints.push((name_range.end, ty));
    }
    hints
}

fn analysis_tokens(
    source: &str,
    analysis: &DocumentAnalysis,
) -> Vec<(Token, std::ops::Range<usize>)> {
    if analysis.tokens.is_empty() {
        return tyrus_core::lexer::lex_spanned(source).unwrap_or_default();
    }
    analysis
        .tokens
        .iter()
        .map(|token| (token.token.clone(), token.range.clone()))
        .collect()
}

fn initializer_is_arrow_function(tokens: &[(Token, std::ops::Range<usize>)], start: usize) -> bool {
    let mut i = start;
    if matches!(tokens.get(i).map(|(token, _)| token), Some(Token::Async)) {
        i += 1;
    }
    if matches!(tokens.get(i).map(|(token, _)| token), Some(Token::Lt)) {
        let mut depth = 0i32;
        loop {
            match tokens.get(i).map(|(token, _)| token) {
                Some(Token::Lt) => depth += 1,
                Some(Token::Gt) => {
                    depth -= 1;
                    if depth == 0 {
                        i += 1;
                        break;
                    }
                }
                Some(_) => {}
                None => return false,
            }
            i += 1;
        }
    }
    if !matches!(tokens.get(i).map(|(token, _)| token), Some(Token::LParen)) {
        return false;
    }
    let mut parens = 0i32;
    loop {
        match tokens.get(i).map(|(token, _)| token) {
            Some(Token::LParen) => parens += 1,
            Some(Token::RParen) => {
                parens -= 1;
                if parens == 0 {
                    i += 1;
                    break;
                }
            }
            Some(_) => {}
            None => return false,
        }
        i += 1;
    }
    let mut depth = 0i32;
    while let Some((token, _)) = tokens.get(i) {
        match token {
            Token::Arrow if depth == 0 => return true,
            Token::LParen | Token::LBracket | Token::LBrace => depth += 1,
            Token::RParen | Token::RBracket | Token::RBrace => depth -= 1,
            Token::Semicolon if depth == 0 => return false,
            _ => {}
        }
        i += 1;
    }
    false
}

fn initializer_type_hint(
    tokens: &[(Token, std::ops::Range<usize>)],
    eq_index: usize,
    analysis: &DocumentAnalysis,
) -> Option<String> {
    let range = initializer_range(tokens, eq_index)?;
    analysis
        .expression_types
        .iter()
        .filter(|expr| range.start <= expr.range.start && expr.range.end <= range.end)
        .max_by_key(|expr| expr.range.end.saturating_sub(expr.range.start))
        .map(|expr| expr.ty.clone())
}

fn initializer_range(
    tokens: &[(Token, std::ops::Range<usize>)],
    eq_index: usize,
) -> Option<std::ops::Range<usize>> {
    let start = tokens.get(eq_index + 1)?.1.start;
    let mut end = tokens.get(eq_index + 1)?.1.end;
    let mut parens = 0usize;
    let mut brackets = 0usize;
    let mut braces = 0usize;
    for (token, range) in tokens.iter().skip(eq_index + 1) {
        match token {
            Token::Semicolon if parens == 0 && brackets == 0 && braces == 0 => break,
            Token::LParen => parens += 1,
            Token::RParen => parens = parens.saturating_sub(1),
            Token::LBracket => brackets += 1,
            Token::RBracket => brackets = brackets.saturating_sub(1),
            Token::LBrace => braces += 1,
            Token::RBrace => braces = braces.saturating_sub(1),
            _ => {}
        }
        end = range.end;
    }
    Some(start..end)
}

pub(super) fn validator_hover_at(
    source: &str,
    offset: usize,
) -> Option<(std::ops::Range<usize>, String)> {
    let tokens = tyrus_core::lexer::lex_spanned(source).unwrap_or_default();
    for (index, (token, range)) in tokens.iter().enumerate() {
        if !matches!(token, Token::AtSign) {
            continue;
        }
        let Some((Token::Ident(name), name_range)) = tokens.get(index + 1) else {
            continue;
        };
        let full_range = range.start..name_range.end;
        if !full_range.contains(&offset) && offset != full_range.end {
            continue;
        }
        let info = validator_info(name)?;
        let concrete = validator_decorator_from_tokens(name, &tokens, index)
            .and_then(|decorator| validator_message(&decorator));
        return Some((full_range, validator_markdown(&info, concrete.as_deref())));
    }
    None
}

fn validator_markdown(info: &ValidatorInfo, concrete_message: Option<&str>) -> String {
    let mut markdown = format!(
        "{}\n\n```tyrus\n{}\n```\n\nDefault message: `{}`",
        info.description, info.signature, info.default_message
    );
    if let Some(message) = concrete_message {
        markdown.push_str(&format!("\n\nConcrete message: `{message}`"));
    }
    markdown
}

fn validator_decorator_from_tokens(
    name: &str,
    tokens: &[(Token, std::ops::Range<usize>)],
    at_index: usize,
) -> Option<ValidatorDecorator> {
    let mut args = Vec::new();
    let mut argument_ranges = Vec::new();
    if !matches!(
        tokens.get(at_index + 2).map(|(token, _)| token),
        Some(Token::LParen)
    ) {
        return Some(ValidatorDecorator {
            name: name.to_string(),
            display_name: name.to_string(),
            args,
            argument_ranges,
            span: Span::DUMMY,
        });
    }
    let mut index = at_index + 3;
    while !matches!(
        tokens.get(index).map(|(token, _)| token),
        Some(Token::RParen)
    ) {
        let start = tokens.get(index)?.1.start;
        args.push(parse_validator_hover_arg(tokens, &mut index)?);
        let end = tokens
            .get(index.saturating_sub(1))
            .map_or(start, |(_, range)| range.end);
        argument_ranges.push(start..end);
        match tokens.get(index).map(|(token, _)| token) {
            Some(Token::Comma) => index += 1,
            Some(Token::RParen) => {}
            _ => return None,
        }
    }
    Some(ValidatorDecorator {
        name: name.to_string(),
        display_name: name.to_string(),
        args,
        argument_ranges,
        span: Span::DUMMY,
    })
}

fn parse_validator_hover_arg(
    tokens: &[(Token, std::ops::Range<usize>)],
    index: &mut usize,
) -> Option<Expr> {
    let (token, range) = tokens.get(*index)?;
    let span: tyrus_core::span::Span = range.clone().into();
    let expr = match token {
        Token::IntLit(value) => ExprKind::IntLit(*value),
        Token::FloatLit(value) => ExprKind::FloatLit(value.clone()),
        Token::SpecialFloatLit(value) => ExprKind::FloatLit(value.clone()),
        Token::StringLit(value) => ExprKind::StrLit(value.clone()),
        Token::True => ExprKind::BoolLit(true),
        Token::False => ExprKind::BoolLit(false),
        Token::Null => ExprKind::Null,
        Token::Minus => {
            *index += 1;
            ExprKind::Neg(Box::new(parse_validator_hover_arg(tokens, index)?))
        }
        Token::LBracket => {
            *index += 1;
            let mut values = Vec::new();
            while !matches!(
                tokens.get(*index).map(|(token, _)| token),
                Some(Token::RBracket)
            ) {
                values.push(parse_validator_hover_arg(tokens, index)?);
                match tokens.get(*index).map(|(token, _)| token) {
                    Some(Token::Comma) => *index += 1,
                    Some(Token::RBracket) => {}
                    _ => return None,
                }
            }
            ExprKind::ArrayLit(values)
        }
        _ => return None,
    };
    *index += 1;
    Some(Expr::new(expr, span))
}

fn declared_type_completion_items(source: &str, program: &Program) -> Vec<CompletionItem> {
    let docs = extract_doc_comments(source);
    program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::InterfaceDecl { name, .. } => {
                Some((name, "interface", CompletionItemKind::INTERFACE))
            }
            Item::TypeAlias { name, .. } => Some((name, "type alias", CompletionItemKind::STRUCT)),
            Item::EnumDecl { name, .. } => Some((name, "enum", CompletionItemKind::ENUM)),
            _ => None,
        })
        .map(|(name, detail, kind)| {
            let decl_start = source
                .match_indices(name)
                .map(|(index, _)| index)
                .find(|index| {
                    source[..*index].trim_end().ends_with("interface")
                        || source[..*index].trim_end().ends_with("type")
                        || source[..*index].trim_end().ends_with("enum")
                })
                .unwrap_or(0);
            let anchor = declaration_doc_anchor(source, decl_start);
            let mut item = CompletionItem {
                label: name.clone(),
                kind: Some(kind),
                detail: Some(detail.to_string()),
                sort_text: Some(format!("0_type_{name}")),
                ..CompletionItem::default()
            };
            if let Some(doc) = doc_comment_before_decl(source, anchor, &docs) {
                item.documentation = Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc.to_markdown(),
                }));
            }
            item
        })
        .collect()
}

pub(super) fn http_router_completion_items() -> Vec<CompletionItem> {
    [
        (
            "get",
            "(path: string, options: HttpRouteOptions) => void",
            &["path", "options"][..],
        ),
        (
            "post",
            "(path: string, options: HttpRouteOptions) => void",
            &["path", "options"][..],
        ),
        (
            "put",
            "(path: string, options: HttpRouteOptions) => void",
            &["path", "options"][..],
        ),
        (
            "patch",
            "(path: string, options: HttpRouteOptions) => void",
            &["path", "options"][..],
        ),
        (
            "delete",
            "(path: string, options: HttpRouteOptions) => void",
            &["path", "options"][..],
        ),
        (
            "head",
            "(path: string, options: HttpRouteOptions) => void",
            &["path", "options"][..],
        ),
        (
            "options",
            "(path: string, options: HttpRouteOptions) => void",
            &["path", "options"][..],
        ),
        (
            "connect",
            "(path: string, options: HttpRouteOptions) => void",
            &["path", "options"][..],
        ),
        (
            "query",
            "(path: string, options: HttpRouteOptions) => void",
            &["path", "options"][..],
        ),
        (
            "trace",
            "(path: string, options: HttpRouteOptions) => void",
            &["path", "options"][..],
        ),
        ("cors", "(options: CorsOptions) => void", &["options"][..]),
        (
            "rateLimit",
            "(limit: { max: i64, timeWindow: string, globalMax?: i64, ban?: i64, banWindow?: string }) => void",
            &["limit"][..],
        ),
        (
            "fetchPolicy",
            "(options: FetchPolicyOptions) => void",
            &["options"][..],
        ),
        (
            "trustedProxies",
            "(proxies: string[], identityHeader?: string) => void",
            &["proxies", "identityHeader"][..],
        ),
        (
            "ws",
            "(path: string, options: { handler: (socket: WebSocket) => Promise<void>, origins?: string[], subprotocols?: string[], maxMessageBytes?: i64, preHandler?: ((request: HttpRequest) => HttpResponse | true)[], config?: { rateLimit?: RateLimit } }) => void",
            &["path", "options"][..],
        ),
        (
            "docs",
            "(options?: { path?: string, title?: string, version?: string, description?: string, auth?: { bearer?: boolean, bearerFormat?: string, apiKey?: { name: string, in: string }, oauth2?: { authorizationUrl?: string, tokenUrl: string, scopes?: string[] } }, css?: string, preHandler?: ((request: HttpRequest) => HttpResponse | true)[] }) => void",
            &[][..],
        ),
        (
            "listen",
            "(port: i32, options?: ServerOptions) => void",
            &["port"][..],
        ),
        (
            "start",
            "(port: i32, options?: ServerOptions) => HttpServer",
            &["port"][..],
        ),
    ]
    .into_iter()
    .map(|(name, detail, parameters)| {
        let mut item = CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(detail.to_string()),
            ..CompletionItem::default()
        };
        let parameters = parameters
            .iter()
            .map(|parameter| (*parameter).to_string())
            .collect::<Vec<_>>();
        apply_function_snippet(&mut item, name, &parameters);
        item
    })
    .collect()
}

pub(super) fn non_mutating_array_completion_items() -> Vec<CompletionItem> {
    array_completion_items()
        .into_iter()
        .filter(|item| !tyrus_core::type_checker::is_array_mutating_method(&item.label))
        .collect()
}

pub(super) fn array_completion_items() -> Vec<CompletionItem> {
    let mut items = [
        ("push", "(...items: T[]) => usize", &["item"][..]),
        ("pop", "() => T?", &[][..]),
        ("shift", "() => T?", &[][..]),
        (
            "splice",
            "(start: i64, deleteCount?: i64, ...items: T[]) => T[]",
            &["start", "deleteCount", "item"][..],
        ),
        ("unshift", "(...items: T[]) => usize", &["item"][..]),
        ("reverse", "() => T[]", &[][..]),
                ("includes", "(value: T) => boolean", &["value"][..]),
        ("equals", "(other: T[]) => boolean", &["other"][..]),
        ("indexOf", "(value: T) => i64", &["value"][..]),
        ("lastIndexOf", "(value: T) => i64", &["value"][..]),
        (
            "slice",
            "(start?: i64, end?: i64) => T[]",
            &["start", "end"][..],
        ),
        ("concat", "(other: T[]) => T[]", &["other"][..]),
        ("zip", "<U>(other: U[]) => [T, U][]", &["other"][..]),
        (
            "fill",
            "(value: T, start?: i64, end?: i64) => T[]",
            &["value", "start", "end"][..],
        ),
        (
            "copyWithin",
            "(target: i64, start: i64, end?: i64) => T[]",
            &["target", "start", "end"][..],
        ),
        ("toReversed", "() => T[]", &[][..]),
        (
            "splitAt",
            "(index: usize) => [T[], T[]]",
            &["index"][..],
        ),
        (
            "count",
            "(condition?: T | ((value: T) => boolean)) => usize",
            &["condition"][..],
        ),
        ("at", "(index: i64) => T?", &["index"][..]),
        ("random", "() => T?", &[][..]),
        ("shuffle", "() => T[]", &[][..]),
        (
            "sample",
            "(count: usize, options?: { repeat?: boolean }) => T[]",
            &["count", "options"][..],
        ),
        ("keys", "() => usize[]", &[][..]),
        ("values", "() => T[]", &[][..]),
        ("entries", "() => [usize, T][]", &[][..]),
        (
            "with",
            "(index: i64, value: T) => T[]",
            &["index", "value"][..],
        ),
        ("join", "(separator?: string) => string", &["separator"][..]),
        (
            "map",
            "<U>(callback: (value: T, index: i64, array: T[]) => U) => U[]",
            &["callback"][..],
        ),
        (
            "filter",
            "(callback: (value: T, index: i64, array: T[]) => boolean) => T[]",
            &["callback"][..],
        ),
        ("some", "(callback: (value: T, index: i64, array: T[]) => boolean) => boolean", &["callback"][..]),
        ("every", "(callback: (value: T, index: i64, array: T[]) => boolean) => boolean", &["callback"][..]),
        ("find", "(callback: (value: T, index: i64, array: T[]) => boolean) => T?", &["callback"][..]),
        ("findLast", "(callback: (value: T, index: i64, array: T[]) => boolean) => T?", &["callback"][..]),
        ("findIndex", "(callback: (value: T, index: i64, array: T[]) => boolean) => i64", &["callback"][..]),
        ("findLastIndex", "(callback: (value: T, index: i64, array: T[]) => boolean) => i64", &["callback"][..]),
        ("forEach", "(callback: (value: T, index: i64, array: T[]) => void) => void", &["callback"][..]),
        ("reduce", "<U>(callback: (accumulator: U, value: T, index: i64, array: T[]) => U, initialValue?: U) => U", &["callback", "initialValue"][..]),
        ("reduceRight", "<U>(callback: (accumulator: U, value: T, index: i64, array: T[]) => U, initialValue?: U) => U", &["callback", "initialValue"][..]),
        ("sort", "(compareFn?: (left: T, right: T) => i32) => T[]", &["compareFn"][..]),
        ("toSorted", "(compareFn?: (left: T, right: T) => i32) => T[]", &["compareFn"][..]),
        ("flat", "(depth?: i64) => FlatArray<T>[]", &["depth"][..]),
        ("flatMap", "<U>(callback: (value: T, index: i64, array: T[]) => U[]) => U[]", &["callback"][..]),
        ("toggle", "(item: T, key?: string) => T[]", &["item", "key"][..]),
        ("toggleAll", "(items: T[]) => T[]", &["items"][..]),
    ]
    .into_iter()
    .map(|(name, detail, parameters)| {
        let mut item = CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(detail.to_string()),
            ..CompletionItem::default()
        };
        apply_function_snippet(
            &mut item,
            name,
            &parameters
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>(),
        );
        item
    })
    .collect::<Vec<_>>();
    items.extend(
        ["length", "capacity"]
            .into_iter()
            .map(|name| CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some("i64".to_string()),
                ..CompletionItem::default()
            }),
    );
    items
}

pub(super) fn map_completion_items() -> Vec<CompletionItem> {
    let mut items = [
        ("get", "(key: K) => V?", &["key"][..]),
        (
            "set",
            "(key: K, value: V) => Map<K, V>",
            &["key", "value"][..],
        ),
        ("has", "(key: K) => boolean", &["key"][..]),
        ("delete", "(key: K) => boolean", &["key"][..]),
        ("keys", "() => K[]", &[][..]),
        ("values", "() => V[]", &[][..]),
        ("entries", "() => [K, V][]", &[][..]),
    ]
    .into_iter()
    .map(|(name, detail, parameters)| {
        let mut item = CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(detail.to_string()),
            ..CompletionItem::default()
        };
        apply_function_snippet(
            &mut item,
            name,
            &parameters
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>(),
        );
        item
    })
    .collect::<Vec<_>>();
    items.push(CompletionItem {
        label: "size".to_string(),
        kind: Some(CompletionItemKind::FIELD),
        detail: Some("usize".to_string()),
        ..CompletionItem::default()
    });
    items
}

pub(super) fn set_completion_items() -> Vec<CompletionItem> {
    let mut items = [
        ("add", "(value: T) => Set<T>", &["value"][..]),
        ("has", "(value: T) => boolean", &["value"][..]),
        ("delete", "(value: T) => boolean", &["value"][..]),
        ("values", "() => T[]", &[][..]),
    ]
    .into_iter()
    .map(|(name, detail, parameters)| {
        let mut item = CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(detail.to_string()),
            ..CompletionItem::default()
        };
        apply_function_snippet(
            &mut item,
            name,
            &parameters
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>(),
        );
        item
    })
    .collect::<Vec<_>>();
    items.push(CompletionItem {
        label: "size".to_string(),
        kind: Some(CompletionItemKind::FIELD),
        detail: Some("usize".to_string()),
        ..CompletionItem::default()
    });
    items
}

fn algorithm_completion_items(methods: &[(&str, &str, &[&str])]) -> Vec<CompletionItem> {
    methods
        .iter()
        .map(|(name, detail, parameters)| {
            let mut item = CompletionItem {
                label: (*name).to_string(),
                kind: Some(CompletionItemKind::METHOD),
                detail: Some((*detail).to_string()),
                ..CompletionItem::default()
            };
            apply_function_snippet(
                &mut item,
                name,
                &parameters
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect::<Vec<_>>(),
            );
            item
        })
        .collect()
}

pub(super) fn binary_heap_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("push", "(item: T) => void", &["item"]),
        ("pop", "() => T?", &[]),
        ("peek", "() => T?", &[]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
        ("clear", "() => void", &[]),
    ])
}

pub(super) fn binomial_heap_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("push", "(item: T) => void", &["item"]),
        ("pop", "() => T?", &[]),
        ("peek", "() => T?", &[]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
        ("clear", "() => void", &[]),
    ])
}

pub(super) fn union_find_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("find", "(x: T) => T", &["x"]),
        ("union", "(x: T, y: T) => boolean", &["x", "y"]),
        ("connected", "(x: T, y: T) => boolean", &["x", "y"]),
        ("setCount", "() => usize", &[]),
    ])
}

pub(super) fn counter_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("add", "(item: T, count?: i32) => void", &["item", "count"]),
        (
            "remove",
            "(item: T, count?: i32) => void",
            &["item", "count"],
        ),
        ("count", "(item: T) => i32", &["item"]),
        ("mostCommon", "(k: usize) => [T, i32][]", &["k"]),
        ("total", "() => i32", &[]),
        ("unique", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
        ("clear", "() => void", &[]),
    ])
}

pub(super) fn deque_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("pushFront", "(item: T) => void", &["item"]),
        ("pushBack", "(item: T) => void", &["item"]),
        ("popFront", "() => T?", &[]),
        ("popBack", "() => T?", &[]),
        ("peekFront", "() => T?", &[]),
        ("peekBack", "() => T?", &[]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
        ("clear", "() => void", &[]),
    ])
}

pub(super) fn fenwick_tree_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        (
            "add",
            "(index: usize, delta: i32) => void",
            &["index", "delta"],
        ),
        ("sum", "(index: usize) => i32", &["index"]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
    ])
}

pub(super) fn segment_tree_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        (
            "update",
            "(index: usize, value: i32) => void",
            &["index", "value"],
        ),
        ("query", "(from: usize, to: usize) => i32", &["from", "to"]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
    ])
}

pub(super) fn segment_tree_lazy_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        (
            "updateRange",
            "(from: usize, to: usize, delta: i32) => void",
            &["from", "to", "delta"],
        ),
        ("query", "(from: usize, to: usize) => i32", &["from", "to"]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
    ])
}

pub(super) fn sparse_table_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("query", "(from: usize, to: usize) => i32", &["from", "to"]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
    ])
}

pub(super) fn bit_set_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        (
            "set",
            "(index: usize, value?: boolean) => void",
            &["index", "value"],
        ),
        ("flip", "(index: usize) => void", &["index"]),
        ("clear", "() => void", &[]),
        ("get", "(index: usize) => boolean", &["index"]),
        ("count", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
        ("isSubset", "(other: BitSet) => boolean", &["other"]),
        ("isSuperset", "(other: BitSet) => boolean", &["other"]),
        ("union", "(other: BitSet) => BitSet", &["other"]),
        ("intersection", "(other: BitSet) => BitSet", &["other"]),
        ("difference", "(other: BitSet) => BitSet", &["other"]),
        ("complement", "() => BitSet", &[]),
    ])
}

pub(super) fn trie_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("insert", "(word: string) => void", &["word"]),
        ("search", "(word: string) => boolean", &["word"]),
        ("startsWith", "(prefix: string) => boolean", &["prefix"]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
        ("clear", "() => void", &[]),
    ])
}

pub(super) fn suffix_tree_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[("getSuffixArray", "() => usize[]", &[])])
}

pub(super) fn lca_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[("query", "(u: usize, v: usize) => usize", &["u", "v"])])
}

pub(super) fn hld_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        (
            "queryPath",
            "(u: usize, v: usize) => [usize, usize][]",
            &["u", "v"],
        ),
        ("position", "(v: usize) => usize", &["v"]),
        ("head", "(v: usize) => usize", &["v"]),
    ])
}

pub(super) fn centroid_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("parentOf", "(v: usize) => usize?", &["v"]),
        ("depth", "(v: usize) => usize", &["v"]),
    ])
}

pub(super) fn bloom_filter_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("insert", "(key: string) => void", &["key"]),
        ("contains", "(key: string) => boolean", &["key"]),
    ])
}

pub(super) fn linked_list_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("pushFront", "(item: T) => void", &["item"]),
        ("pushBack", "(item: T) => void", &["item"]),
        ("popFront", "() => T?", &[]),
        ("popBack", "() => T?", &[]),
        ("front", "() => T?", &[]),
        ("back", "() => T?", &[]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
        ("clear", "() => void", &[]),
    ])
}

pub(super) fn double_linked_list_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("pushFront", "(item: T) => void", &["item"]),
        ("pushBack", "(item: T) => void", &["item"]),
        ("popFront", "() => T?", &[]),
        ("popBack", "() => T?", &[]),
        ("front", "() => T?", &[]),
        ("back", "() => T?", &[]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
        ("clear", "() => void", &[]),
    ])
}

fn ordered_tree_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("insert", "(item: T) => void", &["item"]),
        ("contains", "(item: T) => boolean", &["item"]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
    ])
}

pub(super) fn avl_tree_completion_items() -> Vec<CompletionItem> {
    ordered_tree_completion_items()
}

pub(super) fn red_black_tree_completion_items() -> Vec<CompletionItem> {
    ordered_tree_completion_items()
}

pub(super) fn splay_tree_completion_items() -> Vec<CompletionItem> {
    ordered_tree_completion_items()
}

pub(super) fn treap_completion_items() -> Vec<CompletionItem> {
    ordered_tree_completion_items()
}

pub(super) fn sorted_set_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("insert", "(item: T) => boolean", &["item"]),
        ("remove", "(item: T) => boolean", &["item"]),
        ("contains", "(item: T) => boolean", &["item"]),
        ("min", "() => T?", &[]),
        ("max", "() => T?", &[]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
        ("clear", "() => void", &[]),
    ])
}

pub(super) fn skip_list_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("insert", "(item: T) => boolean", &["item"]),
        ("remove", "(item: T) => boolean", &["item"]),
        ("contains", "(item: T) => boolean", &["item"]),
        ("min", "() => T?", &[]),
        ("max", "() => T?", &[]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
        ("clear", "() => void", &[]),
    ])
}

pub(super) fn sorted_map_completion_items() -> Vec<CompletionItem> {
    algorithm_completion_items(&[
        ("set", "(key: K, value: V) => void", &["key", "value"]),
        ("get", "(key: K) => V?", &["key"]),
        ("has", "(key: K) => boolean", &["key"]),
        ("delete", "(key: K) => boolean", &["key"]),
        ("min", "() => K?", &[]),
        ("max", "() => K?", &[]),
        ("len", "() => usize", &[]),
        ("isEmpty", "() => boolean", &[]),
        ("clear", "() => void", &[]),
    ])
}

pub(super) fn char_completion_items() -> Vec<CompletionItem> {
    let mut items = [
        ("toUpperCase", "() => char", &[][..]),
        ("toLowerCase", "() => char", &[][..]),
        ("isDigit", "() => boolean", &[][..]),
        ("isAlphabetic", "() => boolean", &[][..]),
        ("isAlphanumeric", "() => boolean", &[][..]),
        ("isWhitespace", "() => boolean", &[][..]),
        ("toString", "() => string", &[][..]),
    ]
    .into_iter()
    .map(
        |(name, detail, _parameters): (&str, &str, &[&str])| CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(detail.to_string()),
            insert_text: Some(format!("{name}()")),
            ..CompletionItem::default()
        },
    )
    .collect::<Vec<_>>();
    items.sort_by(|left, right| left.label.cmp(&right.label));
    items
}

pub(super) fn string_completion_items() -> Vec<CompletionItem> {
    let mut items = [
        ("charAt", "(index: i64) => string", &["index"][..]),
        ("chars", "() => char[]", &[][..]),
        ("graphemes", "() => string[]", &[][..]),
        ("charCodeAt", "(index: i64) => i32", &["index"][..]),
        ("includes", "(search: string) => boolean", &["search"][..]),
        ("startsWith", "(search: string) => boolean", &["search"][..]),
        ("endsWith", "(search: string) => boolean", &["search"][..]),
        (
            "indexOf",
            "(search: string, fromIndex?: i64) => i64",
            &["search", "fromIndex"][..],
        ),
        (
            "lastIndexOf",
            "(search: string, fromIndex?: i64) => i64",
            &["search", "fromIndex"][..],
        ),
        (
            "slice",
            "(start?: i64, end?: i64) => string",
            &["start", "end"][..],
        ),
        (
            "substring",
            "(start: i64, end?: i64) => string",
            &["start", "end"][..],
        ),
        ("trim", "() => string", &[][..]),
        ("trimStart", "() => string", &[][..]),
        ("trimEnd", "() => string", &[][..]),
        ("toLowerCase", "() => string", &[][..]),
        ("toUpperCase", "() => string", &[][..]),
        ("match", "(pattern: Regex) => string[]?", &["pattern"][..]),
        (
            "matchAll",
            "(pattern: Regex) => string[][]?",
            &["pattern"][..],
        ),
        ("search", "(pattern: Regex) => i32", &["pattern"][..]),
        (
            "split",
            "(separator: string | Regex) => string[]",
            &["separator"][..],
        ),
        (
            "replace",
            "(search: string | Regex, replacement: string) => string",
            &["search", "replacement"][..],
        ),
        ("repeat", "(count: i64) => string", &["count"][..]),
        (
            "padStart",
            "(targetLength: i64, padString?: string) => string",
            &["targetLength", "padString"][..],
        ),
        (
            "padEnd",
            "(targetLength: i64, padString?: string) => string",
            &["targetLength", "padString"][..],
        ),
        ("concat", "(...values: string[]) => string", &["value"][..]),
        ("count", "(sub?: string) => usize", &["sub"][..]),
        ("toSlugCase", "() => string", &[][..]),
        ("toDashCase", "() => string", &[][..]),
        ("toKebabCase", "() => string", &[][..]),
        ("toPascalCase", "() => string", &[][..]),
        ("toCamelCase", "() => string", &[][..]),
        ("toSnakeCase", "() => string", &[][..]),
        ("truncate", "(maxLength: i64) => string", &["maxLength"][..]),
        (
            "truncateEllipsis",
            "(maxLength: i64) => string",
            &["maxLength"][..],
        ),
        ("stripHtml", "() => string", &[][..]),
        ("escapeHtml", "() => string", &[][..]),
        ("urlEncode", "() => string", &[][..]),
        ("urlDecode", "() => string", &[][..]),
    ]
    .into_iter()
    .map(|(name, detail, parameters)| {
        let mut item = CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(detail.to_string()),
            ..CompletionItem::default()
        };
        apply_function_snippet(
            &mut item,
            name,
            &parameters
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>(),
        );
        item
    })
    .collect::<Vec<_>>();
    items.push(CompletionItem {
        label: "length".to_string(),
        kind: Some(CompletionItemKind::FIELD),
        detail: Some("i64".to_string()),
        ..CompletionItem::default()
    });
    items
}

pub(super) fn regex_completion_items() -> Vec<CompletionItem> {
    let mut items = [
        ("test", "(input: string) => boolean", &["input"][..]),
        ("exec", "(input: string) => string[]?", &["input"][..]),
        ("execAll", "(input: string) => string[][]?", &["input"][..]),
        ("toString", "() => string", &[][..]),
    ]
    .into_iter()
    .map(|(name, detail, parameters)| {
        let mut item = CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(detail.to_string()),
            ..CompletionItem::default()
        };
        apply_function_snippet(
            &mut item,
            name,
            &parameters
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>(),
        );
        item
    })
    .collect::<Vec<_>>();
    items.extend(
        [
            ("source", "string"),
            ("flags", "string"),
            ("ignoreCase", "boolean"),
            ("multiline", "boolean"),
            ("dotAll", "boolean"),
            ("global", "boolean"),
            ("sticky", "boolean"),
            ("unicode", "boolean"),
            ("hasIndices", "boolean"),
            ("lastIndex", "i32"),
        ]
        .into_iter()
        .map(|(label, detail)| CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::FIELD),
            detail: Some(detail.to_string()),
            ..CompletionItem::default()
        }),
    );
    items
}

pub(super) fn symbol_completion_item(symbol: &Symbol) -> CompletionItem {
    let mut item = CompletionItem {
        label: symbol.name.clone(),
        kind: Some(completion_kind(symbol.kind)),
        detail: Some(symbol.ty.clone()),
        ..CompletionItem::default()
    };
    if let Some(doc) = &symbol.documentation {
        item.documentation = Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc.to_markdown(),
        }));
    }
    let parameters = symbol
        .parameters
        .iter()
        .map(|parameter| parameter_pattern_argument(&parameter.pattern))
        .collect::<Vec<_>>();
    apply_function_snippet(&mut item, &symbol.name, &parameters);
    item
}

pub(super) fn parameter_pattern_argument(pattern: &ParamPattern) -> String {
    match pattern {
        ParamPattern::Name(name) => name.clone(),
        ParamPattern::Array(bindings) => format!("[{}]", bindings.join(", ")),
        ParamPattern::Object(bindings) => format!("{{{}}}", bindings.join(", ")),
    }
}

pub(super) fn object_literal_context(
    source: &str,
    offset: usize,
) -> Option<(String, std::collections::HashSet<String>)> {
    let tokens = tyrus_core::lexer::lex_spanned(source).unwrap_or_default();

    let mut open_braces: Vec<usize> = Vec::new();
    for (index, (token, range)) in tokens.iter().enumerate() {
        if range.start >= offset {
            break;
        }
        match token {
            Token::LBrace => open_braces.push(index),
            Token::RBrace => {
                open_braces.pop();
            }
            _ => {}
        }
    }
    let open = *open_braces.last()?;

    if !matches!(
        open.checked_sub(1)
            .and_then(|i| tokens.get(i))
            .map(|t| &t.0),
        Some(Token::Eq)
    ) {
        return None;
    }
    let Some((Token::Ident(name), _)) = open.checked_sub(2).and_then(|i| tokens.get(i)) else {
        return None;
    };
    if !matches!(
        open.checked_sub(3)
            .and_then(|i| tokens.get(i))
            .map(|t| &t.0),
        Some(Token::Colon)
    ) {
        return None;
    }

    let mut keys = std::collections::HashSet::default();
    let mut depth = 1i32;
    let mut index = open + 1;
    while index < tokens.len() {
        match &tokens[index].0 {
            Token::LBrace | Token::LBracket | Token::LParen => depth += 1,
            Token::RBrace | Token::RBracket | Token::RParen => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Token::Ident(field) if depth == 1 => {
                if matches!(tokens.get(index + 1).map(|t| &t.0), Some(Token::Colon)) {
                    keys.insert(field.clone());
                }
            }
            _ => {}
        }
        index += 1;
    }
    Some((name.clone(), keys))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HttpRouteObjectContext {
    Route,
    WebSocket,
    RouteConfig,
    WebSocketConfig,
    RateLimit,
}

fn open_object_brace(tokens: &[(Token, std::ops::Range<usize>)], offset: usize) -> Option<usize> {
    let mut open_braces = Vec::new();
    for (index, (token, range)) in tokens.iter().enumerate() {
        if range.start >= offset {
            break;
        }
        match token {
            Token::LBrace => open_braces.push(index),
            Token::RBrace => {
                open_braces.pop();
            }
            _ => {}
        }
    }
    open_braces.last().copied()
}

fn route_context_at_brace(
    tokens: &[(Token, std::ops::Range<usize>)],
    brace: usize,
) -> Option<HttpRouteObjectContext> {
    let parent = tokens[..brace]
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, (token, _))| matches!(token, Token::LBrace).then_some(index));
    if let Some(parent) = parent {
        if matches!(
            tokens.get(brace.wrapping_sub(1)).map(|token| &token.0),
            Some(Token::Colon)
        ) && matches!(tokens.get(brace.wrapping_sub(2)).map(|token| &token.0), Some(Token::Ident(name)) if name == "config")
        {
            return match route_context_at_brace(tokens, parent)? {
                HttpRouteObjectContext::Route => Some(HttpRouteObjectContext::RouteConfig),
                HttpRouteObjectContext::WebSocket => Some(HttpRouteObjectContext::WebSocketConfig),
                _ => None,
            };
        }
        if matches!(
            tokens.get(brace.wrapping_sub(1)).map(|token| &token.0),
            Some(Token::Colon)
        ) && matches!(tokens.get(brace.wrapping_sub(2)).map(|token| &token.0), Some(Token::Ident(name)) if name == "rateLimit")
        {
            return match route_context_at_brace(tokens, parent)? {
                HttpRouteObjectContext::RouteConfig | HttpRouteObjectContext::WebSocketConfig => {
                    Some(HttpRouteObjectContext::RateLimit)
                }
                _ => None,
            };
        }
    }

    let mut depth = 0_i32;
    let mut opening = None;
    for index in (0..brace).rev() {
        match tokens[index].0 {
            Token::RParen => depth += 1,
            Token::LParen if depth == 0 => {
                opening = Some(index);
                break;
            }
            Token::LParen => depth -= 1,
            _ => {}
        }
    }
    let opening = opening?;
    let Token::Ident(method) = &tokens.get(opening.checked_sub(1)?)?.0 else {
        return None;
    };
    if method == "rateLimit" {
        return Some(HttpRouteObjectContext::RateLimit);
    }
    let has_first_argument = tokens[opening + 1..brace]
        .iter()
        .any(|(token, _)| matches!(token, Token::Comma));
    if !has_first_argument {
        return None;
    }
    match method.as_str() {
        "get" | "post" | "put" | "patch" | "delete" | "head" | "options" | "connect" | "query"
        | "trace" => Some(HttpRouteObjectContext::Route),
        "ws" => Some(HttpRouteObjectContext::WebSocket),
        _ => None,
    }
}

fn object_keys(tokens: &[(Token, std::ops::Range<usize>)], open: usize) -> HashSet<String> {
    let mut keys = HashSet::default();
    let mut depth = 1_i32;
    for index in open + 1..tokens.len() {
        match &tokens[index].0 {
            Token::LBrace | Token::LBracket | Token::LParen => depth += 1,
            Token::RBrace | Token::RBracket | Token::RParen => {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            Token::Ident(field) if depth == 1 => {
                if matches!(
                    tokens.get(index + 1).map(|token| &token.0),
                    Some(Token::Colon)
                ) {
                    keys.insert(field.clone());
                }
            }
            _ => {}
        }
    }
    keys
}

const HTTP_ROUTE_OPTIONS: &[(&str, &str, &str)] = &[
    (
        "handler",
        "(request: HttpRequest) => Promise<HttpReply>",
        "Handles one matching request.",
    ),
    (
        "response",
        "Response schema",
        "Names the response schema or status-to-schema map.",
    ),
    (
        "body",
        "Request body schema",
        "Names the request body schema.",
    ),
    (
        "params",
        "Path parameter schema",
        "Names the schema for parameters declared in the route path.",
    ),
    ("query", "Query schema", "Names the request query schema."),
    (
        "headers",
        "Header schema",
        "Names the request header schema.",
    ),
    (
        "cookies",
        "Cookie schema",
        "Names the request cookie schema.",
    ),
    (
        "preHandler",
        "((request: HttpRequest) => HttpResponse | true)[]",
        "Runs named synchronous guards before the route handler.",
    ),
    (
        "config",
        "{ bodyLimit?: i64, rateLimit?: RateLimit }",
        "Sets route-local body and rate limits.",
    ),
];

const WEBSOCKET_ROUTE_OPTIONS: &[(&str, &str, &str)] = &[
    (
        "handler",
        "(socket: WebSocket) => Promise<void>",
        "Handles one upgraded WebSocket connection.",
    ),
    (
        "origins",
        "string[]",
        "Lists browser origins allowed to upgrade.",
    ),
    (
        "subprotocols",
        "string[]",
        "Lists acceptable WebSocket subprotocols.",
    ),
    (
        "maxMessageBytes",
        "i64",
        "Bounds one complete WebSocket message.",
    ),
    (
        "preHandler",
        "((request: HttpRequest) => HttpResponse | true)[]",
        "Runs named synchronous guards before the upgrade.",
    ),
    (
        "config",
        "{ rateLimit?: RateLimit }",
        "Sets the upgrade's route-local rate limit.",
    ),
];

const HTTP_ROUTE_CONFIG_OPTIONS: &[(&str, &str, &str)] = &[
    (
        "bodyLimit",
        "i64",
        "Limits request body bytes for this route.",
    ),
    (
        "rateLimit",
        "RateLimit",
        "Limits requests accepted by this route.",
    ),
];

const RATE_LIMIT_OPTIONS: &[(&str, &str, &str)] = &[
    (
        "max",
        "i64",
        "Limits requests for one client identity during `timeWindow`.",
    ),
    (
        "timeWindow",
        "string",
        "Sets the token bucket refill duration, such as `\"1m\"`.",
    ),
    (
        "globalMax",
        "i64?",
        "Optionally caps requests from all identities together during `timeWindow`.",
    ),
    (
        "ban",
        "i64?",
        "After this many consecutive exceeded identity limits, returns 403 until `banWindow` expires. `0` bans on the first exceeded request.",
    ),
    (
        "banWindow",
        "string?",
        "Sets the temporary ban duration; defaults to `timeWindow` and requires `ban`.",
    ),
];

fn route_option_fields(
    context: HttpRouteObjectContext,
) -> &'static [(&'static str, &'static str, &'static str)] {
    match context {
        HttpRouteObjectContext::Route => HTTP_ROUTE_OPTIONS,
        HttpRouteObjectContext::WebSocket => WEBSOCKET_ROUTE_OPTIONS,
        HttpRouteObjectContext::RouteConfig => HTTP_ROUTE_CONFIG_OPTIONS,
        HttpRouteObjectContext::WebSocketConfig => &HTTP_ROUTE_CONFIG_OPTIONS[1..],
        HttpRouteObjectContext::RateLimit => RATE_LIMIT_OPTIONS,
    }
}

pub(super) fn http_route_option_completion_items(
    source: &str,
    offset: usize,
) -> Option<Vec<CompletionItem>> {
    let tokens = tyrus_core::lexer::lex_spanned(source).ok()?;
    let open = open_object_brace(&tokens, offset)?;
    let context = route_context_at_brace(&tokens, open)?;
    let existing = object_keys(&tokens, open);
    Some(
        route_option_fields(context)
            .iter()
            .filter(|(name, _, _)| !existing.contains(*name))
            .map(|(name, detail, documentation)| CompletionItem {
                label: (*name).to_string(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some((*detail).to_string()),
                documentation: Some(Documentation::String((*documentation).to_string())),
                insert_text: Some(format!("{name}: ")),
                sort_text: Some(format!("0_route_option_{name}")),
                ..CompletionItem::default()
            })
            .collect(),
    )
}

pub(super) fn http_route_option_hover_at(
    source: &str,
    offset: usize,
) -> Option<(std::ops::Range<usize>, String)> {
    let tokens = tyrus_core::lexer::lex_spanned(source).ok()?;
    let (index, (Token::Ident(name), range)) = tokens
        .iter()
        .enumerate()
        .find(|(_, (_, range))| range.contains(&offset))?
    else {
        return None;
    };
    if !matches!(
        tokens.get(index + 1).map(|token| &token.0),
        Some(Token::Colon)
    ) {
        return None;
    }
    let open = open_object_brace(&tokens, range.start)?;
    let context = route_context_at_brace(&tokens, open)?;
    let (_, detail, documentation) = route_option_fields(context)
        .iter()
        .find(|(candidate, _, _)| candidate == name)?;
    Some((
        range.clone(),
        format!("```tyrus\n{name}: {detail}\n```\n\n{documentation}"),
    ))
}

pub(super) fn object_field_completion_items(
    fields: &[Field],
    existing: &std::collections::HashSet<String>,
) -> Vec<CompletionItem> {
    let missing: Vec<&Field> = fields
        .iter()
        .filter(|field| !existing.contains(&field.name))
        .collect();

    let mut items = Vec::new();

    if missing.len() > 1 {
        let body = missing
            .iter()
            .enumerate()
            .map(|(index, field)| format!("{}: ${{{}}},", field.name, index + 1))
            .collect::<Vec<_>>()
            .join("\n");
        items.push(CompletionItem {
            label: "all".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some(format!("fill all {} remaining fields", missing.len())),
            sort_text: Some("0".to_string()),
            filter_text: Some("all".to_string()),
            insert_text: Some(body),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..CompletionItem::default()
        });
    }

    for field in missing {
        let optional = if field.optional { "?" } else { "" };
        let readonly = if field.readonly { "readonly " } else { "" };
        items.push(CompletionItem {
            label: format!("{}{optional}", field.name),
            kind: Some(CompletionItemKind::FIELD),
            detail: Some(format!("{readonly}{}", field.ty)),
            filter_text: Some(field.name.clone()),
            sort_text: Some(format!("1_{}", field.name)),
            insert_text: Some(format!("{}: ", field.name)),
            ..CompletionItem::default()
        });
    }

    items
}

pub(super) fn interface_object_snippet_item(name: &str, fields: &[Field]) -> CompletionItem {
    let body = if fields.is_empty() {
        String::new()
    } else {
        let lines = fields
            .iter()
            .enumerate()
            .map(|(index, field)| format!("    {}: ${{{}}},", field.name, index + 2))
            .collect::<Vec<_>>()
            .join("\n");
        format!("\n{lines}\n")
    };
    let insert_text = format!("const ${{1:value}}: {name} = {{{body}}};$0");
    CompletionItem {
        label: format!("{name} (object)"),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: Some(format!("create a {name} object literal")),
        filter_text: Some(name.to_string()),
        insert_text: Some(insert_text),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..CompletionItem::default()
    }
}

pub(super) fn apply_function_snippet(item: &mut CompletionItem, name: &str, parameters: &[String]) {
    apply_function_snippet_with_terminator(item, name, parameters, false);
}

pub(super) fn apply_function_snippet_with_terminator(
    item: &mut CompletionItem,
    name: &str,
    parameters: &[String],
    terminate_statement: bool,
) {
    if !matches!(
        item.kind,
        Some(CompletionItemKind::FUNCTION | CompletionItemKind::METHOD)
    ) {
        return;
    }
    let arguments = parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| format!("${{{}:{parameter}}}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    let suffix = if terminate_statement { ";$0" } else { "" };
    item.insert_text = Some(format!("{name}({arguments}){suffix}"));
    item.insert_text_format = Some(InsertTextFormat::SNIPPET);
}

pub(super) fn completion_starts_statement(source: &str, offset: usize) -> bool {
    let Some(prefix) = source.get(..offset) else {
        return false;
    };
    let before_candidate = prefix.trim_end_matches(|character: char| {
        character.is_ascii_alphanumeric() || character == '_' || character == '.'
    });
    let current_line = before_candidate
        .rsplit('\n')
        .next()
        .unwrap_or(before_candidate);
    if current_line.trim().is_empty() {
        return true;
    }
    let before = before_candidate.trim_end();
    before
        .chars()
        .next_back()
        .is_some_and(|character| matches!(character, ';' | '{' | '}'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tyrus_core::analysis::analyze;

    #[test]
    fn keyword_completion_offers_typeof() {
        let item = keyword_completion_items()
            .into_iter()
            .find(|item| item.label == "typeof")
            .expect("typeof should be offered as a keyword completion");
        assert_eq!(item.kind, Some(CompletionItemKind::KEYWORD));
    }

    #[test]
    fn scans_enum_member_names_from_incomplete_source() {
        let source = "enum Status { Success, Error, Pending }\nconst s: Status = Status.\n";
        let members = enum_member_names(source, "Status").unwrap();
        assert_eq!(members, vec!["Success", "Error", "Pending"]);
    }

    #[test]
    fn scans_valued_enum_members_skipping_values() {
        let source = "enum HttpStatus { Ok = 200, NotFound = 404 }";
        assert_eq!(
            enum_member_names(source, "HttpStatus").unwrap(),
            vec!["Ok", "NotFound"]
        );
        let role = "enum Role: string { Admin = \"admin\", User = \"user\" }";
        assert_eq!(
            enum_member_names(role, "Role").unwrap(),
            vec!["Admin", "User"]
        );
    }

    #[test]
    fn member_completion_offers_only_enum_members_while_typing() {
        let source = "enum Status { Success, Error, Pending }\nconst main = (): void => {\n    const s: Status = Status.\n};\n";
        let offset = source.find("Status.\n").unwrap() + "Status.".len();
        let analysis = analyze(source);
        let items = member_completion_items(source, offset, &analysis).unwrap();
        let labels: Vec<_> = items.iter().map(|item| item.label.clone()).collect();
        assert_eq!(labels, vec!["Success", "Error", "Pending"]);
        assert!(
            items
                .iter()
                .all(|item| item.kind == Some(CompletionItemKind::ENUM_MEMBER))
        );
    }

    #[test]
    fn member_completion_offers_native_imported_enum_members_while_typing() {
        let source =
            "import { HttpStatus as Status } from \"tyrus:http\";\nconst code: Status = Status.\n";
        let offset = source.find("Status.\n").unwrap() + "Status.".len();
        let analysis = analyze(source);
        let items = member_completion_items(source, offset, &analysis).unwrap();
        let labels: Vec<_> = items.iter().map(|item| item.label.clone()).collect();
        assert!(labels.contains(&"Ok".to_string()));
        assert!(labels.contains(&"Created".to_string()));
        assert!(labels.contains(&"NotFound".to_string()));
        assert!(
            items
                .iter()
                .all(|item| item.kind == Some(CompletionItemKind::ENUM_MEMBER))
        );
    }

    fn test_completion_labels(source: &str) -> Option<Vec<String>> {
        let offset = source.len();
        let analysis = analyze(source);
        member_completion_items(source, offset, &analysis)
            .map(|items| items.into_iter().map(|item| item.label).collect())
    }

    #[test]
    fn member_completion_offers_matchers_after_expect() {
        let labels = test_completion_labels(
            "import { expect } from \"tyrus:test\";\nexpect(sum(\")\", 2)).",
        )
        .unwrap();
        assert_eq!(
            labels.len(),
            tyrus_core::test::MATCHERS.len() + 3,
            "{labels:?}"
        );
        for expected in [
            "toBe",
            "toEqual",
            "toThrow",
            "toBeCloseTo",
            "not",
            "rejects",
            "resolves",
        ] {
            assert!(labels.iter().any(|label| label == expected), "{labels:?}");
        }
    }

    #[test]
    fn member_completion_after_a_modifier_stops_offering_it() {
        let labels =
            test_completion_labels("import { expect } from \"tyrus:test\";\nexpect(1).not.")
                .unwrap();
        assert!(labels.iter().any(|label| label == "toBe"), "{labels:?}");
        assert!(!labels.iter().any(|label| label == "not"), "{labels:?}");
        for chain in ["rejects", "resolves"] {
            let labels = test_completion_labels(&format!(
                "import {{ expect }} from \"tyrus:test\";\nexpect(load()).{chain}."
            ))
            .unwrap();
            assert!(labels.iter().any(|label| label == "toThrow"), "{labels:?}");
            assert!(labels.iter().any(|label| label == "not"), "{labels:?}");
            assert!(!labels.iter().any(|label| label == "rejects"), "{labels:?}");
            assert!(
                !labels.iter().any(|label| label == "resolves"),
                "{labels:?}"
            );
        }
    }

    #[test]
    fn member_completion_offers_modifiers_through_import_aliases() {
        let source = "import { describe, test as check } from \"tyrus:test\";\ncheck.";
        assert_eq!(
            test_completion_labels(source).unwrap(),
            ["skip", "only", "todo", "each"]
        );
        let source = "import { describe, test as check } from \"tyrus:test\";\ndescribe.";
        assert_eq!(test_completion_labels(source).unwrap(), ["skip", "only"]);
        let items = member_completion_items(source, source.len(), &analyze(source)).unwrap();
        assert_eq!(
            items[0].detail.as_deref(),
            Some("(modifier) describe.skip(name, callback)")
        );
    }

    #[test]
    fn member_completion_filters_test_modifiers_while_typing() {
        for (receiver, partial, expected) in [
            ("test", "s", "skip"),
            ("test", "t", "todo"),
            ("test", "e", "each"),
            ("it", "s", "skip"),
            ("it", "t", "todo"),
            ("it", "e", "each"),
        ] {
            let source =
                format!("import {{ test, it }} from \"tyrus:test\";\n{receiver}.{partial}");
            assert_eq!(test_completion_labels(&source).unwrap(), [expected]);
        }
    }

    #[test]
    fn member_completion_ignores_expect_without_the_test_import() {
        let labels = test_completion_labels("const expect = (n: i32): i32 => n;\nexpect(1).");
        assert!(
            labels
                .as_ref()
                .is_none_or(|labels| !labels.iter().any(|label| label == "toBe")),
            "{labels:?}"
        );
    }

    #[test]
    fn member_completion_ignores_unknown_receiver() {
        let source = "const x: i32 = 1;\nx.\n";
        let offset = source.find("x.\n").unwrap() + "x.".len();
        let analysis = analyze(source);
        let items = member_completion_items(source, offset, &analysis).unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn member_completion_offers_static_json_decoding_for_data_interfaces() {
        let source = "interface User { name: string; }\nUser.\n";
        let offset = source.find("User.\n").unwrap() + "User.".len();
        let analysis = analyze(source);
        let items = member_completion_items(source, offset, &analysis).unwrap();
        let method = items
            .iter()
            .find(|item| item.label == "fromJson")
            .expect("fromJson should be offered for data interfaces");
        assert_eq!(
            method.detail.as_deref(),
            Some("(User) fromJson(text: string): Result<User, JsonError>")
        );
    }

    #[test]
    fn member_completion_offers_map_operations() {
        let source =
            "const main = (): void => {\n    let m: Record<string, i32> = { a: 1 };\n    m.\n};\n";
        let offset = source.find("m.\n").unwrap() + "m.".len();
        let analysis = analyze(source);
        let items = member_completion_items(source, offset, &analysis).unwrap();
        let labels: Vec<_> = items.iter().map(|item| item.label.clone()).collect();
        for expected in ["has", "delete", "keys", "size"] {
            assert!(labels.contains(&expected.to_string()), "missing {expected}");
        }
    }

    #[test]
    fn member_completion_offers_the_websocket_binary_duplex() {
        let source = "import { WebSocket } from \"tyrus:http\";\nconst socket: WebSocket = socket;\nsocket.\n";
        let offset = source.rfind("socket.").unwrap() + "socket.".len();
        let analysis = analyze(source);
        let items = member_completion_items(source, offset, &analysis).unwrap();
        let stream = items
            .iter()
            .find(|item| item.label == "stream")
            .expect("WebSocket.stream should be offered");
        assert_eq!(stream.kind, Some(CompletionItemKind::METHOD));
        assert_eq!(
            stream.detail.as_deref(),
            Some("() => Promise<DuplexStream<u8[], u8[]>>")
        );
        assert!(stream
            .documentation
            .as_ref()
            .is_some_and(|documentation| matches!(documentation, Documentation::String(text) if text.contains("complete binary"))));
    }

    #[test]
    fn route_option_completion_includes_http_and_nested_config_keys() {
        let source = "router.get(\"/users/:id\", { \n";
        let items = http_route_option_completion_items(source, source.len()).unwrap();
        let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
        for expected in ["handler", "response", "params", "config", "preHandler"] {
            assert!(labels.contains(&expected), "missing {expected}");
        }

        let source = "router.get(\"/users\", { config: { \n";
        let items = http_route_option_completion_items(source, source.len()).unwrap();
        let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, vec!["bodyLimit", "rateLimit"]);

        let source = "router.get(\"/users\", { config: { rateLimit: { \n";
        let items = http_route_option_completion_items(source, source.len()).unwrap();
        let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(
            labels,
            vec!["max", "timeWindow", "globalMax", "ban", "banWindow"]
        );

        let source = "router.rateLimit({ \n";
        let items = http_route_option_completion_items(source, source.len()).unwrap();
        let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(
            labels,
            vec!["max", "timeWindow", "globalMax", "ban", "banWindow"]
        );
    }

    #[test]
    fn route_option_completion_distinguishes_websocket_keys_and_hover() {
        let source = "router.ws(\"/chat\", { \n";
        let items = http_route_option_completion_items(source, source.len()).unwrap();
        let labels: Vec<_> = items.iter().map(|item| item.label.as_str()).collect();
        assert!(labels.contains(&"handler"));
        assert!(labels.contains(&"maxMessageBytes"));
        assert!(!labels.contains(&"response"));

        let source = "router.get(\"/users\", { response: Users });";
        let offset = source.find("response").unwrap() + 2;
        let (range, markdown) = http_route_option_hover_at(source, offset).unwrap();
        assert_eq!(&source[range], "response");
        assert!(markdown.contains("Response schema"));

        let source = "router.get(\"/users\", { config: { rateLimit: { globalMax: 50 } } });";
        let offset = source.find("globalMax").unwrap() + 2;
        let (range, markdown) = http_route_option_hover_at(source, offset).unwrap();
        assert_eq!(&source[range], "globalMax");
        assert!(markdown.contains("all identities"));
    }

    #[test]
    fn member_completion_offers_behavior_interface_methods() {
        let source = r#"
            interface User { age: u32; }
            interface UserMethods at User {
                isAdult(user: User): boolean { return user.age >= 18; }
            }
            const user: User = { age: 21 };
            user.
        "#;
        let offset = source.rfind("user.").unwrap() + "user.".len();
        let analysis = analyze(source);
        let items = member_completion_items(source, offset, &analysis).unwrap();
        let method = items
            .iter()
            .find(|item| item.label == "isAdult")
            .expect("behavior method should be offered");
        assert_eq!(method.kind, Some(CompletionItemKind::METHOD));
        assert!(
            method
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("isAdult(): boolean"))
        );
    }

    #[test]
    fn member_completion_offers_automatic_debug_for_data_interfaces() {
        let source = r#"
            interface User { name: string; age: i32; }
            const user: User = { name: "Ada", age: 30 };
            user.
        "#;
        let offset = source.rfind("user.").unwrap() + "user.".len();
        let analysis = analyze(source);
        let items = member_completion_items(source, offset, &analysis).unwrap();
        let method = items
            .iter()
            .find(|item| item.label == "toString")
            .expect("automatic debug method should be offered");
        assert_eq!(method.kind, Some(CompletionItemKind::METHOD));
        assert_eq!(method.detail.as_deref(), Some("(User) toString(): string"));
        assert!(
            items
                .iter()
                .any(|item| item.detail.as_deref() == Some("(User) copy(): User"))
        );
        assert!(
            items
                .iter()
                .any(|item| item.detail.as_deref() == Some("(User) equals(other: User): boolean"))
        );
    }

    #[test]
    fn member_completion_offers_behavior_methods_for_primitive_receivers() {
        let source = r#"
            interface NumberMethods at i32 {
                isPositive(value: i32): boolean { return value > 0; }
            }
            const number: i32 = 4;
            number.
        "#;
        let offset = source.rfind("number.").unwrap() + "number.".len();
        let analysis = analyze(source);
        let items = member_completion_items(source, offset, &analysis).unwrap();
        let method = items
            .iter()
            .find(|item| item.label == "isPositive")
            .expect("primitive behavior method should be offered");
        assert_eq!(method.kind, Some(CompletionItemKind::METHOD));
        assert!(
            method
                .detail
                .as_deref()
                .is_some_and(|detail| detail.contains("isPositive(): boolean"))
        );
    }

    #[test]
    fn member_completion_offers_imported_behavior_methods() {
        let source = r#"
            import { User, UserMethods } from "./model";
            const user: User = { age: 21 };
            user.
        "#;
        let model = r#"
            export interface User { age: u32; }
            export const ageText = (user: User): string => user.age.toString();
            export interface UserMethods at User { ageText; }
        "#;
        let program = Parser::from_source(model).unwrap().parse_program().unwrap();
        let behavior = program
            .items
            .iter()
            .find_map(|item| match item {
                Item::BehaviorInterfaceDecl { .. } => {
                    tyrus_core::module_loader::behavior_interface_info(item, &program.items)
                }
                _ => None,
            })
            .expect("behavior interface should parse");
        let mut imports = HashMap::default();
        imports.insert("UserMethods".to_string(), behavior);
        let offset = source.rfind("user.").unwrap() + "user.".len();
        let analysis = analyze(source);
        let items =
            member_completion_items_with_behaviors(source, offset, &analysis, &imports).unwrap();
        assert!(items.iter().any(|item| item.label == "ageText"));
    }

    #[test]
    fn member_completion_offers_local_shorthand_behavior_bindings() {
        let source = r#"
            interface User { age: u32; }
            const ageText = (user: User): string => user.age.toString();
            interface UserMethods at User { ageText; }
            const user: User = { age: 21 };
            user.
        "#;
        let offset = source.rfind("user.").unwrap() + "user.".len();
        let analysis = analyze(source);
        let items = member_completion_items(source, offset, &analysis).unwrap();
        assert!(items.iter().any(|item| item.label == "ageText"));
    }
}
