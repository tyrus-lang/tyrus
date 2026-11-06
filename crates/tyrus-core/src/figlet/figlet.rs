use crate::ast::Expr;
use crate::ast::ExprKind;

pub const FIGLET_MARKER: &str = "__Figlet";

pub const DEFAULT_FONT: &str = "standard";

pub const FONT_NAMES: &[&str] = &["standard", "small", "big", "slant"];

pub fn is_known_font(name: &str) -> bool {
    FONT_NAMES.contains(&name)
}

pub enum FigletForm<'a> {
    Rendered {
        text: &'a Expr,
        font: Option<&'a str>,
    },
    Arity,
    NonLiteralFont(&'a Expr),
}

pub fn as_figlet<'a>(expr: &'a Expr, is_figlet: &impl Fn(&str) -> bool) -> Option<FigletForm<'a>> {
    match expr.kind() {
        ExprKind::FnCall { callee, args, .. } if is_figlet(callee) => Some(match args.as_slice() {
            [text] => FigletForm::Rendered { text, font: None },
            [text, font] => match font {
                Expr {
                    kind: ExprKind::StrLit(name),
                    ..
                } => FigletForm::Rendered {
                    text,
                    font: Some(name),
                },
                _ => FigletForm::NonLiteralFont(font),
            },
            _ => FigletForm::Arity,
        }),
        _ => None,
    }
}

pub fn is_incomplete_figlet(expr: &Expr, is_figlet: &impl Fn(&str) -> bool) -> bool {
    matches!(expr.kind(), ExprKind::Ident(name) if is_figlet(name))
}
