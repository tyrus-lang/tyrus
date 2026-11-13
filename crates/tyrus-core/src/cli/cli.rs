use crate::ast::{Field, Type};
use crate::span::Span;

/// A prompt option interface: its name and its fields, in declaration order.
/// The field order is significant: it is the C-ABI struct layout the runtime
/// relies on, so runtime structs in `tyrus-runtime/src/cli.rs` must match it.
pub struct OptionInterface {
    pub name: &'static str,
    pub fields: Vec<Field>,
}

pub struct PromptBuiltin {
    pub name: &'static str,
    pub target: &'static str,
    pub options: &'static str,
    pub ret: Type,
}

fn required(name: &str, ty: Type) -> Field {
    Field {
        name: name.to_string(),
        ty,
        readonly: false,
        optional: false,
        decorators: Vec::new(),
        span: Span::DUMMY,
    }
}

fn optional(name: &str, ty: Type) -> Field {
    Field {
        name: name.to_string(),
        ty,
        readonly: false,
        optional: true,
        decorators: Vec::new(),
        span: Span::DUMMY,
    }
}

fn string_array() -> Type {
    Type::Array(Box::new(Type::Str))
}

pub fn option_interfaces() -> Vec<OptionInterface> {
    vec![
        OptionInterface {
            name: "InputOptions",
            fields: vec![
                required("message", Type::Str),
                optional("default", Type::Str),
                optional("placeholder", Type::Str),
                optional("help", Type::Str),
                optional("minLength", Type::I32),
                optional("maxLength", Type::I32),
                optional("pattern", Type::Regex),
                optional("patternMessage", Type::Str),
            ],
        },
        OptionInterface {
            name: "PasswordOptions",
            fields: vec![
                required("message", Type::Str),
                optional("help", Type::Str),
                optional("confirm", Type::Bool),
                optional("confirmMessage", Type::Str),
                optional("pattern", Type::Regex),
                optional("patternMessage", Type::Str),
            ],
        },
        OptionInterface {
            name: "ConfirmOptions",
            fields: vec![
                required("message", Type::Str),
                optional("default", Type::Bool),
                optional("help", Type::Str),
                optional("yes", Type::Str),
                optional("no", Type::Str),
            ],
        },
        OptionInterface {
            name: "SelectOptions",
            fields: vec![
                required("message", Type::Str),
                required("choices", string_array()),
                optional("default", Type::Str),
                optional("help", Type::Str),
                optional("pageSize", Type::I32),
            ],
        },
        OptionInterface {
            name: "MultiSelectOptions",
            fields: vec![
                required("message", Type::Str),
                required("choices", string_array()),
                optional("help", Type::Str),
                optional("pageSize", Type::I32),
                optional("required", Type::Bool),
            ],
        },
        OptionInterface {
            name: "NumberOptions",
            fields: vec![
                required("message", Type::Str),
                optional("default", Type::F64),
                optional("min", Type::F64),
                optional("max", Type::F64),
                optional("help", Type::Str),
            ],
        },
        OptionInterface {
            name: "IntegerOptions",
            fields: vec![
                required("message", Type::Str),
                optional("default", Type::I64),
                optional("min", Type::I64),
                optional("max", Type::I64),
                optional("help", Type::Str),
            ],
        },
    ]
}

pub fn prompt_builtins() -> Vec<PromptBuiltin> {
    vec![
        PromptBuiltin {
            name: "input",
            target: "__tyrus_cli_input",
            options: "InputOptions",
            ret: Type::Str,
        },
        PromptBuiltin {
            name: "password",
            target: "__tyrus_cli_password",
            options: "PasswordOptions",
            ret: Type::Str,
        },
        PromptBuiltin {
            name: "confirm",
            target: "__tyrus_cli_confirm",
            options: "ConfirmOptions",
            ret: Type::Bool,
        },
        PromptBuiltin {
            name: "select",
            target: "__tyrus_cli_select",
            options: "SelectOptions",
            ret: Type::Str,
        },
        PromptBuiltin {
            name: "multiselect",
            target: "__tyrus_cli_multiselect",
            options: "MultiSelectOptions",
            ret: string_array(),
        },
        PromptBuiltin {
            name: "number",
            target: "__tyrus_cli_number",
            options: "NumberOptions",
            ret: Type::F64,
        },
        PromptBuiltin {
            name: "integer",
            target: "__tyrus_cli_integer",
            options: "IntegerOptions",
            ret: Type::I64,
        },
    ]
}

pub fn prompt_type(builtin: &PromptBuiltin) -> Type {
    Type::Function {
        params: vec![Type::Named(builtin.options.to_string())],
        ret: Box::new(builtin.ret.clone()),
    }
}

pub const IS_TTY_TARGET: &str = "__tyrus_cli_is_tty";

pub fn is_tty_type() -> Type {
    Type::Function {
        params: Vec::new(),
        ret: Box::new(Type::Bool),
    }
}
