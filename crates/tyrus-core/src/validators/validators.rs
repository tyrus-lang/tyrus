use crate::ast::ExprKind;
use crate::ast::StmtKind;
use crate::collections::{HashMap, HashSet};
use crate::span::Span;

use crate::ast::{
    Expr, Field, FloatLiteral, Item, Param, Program, ResolvedEnum, Stmt, Type, ValidatorDecorator,
    expr_display, resolve_enum,
};

pub fn resolve_membership_decorator_constants(
    program: &mut Program,
    imported_enums: &HashMap<String, ResolvedEnum>,
) {
    let mut enums = imported_enums.clone();
    let mut constants = HashMap::default();
    for item in &program.items {
        match item {
            Item::EnumDecl {
                name,
                repr,
                members,
                ..
            } => {
                enums.insert(name.clone(), resolve_enum(repr, members).0);
            }
            Item::Stmt(Stmt {
                kind:
                    StmtKind::VarDecl {
                        mutable: false,
                        name,
                        value: Some(value),
                        ..
                    },
                ..
            }) => {
                constants.insert(name.clone(), value.clone());
            }
            _ => {}
        }
    }

    for item in &mut program.items {
        match item {
            Item::InterfaceDecl { fields, .. } => {
                resolve_membership_field_decorators(fields, &constants, &enums);
            }
            Item::FnDecl { params, .. } => {
                resolve_membership_param_decorators(params, &constants, &enums);
            }
            Item::Stmt(Stmt {
                kind:
                    StmtKind::VarDecl {
                        value:
                            Some(Expr {
                                kind: ExprKind::ArrowFn { params, .. },
                                ..
                            }),
                        ..
                    },
                ..
            }) => resolve_membership_param_decorators(params, &constants, &enums),
            _ => {}
        }
    }
}

fn resolve_membership_field_decorators(
    fields: &mut [Field],
    constants: &HashMap<String, Expr>,
    enums: &HashMap<String, ResolvedEnum>,
) {
    for field in fields {
        resolve_membership_decorators(&mut field.decorators, constants, enums);
    }
}

fn resolve_membership_param_decorators(
    params: &mut [Param],
    constants: &HashMap<String, Expr>,
    enums: &HashMap<String, ResolvedEnum>,
) {
    for param in params {
        resolve_membership_decorators(&mut param.decorators, constants, enums);
    }
}

fn resolve_membership_decorators(
    decorators: &mut [ValidatorDecorator],
    constants: &HashMap<String, Expr>,
    enums: &HashMap<String, ResolvedEnum>,
) {
    for decorator in decorators {
        if !matches!(decorator.name.as_str(), "in" | "notIn") {
            continue;
        }
        let Some(argument) = decorator.args.first() else {
            continue;
        };
        let mut resolving = HashSet::default();
        if let Some(resolved) =
            resolve_membership_constant(argument, constants, enums, &mut resolving)
        {
            decorator.args[0] = resolved;
        }
    }
}

fn resolve_membership_constant(
    expression: &Expr,
    constants: &HashMap<String, Expr>,
    enums: &HashMap<String, ResolvedEnum>,
    resolving: &mut HashSet<String>,
) -> Option<Expr> {
    match expression.kind() {
        ExprKind::ArrayLit(values) => values
            .iter()
            .map(|value| resolve_membership_constant(value, constants, enums, resolving))
            .collect::<Option<Vec<_>>>()
            .map(|values| Expr::synthetic(ExprKind::ArrayLit(values))),
        ExprKind::ConstAssert(value) | ExprKind::Comptime(value) => {
            resolve_membership_constant(value, constants, enums, resolving)
        }
        ExprKind::Ident(name) => {
            if !resolving.insert(name.clone()) {
                return None;
            }
            let resolved = constants
                .get(name)
                .and_then(|value| resolve_membership_constant(value, constants, enums, resolving));
            resolving.remove(name);
            resolved
        }
        ExprKind::FieldAccess { object, field } => {
            let Expr {
                kind: ExprKind::Ident(enum_name),
                ..
            } = object.as_ref()
            else {
                return None;
            };
            enums
                .get(enum_name)?
                .members
                .iter()
                .find_map(|(name, value)| (name == field).then(|| value.clone()))
        }
        _ => primitive_value(expression),
    }
}

#[derive(Debug, Clone)]
pub struct ValidatorDiagnostic {
    pub code: &'static str,
    pub severity: ValidatorDiagnosticSeverity,
    pub message: String,
    pub span: Span,
    pub subject: Option<String>,
    pub help: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidatorDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct ValidatorInfo {
    pub name: &'static str,
    pub signature: String,
    pub description: &'static str,
    pub default_message: &'static str,
    pub insert_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValidatorTarget {
    Numeric,
    Integer,
    String,
    Array,
    Any,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ArgKind {
    Number,
    Integer,
    String,
    Regex,
    Primitive,
    Array,
}

#[derive(Debug, Clone, Copy)]
struct ValidatorSpec {
    name: &'static str,
    target: ValidatorTarget,
    required_args: &'static [ArgKind],
    optional_args: &'static [ArgKind],
}

const NUMBER: &[ArgKind] = &[ArgKind::Number];
const NUMBER_NUMBER: &[ArgKind] = &[ArgKind::Number, ArgKind::Number];
const INTEGER: &[ArgKind] = &[ArgKind::Integer];
const INTEGER_STRING: &[ArgKind] = &[ArgKind::Integer, ArgKind::String];
const STRING: &[ArgKind] = &[ArgKind::String];
const REGEX: &[ArgKind] = &[ArgKind::Regex];
const PRIMITIVE: &[ArgKind] = &[ArgKind::Primitive];
const ARRAY: &[ArgKind] = &[ArgKind::Array];

const VALIDATORS: &[ValidatorSpec] = &[
    ValidatorSpec {
        name: "cep",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "cnpj",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "cpf",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "e164",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "email",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "endsWith",
        target: ValidatorTarget::String,
        required_args: STRING,
        optional_args: &[],
    },
    ValidatorSpec {
        name: "equal",
        target: ValidatorTarget::Any,
        required_args: PRIMITIVE,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "in",
        target: ValidatorTarget::Any,
        required_args: ARRAY,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "includes",
        target: ValidatorTarget::String,
        required_args: STRING,
        optional_args: &[],
    },
    ValidatorSpec {
        name: "integer",
        target: ValidatorTarget::Numeric,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "ip",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "ipv4",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "ipv6",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "isLowercase",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "isTrimmed",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "isUppercase",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "length",
        target: ValidatorTarget::String,
        required_args: INTEGER,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "lengthItems",
        target: ValidatorTarget::Array,
        required_args: INTEGER,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "max",
        target: ValidatorTarget::Numeric,
        required_args: NUMBER,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "maxItems",
        target: ValidatorTarget::Array,
        required_args: INTEGER,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "maxLength",
        target: ValidatorTarget::String,
        required_args: INTEGER,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "min",
        target: ValidatorTarget::Numeric,
        required_args: NUMBER,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "minItems",
        target: ValidatorTarget::Array,
        required_args: INTEGER,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "minLength",
        target: ValidatorTarget::String,
        required_args: INTEGER,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "multipleOf",
        target: ValidatorTarget::Numeric,
        required_args: NUMBER,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "negative",
        target: ValidatorTarget::Numeric,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "nonNegative",
        target: ValidatorTarget::Numeric,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "nonPositive",
        target: ValidatorTarget::Numeric,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "notBlank",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "notEmpty",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "notEmptyItems",
        target: ValidatorTarget::Array,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "notEqual",
        target: ValidatorTarget::Any,
        required_args: PRIMITIVE,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "notIn",
        target: ValidatorTarget::Any,
        required_args: ARRAY,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "notNull",
        target: ValidatorTarget::Optional,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "optional",
        target: ValidatorTarget::Optional,
        required_args: &[],
        optional_args: &[],
    },
    ValidatorSpec {
        name: "port",
        target: ValidatorTarget::Integer,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "positive",
        target: ValidatorTarget::Numeric,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "powerOfTwo",
        target: ValidatorTarget::Integer,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "range",
        target: ValidatorTarget::Numeric,
        required_args: NUMBER_NUMBER,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "regex",
        target: ValidatorTarget::String,
        required_args: REGEX,
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "required",
        target: ValidatorTarget::Any,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "safeInteger",
        target: ValidatorTarget::Numeric,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "startsWith",
        target: ValidatorTarget::String,
        required_args: STRING,
        optional_args: &[],
    },
    ValidatorSpec {
        name: "statusCode",
        target: ValidatorTarget::Integer,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "unique",
        target: ValidatorTarget::Array,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "url",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: STRING,
    },
    ValidatorSpec {
        name: "uuid",
        target: ValidatorTarget::String,
        required_args: &[],
        optional_args: INTEGER_STRING,
    },
];

pub fn validator_infos() -> Vec<ValidatorInfo> {
    VALIDATORS.iter().map(validator_info_for_spec).collect()
}

pub fn validator_info(name: &str) -> Option<ValidatorInfo> {
    find_spec(name).map(validator_info_for_spec)
}

fn validator_info_for_spec(spec: &'static ValidatorSpec) -> ValidatorInfo {
    ValidatorInfo {
        name: spec.name,
        signature: signature(spec),
        description: validator_description(spec.name),
        default_message: validator_default_message_text(spec.name),
        insert_text: validator_insert_text(spec),
    }
}

pub fn validator_message(decorator: &ValidatorDecorator) -> Option<String> {
    let spec = find_spec(&decorator.name)?;
    if validate_args(spec, &decorator.args).is_some() {
        return None;
    }
    custom_message(spec, &decorator.args)
        .map(str::to_string)
        .or_else(|| Some(validator_default_message(decorator)))
}

pub fn validator_display(decorator: &ValidatorDecorator) -> String {
    if decorator.args.is_empty() {
        return format!("@{}()", decorator.display_name);
    }
    let args = decorator
        .args
        .iter()
        .map(validator_arg_display)
        .collect::<Vec<_>>()
        .join(", ");
    format!("@{}({args})", decorator.display_name)
}

pub fn validate_literal_value(
    decorator: &ValidatorDecorator,
    value: &Expr,
    target: &Type,
) -> Option<String> {
    let spec = find_spec(&decorator.name)?;
    if validate_args(spec, &decorator.args).is_some() {
        return None;
    }
    if let Some(domain) = numeric_domain(target)
        && let Some(valid) =
            validate_numeric_literal(&decorator.name, &decorator.args, value, domain)
    {
        return (!valid).then(|| {
            validator_message(decorator).unwrap_or_else(|| "value is invalid".to_string())
        });
    }
    let valid = match decorator.name.as_str() {
        "equal" => primitive_arg_expr(&decorator.args, 0)
            .zip(primitive_value(value))
            .map(|(expected, actual)| expected == actual),
        "notEqual" => primitive_arg_expr(&decorator.args, 0)
            .zip(primitive_value(value))
            .map(|(expected, actual)| expected != actual),
        "in" => literal_array_arg(&decorator.args, 0)
            .zip(primitive_value(value))
            .map(|(values, actual)| values.iter().any(|expected| expected == &actual)),
        "notIn" => literal_array_arg(&decorator.args, 0)
            .zip(primitive_value(value))
            .map(|(values, actual)| values.iter().all(|expected| expected != &actual)),
        "minLength" => string_len(value)
            .zip(integer_arg_value(&decorator.args, 0))
            .map(|(actual, min)| actual >= min),
        "maxLength" => string_len(value)
            .zip(integer_arg_value(&decorator.args, 0))
            .map(|(actual, max)| actual <= max),
        "length" => string_len(value)
            .zip(integer_arg_value(&decorator.args, 0))
            .map(|(actual, exact)| actual == exact),
        "notEmpty" => string_value(value).map(tyrus_validators::is_not_empty),
        "notBlank" => string_value(value).map(tyrus_validators::is_not_blank),
        "isTrimmed" => string_value(value).map(tyrus_validators::is_trimmed),
        "isLowercase" => string_value(value).map(tyrus_validators::is_lowercase),
        "isUppercase" => string_value(value).map(tyrus_validators::is_uppercase),
        "startsWith" => string_value(value)
            .zip(string_arg_value(&decorator.args, 0))
            .map(|(actual, prefix)| actual.starts_with(prefix)),
        "endsWith" => string_value(value)
            .zip(string_arg_value(&decorator.args, 0))
            .map(|(actual, suffix)| actual.ends_with(suffix)),
        "includes" => string_value(value)
            .zip(string_arg_value(&decorator.args, 0))
            .map(|(actual, needle)| actual.contains(needle)),
        "email" => string_value(value).map(tyrus_validators::is_email),
        "url" => string_value(value).map(tyrus_validators::is_url),
        "uuid" => string_value(value).map(|actual| {
            let version = integer_arg_value(&decorator.args, 0);
            tyrus_validators::is_uuid(actual, version)
        }),
        "ip" => string_value(value).map(tyrus_validators::is_ip),
        "ipv4" => string_value(value).map(tyrus_validators::is_ipv4),
        "ipv6" => string_value(value).map(tyrus_validators::is_ipv6),
        "e164" => string_value(value).map(tyrus_validators::is_e164),
        "cpf" => string_value(value).map(tyrus_validators::is_cpf),
        "cnpj" => string_value(value).map(tyrus_validators::is_cnpj),
        "cep" => string_value(value).map(tyrus_validators::is_cep),
        "regex" => string_value(value).map(|actual| {
            let Some(Expr {
                kind: ExprKind::RegexLit { pattern, flags },
                ..
            }) = decorator.args.first()
            else {
                return true;
            };
            let Ok(pattern) = regex_runtime_pattern(pattern, flags) else {
                return true;
            };
            regex::Regex::new(&pattern).is_ok_and(|regex| regex.is_match(actual))
        }),
        "minItems" => array_len(value)
            .zip(integer_arg_value(&decorator.args, 0))
            .map(|(actual, min)| actual >= min),
        "maxItems" => array_len(value)
            .zip(integer_arg_value(&decorator.args, 0))
            .map(|(actual, max)| actual <= max),
        "lengthItems" => array_len(value)
            .zip(integer_arg_value(&decorator.args, 0))
            .map(|(actual, exact)| actual == exact),
        "notEmptyItems" => array_len(value).map(|actual| actual > 0),
        "unique" => literal_array_value(value).map(|values| {
            values
                .iter()
                .enumerate()
                .all(|(index, value)| !values[..index].iter().any(|previous| previous == value))
        }),
        "notNull" | "required" => matches!(value.kind(), ExprKind::Null).then_some(false),
        _ => return None,
    }?;

    (!valid).then(|| validator_message(decorator).unwrap_or_else(|| "value is invalid".to_string()))
}

pub fn validate_decorator(
    decorator: &ValidatorDecorator,
    target_type: &Type,
) -> Vec<ValidatorDiagnostic> {
    let Some(spec) = find_spec(&decorator.name) else {
        return vec![ValidatorDiagnostic {
            code: "E3160",
            severity: ValidatorDiagnosticSeverity::Error,
            message: format!("unknown validator `@{}`", decorator.name),
            subject: Some(format!("@{}", decorator.name)),
            help: Some(
                "use a built-in validator such as `@min`, `@email`, or `@notEqual`".to_string(),
            ),
            span: decorator.span,
        }];
    };

    let mut diagnostics = Vec::new();
    if !target_matches(spec.target, target_type) {
        diagnostics.push(ValidatorDiagnostic {
            code: "E3161",
            severity: ValidatorDiagnosticSeverity::Error,
            message: format!(
                "`@{}` cannot be used with `{}`",
                decorator.name, target_type
            ),
            subject: Some(format!("@{}", decorator.name)),
            help: Some(target_help(spec.target).to_string()),
            span: decorator.span,
        });
    }
    if let Some(message) = validate_args(spec, &decorator.args) {
        diagnostics.push(ValidatorDiagnostic {
            code: "E3162",
            severity: ValidatorDiagnosticSeverity::Error,
            message,
            subject: Some(format!("@{}", decorator.name)),
            help: Some(format!("expected signature `{}`", signature(spec))),
            span: decorator.span,
        });
    } else if let Some(message) = validate_regex_pattern(decorator) {
        diagnostics.push(ValidatorDiagnostic {
            code: "E3162",
            severity: ValidatorDiagnosticSeverity::Error,
            message,
            subject: Some(format!("@{}", decorator.name)),
            help: Some(
                "use a supported pattern; look-around assertions and backreferences are not supported"
                    .to_string(),
            ),
         span: decorator.span, });
    }
    if let Some(message) = validate_membership_argument(decorator, target_type) {
        diagnostics.push(ValidatorDiagnostic {
            code: "E3162",
            severity: ValidatorDiagnosticSeverity::Error,
            message,
            subject: Some(format!("@{}", decorator.name)),
            help: Some(format!(
                "use a compile-time constant array whose items are compatible with `{target_type}`"
            )),
            span: decorator.span,
        });
    }
    diagnostics
}

fn validate_membership_argument(
    decorator: &ValidatorDecorator,
    target_type: &Type,
) -> Option<String> {
    if !matches!(decorator.name.as_str(), "in" | "notIn") {
        return None;
    }
    let Expr {
        kind: ExprKind::ArrayLit(values),
        ..
    } = decorator.args.first()?
    else {
        return None;
    };
    let first = values.first().and_then(membership_literal_kind);
    for value in values {
        let Some(kind) = membership_literal_kind(value) else {
            return Some(format!(
                "`@{}` values must be primitive literals or enum members resolved at compile time",
                decorator.name
            ));
        };
        if first.is_some_and(|first| first != kind) {
            return Some(format!(
                "`@{}` values must form a homogeneous array",
                decorator.name
            ));
        }
        if !membership_literal_matches_type(value, target_type) {
            return Some(format!(
                "`@{}` value `{}` is not compatible with `{target_type}`",
                decorator.name,
                expr_display(value)
            ));
        }
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MembershipLiteralKind {
    String,
    Integer,
    Float,
    Boolean,
}

fn membership_literal_kind(value: &Expr) -> Option<MembershipLiteralKind> {
    match primitive_value(value)?.kind() {
        ExprKind::StrLit(_) => Some(MembershipLiteralKind::String),
        ExprKind::IntLit(_) | ExprKind::UIntLit(_) => Some(MembershipLiteralKind::Integer),
        ExprKind::FloatLit(_) => Some(MembershipLiteralKind::Float),
        ExprKind::BoolLit(_) => Some(MembershipLiteralKind::Boolean),
        _ => None,
    }
}

fn membership_literal_matches_type(value: &Expr, target_type: &Type) -> bool {
    let target_type = unwrap_optional(target_type);
    match target_type {
        Type::Named(_) => false,
        Type::Str => matches!(
            membership_literal_kind(value),
            Some(MembershipLiteralKind::String)
        ),
        Type::Bool => matches!(
            membership_literal_kind(value),
            Some(MembershipLiteralKind::Boolean)
        ),
        ty if is_membership_integer_type(ty) => matches!(
            membership_literal_kind(value),
            Some(MembershipLiteralKind::Integer)
        ),
        ty if is_numeric(ty) => matches!(
            membership_literal_kind(value),
            Some(MembershipLiteralKind::Integer | MembershipLiteralKind::Float)
        ),
        _ => false,
    }
}

fn is_membership_integer_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::Isize
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::Usize
    )
}

pub fn validate_decorator_set(decorators: &[ValidatorDecorator]) -> Vec<ValidatorDiagnostic> {
    let mut diagnostics = Vec::new();
    for (index, decorator) in decorators.iter().enumerate() {
        let Some(spec) = find_spec(&decorator.name) else {
            continue;
        };
        if validate_args(spec, &decorator.args).is_some() {
            continue;
        }
        if is_bound_validator(&decorator.name) {
            continue;
        }
        if decorators[..index].iter().any(|previous| {
            same_validator_key(previous, decorator)
                && find_spec(&previous.name).is_some_and(|previous_spec| {
                    validate_args(previous_spec, &previous.args).is_none()
                })
        }) {
            diagnostics.push(ValidatorDiagnostic {
                code: "E3164",
                severity: ValidatorDiagnosticSeverity::Error,
                message: format!(
                    "`@{}` is redundant because the same validator is already applied",
                    decorator.name
                ),
                subject: Some(format!(
                    "@{}#{}",
                    decorator.name,
                    decorators[..=index]
                        .iter()
                        .filter(|other| other.name == decorator.name)
                        .count()
                )),
                help: Some("remove the repeated validator from this target".to_string()),
                span: decorator.span,
            });
        }
    }

    diagnostics.extend(validate_repeated_bounds(decorators));
    diagnostics.extend(validate_numeric_bounds(decorators));
    diagnostics.extend(validate_length_bounds(
        decorators,
        "minLength",
        "maxLength",
        "length",
        "string length",
    ));
    diagnostics.extend(validate_length_bounds(
        decorators,
        "minItems",
        "maxItems",
        "lengthItems",
        "array length",
    ));
    diagnostics.extend(validate_equal_not_equal(decorators));
    diagnostics
}

pub fn normalize_decorator_set(decorators: &[ValidatorDecorator]) -> Vec<ValidatorDecorator> {
    let mut normalized = Vec::new();
    for (index, decorator) in decorators.iter().enumerate() {
        if decorators[..index]
            .iter()
            .any(|previous| previous.name == decorator.name && previous.args == decorator.args)
        {
            continue;
        }
        if redundant_bound_at(decorators, index) {
            continue;
        }
        normalized.push(decorator.clone());
    }
    normalized
}

fn redundant_bound_at(decorators: &[ValidatorDecorator], index: usize) -> bool {
    let Some(decorator) = decorators.get(index) else {
        return false;
    };
    if !is_bound_validator(&decorator.name) {
        return false;
    }
    let Some(value) = numeric_arg_value(&decorator.args, 0) else {
        return false;
    };
    let is_upper = matches!(decorator.name.as_str(), "max" | "maxLength" | "maxItems");
    decorators
        .iter()
        .enumerate()
        .filter(|(other_index, other)| *other_index != index && other.name == decorator.name)
        .filter_map(|(_, other)| numeric_arg_value(&other.args, 0))
        .any(|other_value| {
            if is_upper {
                other_value < value
            } else {
                other_value > value
            }
        })
}

fn find_spec(name: &str) -> Option<&'static ValidatorSpec> {
    VALIDATORS.iter().find(|spec| spec.name == name)
}

fn validator_description(name: &str) -> &'static str {
    match name {
        "cep" => "Validates a Brazilian postal code string.",
        "cnpj" => {
            "Validates a Brazilian CNPJ string, including alphanumeric CNPJ values with numeric check digits."
        }
        "cpf" => "Validates a Brazilian CPF string with check digits.",
        "e164" => "Validates an international E.164 phone number string.",
        "email" => "Validates a practical ASCII e-mail address string.",
        "endsWith" => "Requires a string to end with the given suffix.",
        "equal" => "Requires a value to be equal to the given primitive literal.",
        "in" => "Requires a value to be included in a literal array.",
        "includes" => "Requires a string to include the given substring.",
        "integer" => "Requires a numeric value to be an integer.",
        "ip" => "Validates an IPv4 or IPv6 address string.",
        "ipv4" => "Validates an IPv4 address string.",
        "ipv6" => "Validates an IPv6 address string.",
        "isLowercase" => "Requires a string to already be lowercase.",
        "isTrimmed" => "Requires a string to already be trimmed.",
        "isUppercase" => "Requires a string to already be uppercase.",
        "length" => "Requires a string to have exactly the given length.",
        "lengthItems" => "Requires an array to have exactly the given item count.",
        "max" => "Requires a numeric value to be less than or equal to the limit.",
        "maxItems" => "Requires an array to contain at most the given item count.",
        "maxLength" => "Requires a string to contain at most the given character count.",
        "min" => "Requires a numeric value to be greater than or equal to the limit.",
        "minItems" => "Requires an array to contain at least the given item count.",
        "minLength" => "Requires a string to contain at least the given character count.",
        "multipleOf" => "Requires a numeric value to be a multiple of the given number.",
        "negative" => "Requires a numeric value to be less than zero.",
        "nonNegative" => "Requires a numeric value to be greater than or equal to zero.",
        "nonPositive" => "Requires a numeric value to be less than or equal to zero.",
        "notBlank" => "Requires a string to contain non-whitespace text.",
        "notEmpty" => "Requires a string to contain at least one character.",
        "notEmptyItems" => "Requires an array to contain at least one item.",
        "notEqual" => "Requires a value to be different from the given primitive literal.",
        "notIn" => "Requires a value not to be included in a literal array.",
        "notNull" => "Requires an optional value to be non-null.",
        "optional" => {
            "Marks a nullable `T?` value as explicitly optional; absence is allowed at external boundaries (counterpart of `@required`)."
        }
        "port" => {
            "Requires a numeric value to be a registered port between 1024 and 49151 (above the privileged range, below the ephemeral range)."
        }
        "positive" => "Requires a numeric value to be greater than zero.",
        "powerOfTwo" => "Requires a numeric value to be a power of two.",
        "range" => "Requires a numeric value to be inside the inclusive range.",
        "regex" => "Requires a string to match the given regular expression.",
        "required" => "Requires a value to be present at an external boundary.",
        "safeInteger" => "Requires a numeric value to fit in the safe integer range.",
        "startsWith" => "Requires a string to start with the given prefix.",
        "statusCode" => {
            "Requires a numeric value to be a valid HTTP status code between 100 and 599."
        }
        "unique" => "Requires an array to contain unique items.",
        "url" => "Validates a URL string.",
        "uuid" => "Validates a UUID string.",
        _ => "Validates the decorated target.",
    }
}

fn validator_default_message_text(name: &str) -> &'static str {
    match name {
        "cep" => "value must be a valid Brazilian postal code",
        "cnpj" => "value must be a valid Brazilian CNPJ",
        "cpf" => "value must be a valid Brazilian CPF",
        "e164" => "value must be a valid E.164 phone number",
        "email" => "value must be a valid e-mail address",
        "endsWith" => "value must end with the expected suffix",
        "equal" => "value must be equal to the expected value",
        "in" => "value must be one of the allowed values",
        "includes" => "value must include the expected substring",
        "integer" => "value must be an integer",
        "ip" => "value must be a valid IP address",
        "ipv4" => "value must be a valid IPv4 address",
        "ipv6" => "value must be a valid IPv6 address",
        "isLowercase" => "value must already be lowercase",
        "isTrimmed" => "value must already be trimmed",
        "isUppercase" => "value must already be uppercase",
        "length" => "value must have exactly the expected length",
        "lengthItems" => "array must contain exactly the expected number of items",
        "max" => "value must be less than or equal to the maximum",
        "maxItems" => "array must contain at most the maximum number of items",
        "maxLength" => "value must contain at most the maximum number of characters",
        "min" => "value must be greater than or equal to the minimum",
        "minItems" => "array must contain at least the minimum number of items",
        "minLength" => "value must contain at least the minimum number of characters",
        "multipleOf" => "value must be a multiple of the expected number",
        "negative" => "value must be negative",
        "nonNegative" => "value must be non-negative",
        "nonPositive" => "value must be non-positive",
        "notBlank" => "value must not be blank",
        "notEmpty" => "value must not be empty",
        "notEmptyItems" => "array must not be empty",
        "notEqual" => "value must be different from the disallowed value",
        "notIn" => "value must not be one of the disallowed values",
        "notNull" => "value must not be null",
        "optional" => "value is optional",
        "port" => "value must be a registered port between 1024 and 49151",
        "positive" => "value must be positive",
        "powerOfTwo" => "value must be a power of two",
        "range" => "value must be inside the allowed range",
        "regex" => "value must match the expected pattern",
        "required" => "value is required",
        "safeInteger" => "value must be a safe integer",
        "startsWith" => "value must start with the expected prefix",
        "statusCode" => "value must be a valid HTTP status code between 100 and 599",
        "unique" => "array items must be unique",
        "url" => "value must be a valid URL",
        "uuid" => "value must be a valid UUID",
        _ => "value is invalid",
    }
}

fn custom_message<'a>(spec: &ValidatorSpec, args: &'a [Expr]) -> Option<&'a str> {
    if !spec
        .optional_args
        .last()
        .is_some_and(|kind| *kind == ArgKind::String)
        || args.len() <= spec.required_args.len()
    {
        return None;
    }
    match args.last()?.kind() {
        ExprKind::StrLit(message) => Some(message.as_str()),
        _ => None,
    }
}

fn validator_default_message(decorator: &ValidatorDecorator) -> String {
    let arg = |index| {
        decorator
            .args
            .get(index)
            .map(validator_arg_display)
            .unwrap_or_else(|| "the expected value".to_string())
    };
    match decorator.name.as_str() {
        "endsWith" => format!("value must end with {}", arg(0)),
        "equal" => format!("value must be equal to {}", arg(0)),
        "in" => format!("value must be one of {}", arg(0)),
        "includes" => format!("value must include {}", arg(0)),
        "length" => format!("value must have exactly {} characters", arg(0)),
        "lengthItems" => format!("array must contain exactly {} items", arg(0)),
        "max" => format!("value must be less than or equal to {}", arg(0)),
        "maxItems" => format!("array must contain at most {} items", arg(0)),
        "maxLength" => format!("value must contain at most {} characters", arg(0)),
        "min" => format!("value must be greater than or equal to {}", arg(0)),
        "minItems" => format!("array must contain at least {} items", arg(0)),
        "minLength" => format!("value must contain at least {} characters", arg(0)),
        "multipleOf" => format!("value must be a multiple of {}", arg(0)),
        "notEqual" => format!("value must be different from {}", arg(0)),
        "notIn" => format!("value must not be one of {}", arg(0)),
        "range" => format!("value must be between {} and {}", arg(0), arg(1)),
        "regex" => format!("value must match {}", arg(0)),
        "startsWith" => format!("value must start with {}", arg(0)),
        "uuid" if !decorator.args.is_empty() => {
            format!("value must be a valid UUID version {}", arg(0))
        }
        _ => validator_default_message_text(&decorator.name).to_string(),
    }
}

fn validator_arg_display(expr: &Expr) -> String {
    match expr.kind() {
        ExprKind::StrLit(value) => format!("\"{}\"", display_string_literal(value)),
        ExprKind::RegexLit { pattern, flags } => {
            format!("/{}/{}", pattern.replace('/', "\\/"), flags)
        }
        ExprKind::IntLit(value) => value.to_string(),
        ExprKind::FloatLit(value) => value.text().to_string(),
        ExprKind::BoolLit(value) => value.to_string(),
        ExprKind::Null => "null".to_string(),
        ExprKind::ArrayLit(values) => {
            let values = values
                .iter()
                .map(validator_arg_display)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{values}]")
        }
        ExprKind::Neg(inner) => format!("-{}", validator_arg_display(inner)),
        _ => expr_display(expr),
    }
}

fn display_string_literal(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            '\0' => "\\0".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}

fn validator_insert_text(spec: &ValidatorSpec) -> String {
    let placeholders = validator_placeholders(spec.name);
    if placeholders.is_empty() {
        return format!("{}($0)", spec.name);
    }
    let args = placeholders
        .iter()
        .enumerate()
        .map(|(index, placeholder)| format!("${{{}:{placeholder}}}", index + 1))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}({args})", spec.name)
}

fn validator_placeholders(name: &str) -> &'static [&'static str] {
    match name {
        "endsWith" => &["suffix"],
        "equal" | "notEqual" => &["value"],
        "in" | "notIn" => &["values"],
        "includes" => &["value"],
        "length" | "lengthItems" => &["exact"],
        "max" | "maxItems" | "maxLength" => &["value"],
        "min" | "minItems" | "minLength" => &["value"],
        "multipleOf" => &["n"],
        "range" => &["min", "max"],
        "regex" => &["pattern"],
        "startsWith" => &["prefix"],
        _ => &[],
    }
}

fn target_matches(target: ValidatorTarget, ty: &Type) -> bool {
    match target {
        ValidatorTarget::Any => true,
        ValidatorTarget::Optional => matches!(ty, Type::Optional(_)),
        ValidatorTarget::Numeric => is_numeric(unwrap_optional(ty)),
        ValidatorTarget::Integer => is_integer(unwrap_optional(ty)),
        ValidatorTarget::String => matches!(unwrap_optional(ty), Type::Str),
        ValidatorTarget::Array => matches!(unwrap_optional(ty), Type::Array(_)),
    }
}

fn same_validator_key(left: &ValidatorDecorator, right: &ValidatorDecorator) -> bool {
    if left.name != right.name {
        return false;
    }
    let Some(spec) = find_spec(&left.name) else {
        return false;
    };
    primary_args(spec, &left.args) == primary_args(spec, &right.args)
}

fn primary_args<'a>(spec: &ValidatorSpec, args: &'a [Expr]) -> &'a [Expr] {
    let mut end = args.len();
    if spec
        .optional_args
        .last()
        .is_some_and(|kind| *kind == ArgKind::String)
        && end > spec.required_args.len()
        && matches!(
            args.last(),
            Some(Expr {
                kind: ExprKind::StrLit(_),
                ..
            })
        )
    {
        end -= 1;
    }
    &args[..end]
}

fn is_bound_validator(name: &str) -> bool {
    matches!(
        name,
        "min" | "max" | "minLength" | "maxLength" | "minItems" | "maxItems"
    )
}

fn validate_repeated_bounds(decorators: &[ValidatorDecorator]) -> Vec<ValidatorDiagnostic> {
    const LOWER: &[&str] = &["min", "minLength", "minItems"];
    const UPPER: &[&str] = &["max", "maxLength", "maxItems"];
    let mut diagnostics = Vec::new();
    for name in LOWER.iter().chain(UPPER.iter()) {
        let is_upper = UPPER.contains(name);
        let mut occurrence = 0;
        let mut entries: Vec<(usize, f64, Span)> = Vec::new();
        for decorator in decorators.iter().filter(|d| d.name == *name) {
            occurrence += 1;
            let valid =
                find_spec(name).is_some_and(|spec| validate_args(spec, &decorator.args).is_none());
            if valid && let Some(value) = numeric_arg_value(&decorator.args, 0) {
                entries.push((occurrence, value, decorator.span));
            }
        }
        if entries.len() < 2 {
            continue;
        }
        let binding = entries.iter().copied().reduce(|best, current| {
            let stricter = if is_upper {
                current.1 < best.1
            } else {
                current.1 > best.1
            };
            if stricter { current } else { best }
        });
        let Some((binding_occurrence, _, _)) = binding else {
            continue;
        };
        for (occurrence, _, span) in entries.iter().filter(|(o, _, _)| *o != binding_occurrence) {
            diagnostics.push(ValidatorDiagnostic {
                code: "E3164",
                severity: ValidatorDiagnosticSeverity::Error,
                message: format!(
                    "`@{name}` is redundant because another `@{name}` already applies a stricter bound"
                ),
                subject: Some(format!("@{name}#{occurrence}")),
                help: Some("keep only the strictest bound".to_string()),
                span: *span,
            });
        }
    }
    diagnostics
}

fn validate_numeric_bounds(decorators: &[ValidatorDecorator]) -> Vec<ValidatorDiagnostic> {
    let mut diagnostics = Vec::new();
    let min = numeric_arg(decorators, "min", 0)
        .into_iter()
        .chain(numeric_arg(decorators, "range", 0))
        .max_by(|left, right| left.0.total_cmp(&right.0));
    let max = numeric_arg(decorators, "max", 0)
        .into_iter()
        .chain(numeric_arg(decorators, "range", 1))
        .min_by(|left, right| left.0.total_cmp(&right.0));

    if let (Some((min_value, min_name)), Some((max_value, max_name))) = (min, max)
        && min_value > max_value
    {
        diagnostics.push(contradiction(
            decorators,
                min_name,
                format!(
                    "`@{min_name}` conflicts with `@{max_name}` because the minimum {min_value} is greater than the maximum {max_value}"
                ),
            ));
    }

    if has_validator(decorators, "positive") && has_validator(decorators, "nonPositive") {
        diagnostics.push(contradiction(
            decorators,
            "nonPositive",
            "`@nonPositive` conflicts with `@positive`".to_string(),
        ));
    }
    if has_validator(decorators, "negative") && has_validator(decorators, "nonNegative") {
        diagnostics.push(contradiction(
            decorators,
            "nonNegative",
            "`@nonNegative` conflicts with `@negative`".to_string(),
        ));
    }
    if let Some((max_value, _)) = max
        && has_validator(decorators, "positive")
        && max_value <= 0.0
    {
        diagnostics.push(contradiction(
            decorators,
            "max",
            format!("`@max({max_value})` conflicts with `@positive`"),
        ));
    }
    if let Some((min_value, _)) = min
        && has_validator(decorators, "negative")
        && min_value >= 0.0
    {
        diagnostics.push(contradiction(
            decorators,
            "min",
            format!("`@min({min_value})` conflicts with `@negative`"),
        ));
    }

    diagnostics
}

fn validate_length_bounds(
    decorators: &[ValidatorDecorator],
    min_name: &'static str,
    max_name: &'static str,
    exact_name: &'static str,
    label: &'static str,
) -> Vec<ValidatorDiagnostic> {
    let mut diagnostics = Vec::new();
    let min = integer_arg(decorators, min_name, 0)
        .into_iter()
        .max_by_key(|(value, _)| *value);
    let max = integer_arg(decorators, max_name, 0)
        .into_iter()
        .min_by_key(|(value, _)| *value);
    let exact_values = integer_arg(decorators, exact_name, 0);

    if let (Some((min_value, _)), Some((max_value, _))) = (min, max)
        && min_value > max_value
    {
        diagnostics.push(contradiction(
            decorators,
                min_name,
                format!(
                    "`@{min_name}` conflicts with `@{max_name}` because the minimum {label} {min_value} is greater than the maximum {label} {max_value}"
                ),
            ));
    }

    for (exact, _) in exact_values {
        if let Some((min_value, _)) = min
            && exact < min_value
        {
            diagnostics.push(contradiction(
                decorators,
                exact_name,
                format!("`@{exact_name}({exact})` conflicts with `@{min_name}({min_value})`"),
            ));
        }
        if let Some((max_value, _)) = max
            && exact > max_value
        {
            diagnostics.push(contradiction(
                decorators,
                exact_name,
                format!("`@{exact_name}({exact})` conflicts with `@{max_name}({max_value})`"),
            ));
        }
    }

    diagnostics
}

fn validate_equal_not_equal(decorators: &[ValidatorDecorator]) -> Vec<ValidatorDiagnostic> {
    let mut diagnostics = Vec::new();
    let equal_values = primitive_arg(decorators, "equal");
    let not_equal_values = primitive_arg(decorators, "notEqual");

    for (index, (value, _)) in equal_values.iter().enumerate() {
        if equal_values[..index]
            .iter()
            .any(|(previous, _)| previous != value)
        {
            diagnostics.push(contradiction(
                decorators,
                "equal",
                "`@equal` cannot require two different values".to_string(),
            ));
            break;
        }
    }
    for (value, _) in &equal_values {
        if not_equal_values
            .iter()
            .any(|(not_equal, _)| not_equal == value)
        {
            diagnostics.push(contradiction(
                decorators,
                "notEqual",
                format!(
                    "`@notEqual({})` conflicts with `@equal({})`",
                    expr_display(value),
                    expr_display(value)
                ),
            ));
        }
    }

    diagnostics
}

fn contradiction(
    decorators: &[ValidatorDecorator],
    subject: &'static str,
    message: String,
) -> ValidatorDiagnostic {
    let span = decorators
        .iter()
        .find(|decorator| decorator.name == subject)
        .map_or(Span::DUMMY, |decorator| decorator.span);
    ValidatorDiagnostic {
        code: "E3163",
        severity: ValidatorDiagnosticSeverity::Error,
        message,
        subject: Some(format!("@{subject}")),
        help: Some("remove or adjust one of the conflicting validators".to_string()),
        span,
    }
}

fn has_validator(decorators: &[ValidatorDecorator], name: &str) -> bool {
    decorators.iter().any(|decorator| {
        decorator.name == name
            && find_spec(name).is_some_and(|spec| validate_args(spec, &decorator.args).is_none())
    })
}

fn numeric_arg(
    decorators: &[ValidatorDecorator],
    name: &'static str,
    index: usize,
) -> Vec<(f64, &'static str)> {
    decorators
        .iter()
        .filter(|decorator| decorator.name == name)
        .filter(|decorator| {
            find_spec(name).is_some_and(|spec| validate_args(spec, &decorator.args).is_none())
        })
        .filter_map(|decorator| {
            numeric_literal(decorator.args.get(index)?).map(|value| (value, name))
        })
        .collect()
}

fn integer_arg(
    decorators: &[ValidatorDecorator],
    name: &'static str,
    index: usize,
) -> Vec<(i64, &'static str)> {
    decorators
        .iter()
        .filter(|decorator| decorator.name == name)
        .filter(|decorator| {
            find_spec(name).is_some_and(|spec| validate_args(spec, &decorator.args).is_none())
        })
        .filter_map(|decorator| {
            integer_literal(decorator.args.get(index)?).map(|value| (value, name))
        })
        .collect()
}

fn primitive_arg<'a>(
    decorators: &'a [ValidatorDecorator],
    name: &'static str,
) -> Vec<(&'a Expr, &'static str)> {
    decorators
        .iter()
        .filter(|decorator| decorator.name == name)
        .filter(|decorator| {
            find_spec(name).is_some_and(|spec| validate_args(spec, &decorator.args).is_none())
        })
        .filter_map(|decorator| decorator.args.first().map(|value| (value, name)))
        .collect()
}

fn numeric_literal(expr: &Expr) -> Option<f64> {
    match expr.kind() {
        ExprKind::IntLit(value) => Some(*value as f64),
        ExprKind::UIntLit(value) => Some(*value as f64),
        ExprKind::FloatLit(value) => Some(value.value()),
        _ => None,
    }
}

fn integer_literal(expr: &Expr) -> Option<i64> {
    match expr.kind() {
        ExprKind::IntLit(value) => i64::try_from(*value).ok(),
        ExprKind::UIntLit(value) => i64::try_from(*value).ok(),
        _ => None,
    }
}

pub(crate) fn primitive_arg_expr(args: &[Expr], index: usize) -> Option<Expr> {
    primitive_value(args.get(index)?)
}

pub(crate) fn numeric_arg_value(args: &[Expr], index: usize) -> Option<f64> {
    numeric_value(args.get(index)?)
}

pub(crate) fn integer_arg_value(args: &[Expr], index: usize) -> Option<i64> {
    integer_value(args.get(index)?)
}

pub(crate) fn string_arg_value(args: &[Expr], index: usize) -> Option<&str> {
    string_value(args.get(index)?)
}

pub(crate) fn literal_array_arg(args: &[Expr], index: usize) -> Option<Vec<Expr>> {
    literal_array_value(args.get(index)?)
}

fn numeric_value(expr: &Expr) -> Option<f64> {
    match expr.kind() {
        ExprKind::IntLit(value) => Some(*value as f64),
        ExprKind::UIntLit(value) => Some(*value as f64),
        ExprKind::FloatLit(value) => Some(value.value()),
        ExprKind::Neg(inner) => numeric_value(inner).map(|value| -value),
        _ => None,
    }
}

fn integer_value(expr: &Expr) -> Option<i64> {
    match expr.kind() {
        ExprKind::IntLit(value) => i64::try_from(*value).ok(),
        ExprKind::UIntLit(value) => i64::try_from(*value).ok(),
        ExprKind::Neg(inner) => integer_value(inner).map(|value| -value),
        _ => None,
    }
}

fn string_value(expr: &Expr) -> Option<&str> {
    match expr.kind() {
        ExprKind::StrLit(value) => Some(value.as_str()),
        _ => None,
    }
}

fn string_len(expr: &Expr) -> Option<i64> {
    string_value(expr).map(|value| tyrus_validators::char_count(value) as i64)
}

fn array_len(expr: &Expr) -> Option<i64> {
    let Expr {
        kind: ExprKind::ArrayLit(values),
        ..
    } = expr
    else {
        return None;
    };
    values
        .iter()
        .all(|value| !matches!(value.kind(), ExprKind::Spread(_)))
        .then_some(values.len() as i64)
}

fn primitive_value(expr: &Expr) -> Option<Expr> {
    match expr.kind() {
        ExprKind::StrLit(_)
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null => Some(expr.clone()),
        ExprKind::Neg(inner) => match inner.as_ref() {
            Expr {
                kind: ExprKind::IntLit(value),
                ..
            } => Some(Expr::synthetic(ExprKind::IntLit(-value))),
            Expr {
                kind: ExprKind::FloatLit(value),
                ..
            } => Some(Expr::synthetic(ExprKind::FloatLit(
                FloatLiteral::from_value(-value.value()),
            ))),
            _ => None,
        },
        _ => None,
    }
}

fn literal_array_value(expr: &Expr) -> Option<Vec<Expr>> {
    let Expr {
        kind: ExprKind::ArrayLit(values),
        ..
    } = expr
    else {
        return None;
    };
    values.iter().map(primitive_value).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    #[test]
    fn validator_message_uses_default_when_custom_message_is_absent() {
        let decorator = ValidatorDecorator {
            name: "email".to_string(),
            display_name: "email".to_string(),
            args: Vec::new(),
            argument_ranges: Vec::new(),
            span: Span::DUMMY,
        };
        assert_eq!(
            validator_message(&decorator).as_deref(),
            Some("value must be a valid e-mail address")
        );
    }

    #[test]
    fn validator_message_includes_known_validator_arguments() {
        let decorator = ValidatorDecorator {
            name: "min".to_string(),
            display_name: "min".to_string(),
            args: vec![Expr::synthetic(ExprKind::IntLit(5))],
            argument_ranges: Vec::new(),
            span: Span::DUMMY,
        };
        assert_eq!(
            validator_message(&decorator).as_deref(),
            Some("value must be greater than or equal to 5")
        );

        let decorator = ValidatorDecorator {
            name: "range".to_string(),
            display_name: "range".to_string(),
            args: vec![
                Expr::synthetic(ExprKind::IntLit(1)),
                Expr::synthetic(ExprKind::IntLit(10)),
            ],
            argument_ranges: Vec::new(),
            span: Span::DUMMY,
        };
        assert_eq!(
            validator_message(&decorator).as_deref(),
            Some("value must be between 1 and 10")
        );
    }

    #[test]
    fn validator_message_prefers_custom_message() {
        let decorator = ValidatorDecorator {
            name: "minLength".to_string(),
            display_name: "minLength".to_string(),
            args: vec![
                Expr::synthetic(ExprKind::IntLit(3)),
                Expr::synthetic(ExprKind::StrLit(
                    "name must have at least 3 characters".to_string(),
                )),
            ],
            argument_ranges: Vec::new(),
            span: Span::DUMMY,
        };
        assert_eq!(
            validator_message(&decorator).as_deref(),
            Some("name must have at least 3 characters")
        );
    }

    fn validate_literal_value_i32(decorator: &ValidatorDecorator, value: i128) -> Option<String> {
        validate_literal_value(
            decorator,
            &Expr::synthetic(ExprKind::IntLit(value)),
            &Type::I32,
        )
    }

    #[test]
    fn status_code_accepts_valid_http_codes_and_rejects_others() {
        let decorator = ValidatorDecorator {
            name: "statusCode".to_string(),
            display_name: "statusCode".to_string(),
            args: Vec::new(),
            argument_ranges: Vec::new(),
            span: Span::DUMMY,
        };
        assert_eq!(validate_literal_value_i32(&decorator, 200), None);
        assert_eq!(validate_literal_value_i32(&decorator, 404), None);
        assert_eq!(validate_literal_value_i32(&decorator, 599), None);
        assert_eq!(validate_literal_value_i32(&decorator, 100), None);
        let expected = "value must be a valid HTTP status code between 100 and 599".to_string();
        assert_eq!(
            validate_literal_value_i32(&decorator, 99),
            Some(expected.clone())
        );
        assert_eq!(validate_literal_value_i32(&decorator, 600), Some(expected));
    }

    #[test]
    fn validator_message_ignores_invalid_validator_arguments() {
        let decorator = ValidatorDecorator {
            name: "minLength".to_string(),
            display_name: "minLength".to_string(),
            args: vec![Expr::synthetic(ExprKind::StrLit("3".to_string()))],
            argument_ranges: Vec::new(),
            span: Span::DUMMY,
        };
        assert_eq!(validator_message(&decorator), None);
    }
}

fn unwrap_optional(ty: &Type) -> &Type {
    match ty {
        Type::Optional(inner) => inner,
        other => other,
    }
}

fn is_numeric(ty: &Type) -> bool {
    matches!(
        ty,
        Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::Isize
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::Usize
            | Type::F32
            | Type::F64
            | Type::F128
    )
}

fn is_integer(ty: &Type) -> bool {
    matches!(
        ty,
        Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::Isize
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::Usize
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumericDomain {
    Int,
    Float,
}

fn numeric_domain(target: &Type) -> Option<NumericDomain> {
    match unwrap_optional(target) {
        ty if is_integer(ty) => Some(NumericDomain::Int),
        Type::F32 | Type::F64 | Type::F128 => Some(NumericDomain::Float),
        _ => None,
    }
}

fn int_literal(expr: &Expr) -> Option<i128> {
    match expr.kind() {
        ExprKind::IntLit(value) => Some(*value),
        ExprKind::UIntLit(value) => i128::try_from(*value).ok(),
        ExprKind::Neg(inner) => int_literal(inner).map(|value| -value),
        _ => None,
    }
}

fn float_literal(expr: &Expr) -> Option<f64> {
    match expr.kind() {
        ExprKind::IntLit(value) => Some(*value as f64),
        ExprKind::UIntLit(value) => Some(*value as f64),
        ExprKind::FloatLit(value) => Some(value.value()),
        ExprKind::Neg(inner) => float_literal(inner).map(|value| -value),
        _ => None,
    }
}

fn validate_numeric_literal(
    name: &str,
    args: &[Expr],
    value: &Expr,
    domain: NumericDomain,
) -> Option<bool> {
    match domain {
        NumericDomain::Int => {
            let actual = int_literal(value)?;
            let arg = |index: usize| args.get(index).and_then(int_literal);
            match name {
                "min" => Some(actual >= arg(0)?),
                "max" => Some(actual <= arg(0)?),
                "range" => Some(actual >= arg(0)? && actual <= arg(1)?),
                "positive" => Some(actual > 0),
                "negative" => Some(actual < 0),
                "nonNegative" => Some(actual >= 0),
                "nonPositive" => Some(actual <= 0),
                "multipleOf" => {
                    let divisor = arg(0)?;
                    Some(divisor != 0 && actual % divisor == 0)
                }
                "integer" | "safeInteger" => Some(true),
                "powerOfTwo" => Some(actual > 0 && (actual & (actual - 1)) == 0),
                "port" => Some((1024..=49_151).contains(&actual)),
                "statusCode" => Some((100..=599).contains(&actual)),
                _ => None,
            }
        }
        NumericDomain::Float => {
            let actual = float_literal(value)?;
            let arg = |index: usize| args.get(index).and_then(float_literal);
            match name {
                "min" => Some(actual >= arg(0)?),
                "max" => Some(actual <= arg(0)?),
                "range" => Some(actual >= arg(0)? && actual <= arg(1)?),
                "positive" => Some(actual > 0.0),
                "negative" => Some(actual < 0.0),
                "nonNegative" => Some(actual >= 0.0),
                "nonPositive" => Some(actual <= 0.0),
                "multipleOf" => {
                    let divisor = arg(0)?;
                    Some(divisor != 0.0 && actual % divisor == 0.0)
                }
                "integer" => Some(actual.is_finite() && actual.fract() == 0.0),
                "safeInteger" => Some(safe_integer_float(actual)),
                _ => None,
            }
        }
    }
}

fn safe_integer_float(value: f64) -> bool {
    const F64_EXACT_INTEGER_LIMIT: f64 = 9_007_199_254_740_992.0;
    value.is_finite() && value.fract() == 0.0 && value.abs() <= F64_EXACT_INTEGER_LIMIT
}

fn validate_args(spec: &ValidatorSpec, args: &[Expr]) -> Option<String> {
    let min = spec.required_args.len();
    let max = min + spec.optional_args.len();
    if args.len() < min || args.len() > max {
        return Some(format!(
            "`@{}` expects {} argument{}, got {}",
            spec.name,
            if min == max {
                min.to_string()
            } else {
                format!("{min} to {max}")
            },
            if max == 1 { "" } else { "s" },
            args.len()
        ));
    }
    for (index, arg) in args.iter().enumerate() {
        let expected = if index < spec.required_args.len() {
            spec.required_args[index]
        } else {
            spec.optional_args[index - spec.required_args.len()]
        };
        if !arg_matches(expected, arg) {
            return Some(format!(
                "`@{}` argument {} must be {}",
                spec.name,
                index + 1,
                arg_kind_label(expected)
            ));
        }
    }
    None
}

fn validate_regex_pattern(decorator: &ValidatorDecorator) -> Option<String> {
    if decorator.name != "regex" {
        return None;
    }
    let Expr {
        kind: ExprKind::RegexLit { pattern, flags },
        ..
    } = decorator.args.first()?
    else {
        return None;
    };
    regex_runtime_pattern(pattern, flags)
        .and_then(|pattern| {
            regex::Regex::new(&pattern).map_err(|error| {
                format!(
                    "`@regex` pattern is invalid: {error}. Look-around assertions and backreferences are not supported"
                )
            })
        })
        .err()
}

fn arg_matches(kind: ArgKind, arg: &Expr) -> bool {
    match kind {
        ArgKind::Number => matches!(
            arg.kind(),
            ExprKind::IntLit(_) | ExprKind::UIntLit(_) | ExprKind::FloatLit(_)
        ),
        ArgKind::Integer => matches!(arg.kind(), ExprKind::IntLit(_) | ExprKind::UIntLit(_)),
        ArgKind::String => matches!(arg.kind(), ExprKind::StrLit(_)),
        ArgKind::Regex => matches!(arg.kind(), ExprKind::RegexLit { .. }),
        ArgKind::Primitive => matches!(
            arg,
            Expr {
                kind: ExprKind::IntLit(_),
                ..
            } | Expr {
                kind: ExprKind::FloatLit(_),
                ..
            } | Expr {
                kind: ExprKind::StrLit(_),
                ..
            } | Expr {
                kind: ExprKind::BoolLit(_),
                ..
            } | Expr {
                kind: ExprKind::Null,
                ..
            } | Expr {
                kind: ExprKind::UIntLit(_),
                ..
            }
        ),
        ArgKind::Array => matches!(arg.kind(), ExprKind::ArrayLit(_)),
    }
}

fn arg_kind_label(kind: ArgKind) -> &'static str {
    match kind {
        ArgKind::Number => "a numeric literal",
        ArgKind::Integer => "an integer literal",
        ArgKind::String => "a string literal",
        ArgKind::Regex => "a regular expression literal",
        ArgKind::Primitive => "a primitive literal",
        ArgKind::Array => "an array literal",
    }
}

pub fn regex_runtime_pattern(pattern: &str, flags: &str) -> Result<String, String> {
    regex_runtime_pattern_with_context(pattern, flags, "@regex")
}

pub fn regex_runtime_pattern_with_context(
    pattern: &str,
    flags: &str,
    context: &str,
) -> Result<String, String> {
    let mut normalized_flags = String::new();
    for flag in flags.chars() {
        match flag {
            'i' | 'm' | 's' | 'x' => {
                if !normalized_flags.contains(flag) {
                    normalized_flags.push(flag);
                }
            }
            other => {
                return Err(format!(
                    "`{context}` flag `{other}` is not supported; supported flags are `i`, `m`, `s`, and `x`"
                ));
            }
        }
    }
    if normalized_flags.is_empty() {
        Ok(pattern.to_string())
    } else {
        Ok(format!("(?{normalized_flags}){pattern}"))
    }
}

fn target_help(target: ValidatorTarget) -> &'static str {
    match target {
        ValidatorTarget::Numeric => "use this validator on a numeric type",
        ValidatorTarget::Integer => "use this validator on an integer type",
        ValidatorTarget::String => "use this validator on `string`",
        ValidatorTarget::Array => "use this validator on an array type",
        ValidatorTarget::Any => "use a supported validator for this target",
        ValidatorTarget::Optional => "use this validator on a nullable `T?` type",
    }
}

fn signature(spec: &ValidatorSpec) -> String {
    let mut args = spec
        .required_args
        .iter()
        .map(|kind| arg_kind_signature(*kind, false))
        .collect::<Vec<_>>();
    args.extend(
        spec.optional_args
            .iter()
            .map(|kind| arg_kind_signature(*kind, true)),
    );
    format!("@{}({})", spec.name, args.join(", "))
}

fn arg_kind_signature(kind: ArgKind, optional: bool) -> String {
    let label = match kind {
        ArgKind::Number => "number",
        ArgKind::Integer => "integer",
        ArgKind::String => "string",
        ArgKind::Regex => "Regex",
        ArgKind::Primitive => "primitive",
        ArgKind::Array => "array",
    };
    if optional {
        format!("{label}?")
    } else {
        label.to_string()
    }
}
