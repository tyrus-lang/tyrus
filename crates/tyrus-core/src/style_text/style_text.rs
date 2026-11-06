use crate::ast::Expr;
use crate::ast::ExprKind;
use anstyle::{AnsiColor, Effects, Style};

pub const STYLE_TEXT_MARKER: &str = "__StyleText";

pub enum StyleTextForm<'a> {
    Styled { styles: Vec<String>, text: &'a Expr },
    Arity,
    NonLiteralStyle,
}

pub fn as_style_text<'a>(
    expr: &'a Expr,
    is_style_text: &impl Fn(&str) -> bool,
) -> Option<StyleTextForm<'a>> {
    match expr {
        Expr {
            kind: ExprKind::FnCall { callee, args, .. },
            ..
        } if is_style_text(callee) => {
            if args.len() != 2 {
                return Some(StyleTextForm::Arity);
            }
            let (format, text) = if is_format(&args[0]) {
                (&args[0], &args[1])
            } else if is_format(&args[1]) {
                (&args[1], &args[0])
            } else {
                (&args[0], &args[1])
            };
            Some(match extract_styles(format) {
                Some(styles) => StyleTextForm::Styled { styles, text },
                None => StyleTextForm::NonLiteralStyle,
            })
        }
        Expr {
            kind:
                ExprKind::MethodCall {
                    object,
                    method,
                    args,
                    ..
                },
            ..
        } => {
            let mut styles = chain_styles(object, is_style_text)?;
            styles.push(method.clone());
            if args.len() != 1 {
                return Some(StyleTextForm::Arity);
            }
            Some(StyleTextForm::Styled {
                styles,
                text: &args[0],
            })
        }
        _ => None,
    }
}

pub fn is_incomplete_style_text(expr: &Expr, is_style_text: &impl Fn(&str) -> bool) -> bool {
    match expr.kind() {
        ExprKind::Ident(name) => is_style_text(name),
        ExprKind::FieldAccess { .. } => chain_styles(expr, is_style_text).is_some(),
        _ => false,
    }
}

fn chain_styles(expr: &Expr, is_style_text: &impl Fn(&str) -> bool) -> Option<Vec<String>> {
    match expr.kind() {
        ExprKind::Ident(name) if is_style_text(name) => Some(Vec::new()),
        ExprKind::FieldAccess { object, field } => {
            let mut styles = chain_styles(object, is_style_text)?;
            styles.push(field.clone());
            Some(styles)
        }
        _ => None,
    }
}

fn is_format(expr: &Expr) -> bool {
    match expr.kind() {
        ExprKind::ArrayLit(items) => {
            !items.is_empty()
                && items
                    .iter()
                    .all(|item| matches!(item.kind(), ExprKind::StrLit(_)))
        }
        ExprKind::StrLit(value) => is_known_style(value),
        _ => false,
    }
}

fn extract_styles(format: &Expr) -> Option<Vec<String>> {
    match format.kind() {
        ExprKind::StrLit(value) => Some(vec![value.clone()]),
        ExprKind::ArrayLit(items) => items
            .iter()
            .map(|item| match item {
                Expr {
                    kind: ExprKind::StrLit(value),
                    ..
                } => Some(value.clone()),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

pub fn ansi_open(styles: &[String]) -> Result<String, String> {
    let mut style = Style::new();
    for name in styles {
        style = apply(style, name).ok_or_else(|| name.clone())?;
    }
    Ok(style.render().to_string())
}

pub const ANSI_RESET: &str = "\x1b[0m";

pub const STYLE_NAMES: &[&str] = &[
    "reset",
    "bold",
    "dim",
    "italic",
    "underline",
    "doubleunderline",
    "blink",
    "inverse",
    "hidden",
    "strikethrough",
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "gray",
    "blackBright",
    "redBright",
    "greenBright",
    "yellowBright",
    "blueBright",
    "magentaBright",
    "cyanBright",
    "whiteBright",
    "bgBlack",
    "bgRed",
    "bgGreen",
    "bgYellow",
    "bgBlue",
    "bgMagenta",
    "bgCyan",
    "bgWhite",
    "bgGray",
    "bgBlackBright",
    "bgRedBright",
    "bgGreenBright",
    "bgYellowBright",
    "bgBlueBright",
    "bgMagentaBright",
    "bgCyanBright",
    "bgWhiteBright",
];

pub fn is_known_style(name: &str) -> bool {
    apply(Style::new(), name).is_some()
}

fn apply(style: Style, name: &str) -> Option<Style> {
    let effect = |style: Style, effect: Effects| style.effects(style.get_effects() | effect);
    Some(match name {
        "reset" => Style::new(),
        "bold" => effect(style, Effects::BOLD),
        "dim" | "faint" => effect(style, Effects::DIMMED),
        "italic" => effect(style, Effects::ITALIC),
        "underline" => effect(style, Effects::UNDERLINE),
        "doubleunderline" => effect(style, Effects::DOUBLE_UNDERLINE),
        "blink" => effect(style, Effects::BLINK),
        "inverse" => effect(style, Effects::INVERT),
        "hidden" => effect(style, Effects::HIDDEN),
        "strikethrough" => effect(style, Effects::STRIKETHROUGH),
        _ => {
            if let Some(color) = foreground(name) {
                style.fg_color(Some(color.into()))
            } else {
                let color = name.strip_prefix("bg").and_then(background)?;
                style.bg_color(Some(color.into()))
            }
        }
    })
}

fn foreground(name: &str) -> Option<AnsiColor> {
    Some(match name {
        "black" => AnsiColor::Black,
        "red" => AnsiColor::Red,
        "green" => AnsiColor::Green,
        "yellow" => AnsiColor::Yellow,
        "blue" => AnsiColor::Blue,
        "magenta" => AnsiColor::Magenta,
        "cyan" => AnsiColor::Cyan,
        "white" => AnsiColor::White,
        "gray" | "grey" | "blackBright" => AnsiColor::BrightBlack,
        "redBright" => AnsiColor::BrightRed,
        "greenBright" => AnsiColor::BrightGreen,
        "yellowBright" => AnsiColor::BrightYellow,
        "blueBright" => AnsiColor::BrightBlue,
        "magentaBright" => AnsiColor::BrightMagenta,
        "cyanBright" => AnsiColor::BrightCyan,
        "whiteBright" => AnsiColor::BrightWhite,
        _ => return None,
    })
}

fn background(rest: &str) -> Option<AnsiColor> {
    let mut chars = rest.chars();
    let first = chars.next()?.to_ascii_lowercase();
    let name: String = std::iter::once(first).chain(chars).collect();
    foreground(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;
    use crate::parser::Parser;

    fn expr(source: &str) -> Expr {
        let src = format!("const x = {source};\n");
        let tokens = lex(&src).unwrap();
        let program = Parser::new(tokens).parse_program().unwrap();
        match &program.items[0] {
            crate::ast::Item::Stmt(crate::ast::Stmt {
                kind: crate::ast::StmtKind::VarDecl { value: Some(v), .. },
                ..
            }) => v.clone(),
            _ => panic!("expected a variable declaration"),
        }
    }

    fn styles(source: &str) -> Vec<String> {
        let is_st = |name: &str| name == "styleText";
        match as_style_text(&expr(source), &is_st) {
            Some(StyleTextForm::Styled { styles, .. }) => styles,
            other => panic!(
                "expected styled, got a different form (has form: {})",
                other.is_some()
            ),
        }
    }

    #[test]
    fn every_completion_name_is_a_known_style() {
        for name in STYLE_NAMES {
            assert!(is_known_style(name), "`{name}` should be a known style");
        }
    }

    #[test]
    fn renders_ansi_open_from_style_names() {
        assert_eq!(ansi_open(&["green".into()]).unwrap(), "\u{1b}[32m");
        assert_eq!(
            ansi_open(&["bold".into(), "red".into()]).unwrap(),
            "\u{1b}[1m\u{1b}[31m"
        );
        assert_eq!(ansi_open(&["bgRed".into()]).unwrap(), "\u{1b}[41m");
        assert_eq!(ansi_open(&["bgRedBright".into()]).unwrap(), "\u{1b}[101m");
        assert_eq!(ansi_open(&["nope".into()]), Err("nope".to_string()));
        assert!(is_known_style("underline") && !is_known_style("nope"));
    }

    #[test]
    fn recognizes_the_three_syntaxes() {
        assert_eq!(styles("styleText(\"green\", \"hi\")"), ["green"]);
        assert_eq!(
            styles("styleText([\"bold\", \"red\"], \"hi\")"),
            ["bold", "red"]
        );
        assert_eq!(
            styles("styleText.cyan.underline(\"hi\")"),
            ["cyan", "underline"]
        );
    }

    #[test]
    fn disambiguates_format_by_content_for_the_pipe_order() {
        assert_eq!(styles("styleText(\"hi\", [\"magenta\"])"), ["magenta"]);
        assert_eq!(styles("styleText(\"green\", \"red\")"), ["green"]);
    }

    #[test]
    fn reports_arity_and_non_literal() {
        let is_st = |name: &str| name == "styleText";
        assert!(matches!(
            as_style_text(&expr("styleText([\"red\"])"), &is_st),
            Some(StyleTextForm::Arity)
        ));
        assert!(matches!(
            as_style_text(&expr("styleText(fmt, \"hi\")"), &is_st),
            Some(StyleTextForm::NonLiteralStyle)
        ));
        assert!(as_style_text(&expr("other(\"x\")"), &is_st).is_none());
    }
}
