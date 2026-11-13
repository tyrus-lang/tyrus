#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservedKind {
    Active,
    Future,
    Builtin,
}

pub const ACTIVE_KEYWORDS: &[&str] = &[
    "Infinity",
    "NaN",
    "None",
    "and",
    "as",
    "async",
    "at",
    "await",
    "boolean",
    "break",
    "case",
    "catch",
    "char",
    "comptime",
    "const",
    "continue",
    "decimal",
    "default",
    "defer",
    "do",
    "else",
    "enum",
    "errdefer",
    "export",
    "extends",
    "f128",
    "f32",
    "f64",
    "false",
    "finally",
    "for",
    "from",
    "function",
    "i128",
    "i16",
    "i32",
    "i64",
    "i8",
    "if",
    "import",
    "in",
    "interface",
    "is",
    "isize",
    "let",
    "loop",
    "match",
    "never",
    "not",
    "null",
    "of",
    "onfail",
    "or",
    "protect",
    "readonly",
    "return",
    "string",
    "switch",
    "throw",
    "throws",
    "true",
    "try",
    "type",
    "typeof",
    "u128",
    "u16",
    "u32",
    "u64",
    "u8",
    "unknown",
    "usize",
    "validator",
    "void",
    "while",
];

pub const PRIMITIVE_TYPES: &[&str] = &[
    "boolean", "char", "decimal", "f128", "f32", "f64", "i128", "i16", "i32", "i64", "i8", "isize",
    "never", "string", "u128", "u16", "u32", "u64", "u8", "unknown", "usize", "void",
];

pub const LITERAL_KEYWORDS: &[&str] = &["Infinity", "NaN", "None", "false", "null", "true"];

pub const RESERVED_FUTURE: &[&str] = &[
    "abstract",
    "asserts",
    "class",
    "constructor",
    "debugger",
    "declare",
    "delete",
    "get",
    "implements",
    "infer",
    "instanceof",
    "intrinsic",
    "keyof",
    "module",
    "namespace",
    "new",
    "out",
    "override",
    "package",
    "private",
    "protected",
    "public",
    "require",
    "satisfies",
    "set",
    "static",
    "super",
    "this",
    "unique",
    "var",
    "when",
    "with",
    "yield",
];

pub const AMBIENT_BUILTINS: &[&str] = &[
    "Map",
    "Set",
    "assert",
    "assertEqual",
    "assertThrows",
    "console",
    "isNaN",
    "panic",
    "print",
    "printCR",
    "printError",
    "printRaw",
    "printWarn",
];

pub fn reserved_kind(name: &str) -> Option<ReservedKind> {
    if ACTIVE_KEYWORDS.contains(&name) {
        Some(ReservedKind::Active)
    } else if RESERVED_FUTURE.contains(&name) {
        Some(ReservedKind::Future)
    } else if AMBIENT_BUILTINS.contains(&name) {
        Some(ReservedKind::Builtin)
    } else {
        None
    }
}

pub fn is_reserved_future(name: &str) -> bool {
    RESERVED_FUTURE.contains(&name)
}

pub fn is_ambient_builtin(name: &str) -> bool {
    AMBIENT_BUILTINS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{Token, lex};

    fn is_sorted(list: &[&str]) -> bool {
        list.windows(2).all(|pair| pair[0] < pair[1])
    }

    #[test]
    fn lists_are_sorted_and_deduplicated() {
        assert!(is_sorted(ACTIVE_KEYWORDS), "ACTIVE_KEYWORDS must be sorted");
        assert!(is_sorted(RESERVED_FUTURE), "RESERVED_FUTURE must be sorted");
        assert!(
            is_sorted(AMBIENT_BUILTINS),
            "AMBIENT_BUILTINS must be sorted"
        );
    }

    #[test]
    fn lists_do_not_overlap() {
        for word in RESERVED_FUTURE {
            assert!(
                !ACTIVE_KEYWORDS.contains(word),
                "`{word}` is listed as both active and future-reserved"
            );
        }
        for word in AMBIENT_BUILTINS {
            assert!(
                !ACTIVE_KEYWORDS.contains(word),
                "`{word}` is listed as both active and an ambient builtin"
            );
            assert!(
                !RESERVED_FUTURE.contains(word),
                "`{word}` is listed as both future-reserved and an ambient builtin"
            );
        }
    }

    #[test]
    fn primitive_types_and_literals_are_active_and_sorted() {
        assert!(is_sorted(PRIMITIVE_TYPES), "PRIMITIVE_TYPES must be sorted");
        assert!(
            is_sorted(LITERAL_KEYWORDS),
            "LITERAL_KEYWORDS must be sorted"
        );
        for word in PRIMITIVE_TYPES.iter().chain(LITERAL_KEYWORDS) {
            assert!(
                ACTIVE_KEYWORDS.contains(word),
                "`{word}` must also be in ACTIVE_KEYWORDS"
            );
        }
    }

    #[test]
    fn active_keywords_lex_to_tokens() {
        for word in ACTIVE_KEYWORDS {
            let tokens = lex(word).expect("keyword should lex");
            assert_eq!(tokens.len(), 1, "`{word}` should lex to a single token");
            assert!(
                !matches!(tokens[0], Token::Ident(_)),
                "`{word}` is listed as active but lexes as an identifier"
            );
        }
    }

    #[test]
    fn future_keywords_lex_to_idents() {
        for word in RESERVED_FUTURE {
            let tokens = lex(word).expect("word should lex");
            assert_eq!(tokens.len(), 1, "`{word}` should lex to a single token");
            assert!(
                matches!(&tokens[0], Token::Ident(name) if name == word),
                "`{word}` is listed as future-reserved but already lexes to a keyword token"
            );
        }
    }

    #[test]
    fn ambient_builtins_lex_to_idents() {
        for word in AMBIENT_BUILTINS {
            let tokens = lex(word).expect("word should lex");
            assert_eq!(tokens.len(), 1, "`{word}` should lex to a single token");
            assert!(
                matches!(&tokens[0], Token::Ident(name) if name == word),
                "`{word}` is listed as an ambient builtin but already lexes to a keyword token"
            );
        }
    }

    #[test]
    fn classification_matches_lists() {
        assert_eq!(reserved_kind("const"), Some(ReservedKind::Active));
        assert_eq!(reserved_kind("and"), Some(ReservedKind::Active));
        assert_eq!(reserved_kind("class"), Some(ReservedKind::Future));
        assert_eq!(reserved_kind("print"), Some(ReservedKind::Builtin));
        assert_eq!(reserved_kind("console"), Some(ReservedKind::Builtin));
        assert_eq!(reserved_kind("myVariable"), None);
        assert!(is_reserved_future("class"));
        assert!(!is_reserved_future("const"));
        assert!(!is_reserved_future("and"));
        assert!(is_ambient_builtin("print"));
        assert!(is_ambient_builtin("printError"));
        assert!(is_ambient_builtin("printWarn"));
        assert!(is_ambient_builtin("printRaw"));
        assert!(is_ambient_builtin("printCR"));
        assert!(is_ambient_builtin("console"));
        assert!(is_ambient_builtin("assert"));
        assert!(is_ambient_builtin("assertEqual"));
        assert!(is_ambient_builtin("assertThrows"));
        assert!(!is_ambient_builtin("myFunction"));
    }
}
