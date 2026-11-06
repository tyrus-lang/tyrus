use super::*;

impl TypeChecker {
    pub(super) fn check_text_receiver_method(
        &mut self,
        method: &str,
        args: &[Expr],
        object_ty: &ValueType,
        optional_receiver: bool,
    ) -> Option<ValueType> {
        if let ValueType::Known(Type::Char) = &object_ty {
            let (arity, return_type) = match method {
                "toUpperCase" | "toLowerCase" => (0, Type::Char),
                "isDigit" | "isAlphabetic" | "isAlphanumeric" | "isWhitespace" => (0, Type::Bool),
                "toString" => (0, Type::Str),
                other => {
                    self.error(
                        "E0599",
                        format!(
                            "no method named `{other}` for a `char`; use `toUpperCase`, `toLowerCase`, `isDigit`, `isAlphabetic`, `isAlphanumeric`, `isWhitespace` or `toString`"
                        ),
                        Some(method),
                    );
                    return Some(ValueType::Error);
                }
            };
            if args.len() != arity {
                self.error(
                    "E0061",
                    format!(
                        "`{method}` takes {arity} argument(s), but {} were supplied",
                        args.len()
                    ),
                    Some(method),
                );
            }
            for arg in args {
                self.check_expr(arg);
            }
            return Some(optional_value_type(
                ValueType::Known(return_type),
                optional_receiver,
            ));
        }
        if let ValueType::Known(Type::Str) = &object_ty {
            let (minimum, maximum, return_type) = match method {
                "charAt" => (1, 1, Type::Str),
                "charCodeAt" => (1, 1, Type::I32),
                "indexOf" => (1, 2, Type::I64),
                "lastIndexOf" => (1, 2, Type::I64),
                "includes" => (1, 1, Type::Bool),
                "startsWith" | "endsWith" => (1, 1, Type::Bool),
                "copy" => (0, 0, Type::Str),
                "slice" => (1, 2, Type::Str),
                "substring" => (1, 2, Type::Str),
                "trim" | "trimStart" | "trimEnd" => (0, 0, Type::Str),
                "toLowerCase" | "toUpperCase" => (0, 0, Type::Str),
                "match" => (
                    1,
                    1,
                    Type::Optional(Box::new(Type::Array(Box::new(Type::Str)))),
                ),
                "search" => (1, 1, Type::I32),
                "split" => (1, 1, Type::Array(Box::new(Type::Str))),
                "chars" => (0, 0, Type::Array(Box::new(Type::Char))),
                "graphemes" => (0, 0, Type::Array(Box::new(Type::Str))),
                "matchAll" => (
                    1,
                    1,
                    Type::Array(Box::new(Type::Array(Box::new(Type::Str)))),
                ),
                "concat" => (1, usize::MAX, Type::Str),
                "replace" => (2, 2, Type::Str),
                "repeat" => (1, 1, Type::Str),
                "padStart" | "padEnd" => (1, 2, Type::Str),
                "count" => (0, 1, Type::Usize),
                "toSlugCase" | "toDashCase" | "toKebabCase" | "toPascalCase" | "toCamelCase"
                | "toSnakeCase" | "stripHtml" | "escapeHtml" | "urlEncode" | "urlDecode" => {
                    (0, 0, Type::Str)
                }
                "truncate" | "truncateEllipsis" => (1, 1, Type::Str),
                _ => {
                    self.error(
                        "E0599",
                        format!("no method named `{method}` for type `string`",),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Error);
                }
            };
            if args.len() < minimum || args.len() > maximum {
                self.error(
                    "E0061",
                    format!("method `{method}` expects {minimum} to {maximum} argument(s), but {} were provided", args.len()),
                    Some(method),
                );
            }
            if matches!(
                method,
                "charAt"
                    | "charCodeAt"
                    | "indexOf"
                    | "lastIndexOf"
                    | "slice"
                    | "substring"
                    | "repeat"
                    | "padStart"
                    | "padEnd"
                    | "truncate"
                    | "truncateEllipsis"
            ) {
                for (index, arg) in args.iter().enumerate() {
                    let actual = self.check_expr(arg);
                    let expected = match method {
                        "charAt" | "charCodeAt" => Some(Type::Usize),
                        "indexOf" | "lastIndexOf" => {
                            if index == 0 {
                                Some(Type::Str)
                            } else {
                                Some(Type::I64)
                            }
                        }
                        "slice" | "substring" => Some(Type::I64),
                        "includes" | "startsWith" | "endsWith" => Some(Type::Str),
                        "split" => Some(Type::Str),
                        "replace" => Some(Type::Str),
                        "repeat" | "truncate" | "truncateEllipsis" => Some(Type::Usize),
                        "padStart" | "padEnd" => {
                            if index == 0 {
                                Some(Type::Usize)
                            } else {
                                Some(Type::Str)
                            }
                        }
                        "concat" => Some(Type::Str),
                        _ => None,
                    };
                    if let Some(expected) = expected {
                        self.require_assignable(&expected, &actual, method, "E0308");
                    }
                }
            } else {
                for (index, arg) in args.iter().enumerate() {
                    let actual = self.check_expr(arg);
                    let expected = match method {
                        "includes" | "startsWith" | "endsWith" => Some(Type::Str),
                        "match" | "search" => Some(Type::Regex),
                        "split" => {
                            if self.is_assignable(&Type::Regex, &actual) {
                                None
                            } else {
                                Some(Type::Str)
                            }
                        }
                        "replace" => {
                            if index == 0 {
                                if self.is_assignable(&Type::Regex, &actual) {
                                    None
                                } else {
                                    Some(Type::Str)
                                }
                            } else {
                                Some(Type::Str)
                            }
                        }
                        "matchAll" => Some(Type::Regex),
                        "concat" => Some(Type::Str),
                        "padStart" | "padEnd" => {
                            if index == 0 {
                                None
                            } else {
                                Some(Type::Str)
                            }
                        }
                        "count" => Some(Type::Str),
                        _ => None,
                    };
                    if let Some(expected) = expected {
                        self.require_assignable(&expected, &actual, method, "E0308");
                    }
                }
            }
            return Some(optional_value_type(
                ValueType::Known(return_type),
                optional_receiver,
            ));
        }
        if let ValueType::Known(Type::Regex) = &object_ty {
            let (minimum, maximum, return_type) = match method {
                "test" => (1, 1, Type::Bool),
                "exec" => (
                    1,
                    1,
                    Type::Optional(Box::new(Type::Array(Box::new(Type::Str)))),
                ),
                "execAll" => (
                    1,
                    1,
                    Type::Optional(Box::new(Type::Array(Box::new(Type::Array(Box::new(
                        Type::Str,
                    )))))),
                ),
                "toString" => (0, 0, Type::Str),
                _ => {
                    self.error(
                        "E0599",
                        format!("no method named `{method}` for type `Regex`"),
                        Some(method),
                    );
                    for arg in args {
                        self.check_expr(arg);
                    }
                    return Some(ValueType::Error);
                }
            };
            if args.len() < minimum || args.len() > maximum {
                self.error(
                    "E0061",
                    format!("method `{method}` expects {minimum} to {maximum} argument(s), but {} were provided", args.len()),
                    Some(method),
                );
            }
            for arg in args {
                let actual = self.check_expr(arg);
                let expected = match method {
                    "test" | "exec" | "execAll" => Some(Type::Str),
                    _ => None,
                };
                if let Some(expected) = expected {
                    self.require_assignable(&expected, &actual, method, "E0308");
                }
            }
            return Some(optional_value_type(
                ValueType::Known(return_type),
                optional_receiver,
            ));
        }
        None
    }
}
