use crate::ast::{
    AssignOp, BinOp, DecimalLiteral, DestructureBinding, Expr, ExprKind, Field, FloatLiteral,
    FormatSpec, Item, Param, ParamPattern, Program, ResolvedEnum, Stmt, StmtKind, TemplatePart,
    Type, place_root_ident, resolve_enum,
};
use crate::collections::{HashMap, HashSet};
use crate::comptime::{
    CYCLE, ComptimeDiagnostic, ComptimeFacts, ComptimeRelatedInformation, DEPTH_LIMIT,
    FORBIDDEN_EFFECT, INVALID_TARGET, NON_CONSTANT, STEP_LIMIT, UNSUPPORTED_VALUE, VALUE_LIMIT,
};
use crate::span::Span;
use std::collections::BTreeMap;
use tyrus_decimal::{Decimal, DecimalError};

pub const DEFAULT_STEP_LIMIT: u64 = 1_000_000;
pub const DEFAULT_CALL_DEPTH_LIMIT: usize = 256;
pub const DEFAULT_VALUE_ELEMENT_LIMIT: usize = 65_536;
pub const DEFAULT_VALUE_BYTE_LIMIT: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum ConstValue {
    Bool(bool),
    Int {
        value: i128,
        ty: Type,
    },
    UInt {
        value: u128,
        ty: Type,
    },
    Float {
        value: f64,
        text: String,
        ty: Type,
    },
    Decimal(Decimal),
    Char(char),
    String(String),
    Void,
    Array(Vec<ConstValue>),
    Tuple(Vec<ConstValue>),
    Record(BTreeMap<String, ConstValue>),
    Enum {
        enum_name: String,
        member: String,
        value: Box<ConstValue>,
    },
}

impl ConstValue {
    fn materialized_expr(&self, span: Span) -> Expr {
        let kind = match self {
            ConstValue::Bool(value) => ExprKind::BoolLit(*value),
            ConstValue::Int { value, ty } => {
                typed_literal(ExprKind::IntLit(*value), ty, Type::I32, span)
            }
            ConstValue::UInt { value, ty } => {
                typed_literal(ExprKind::UIntLit(*value), ty, Type::U128, span)
            }
            ConstValue::Float { value, text, ty } => {
                let literal = ExprKind::FloatLit(FloatLiteral::new(*value, text.clone()));
                typed_literal(literal, ty, Type::F64, span)
            }
            ConstValue::Decimal(value) => {
                ExprKind::DecimalLit(DecimalLiteral::new(decimal_literal_text(*value)))
            }
            ConstValue::Char(value) => ExprKind::StrLit(value.to_string()),
            ConstValue::String(value) => ExprKind::StrLit(value.clone()),
            ConstValue::Void => ExprKind::If {
                cond: Box::new(Expr::new(ExprKind::BoolLit(true), span)),
                then_block: Vec::new(),
                else_block: None,
            },
            ConstValue::Array(values) | ConstValue::Tuple(values) => ExprKind::ArrayLit(
                values
                    .iter()
                    .map(|value| value.materialized_expr(span))
                    .collect(),
            ),
            ConstValue::Record(fields) => ExprKind::ObjectLit(
                fields
                    .iter()
                    .map(|(name, value)| (name.clone(), value.materialized_expr(span)))
                    .collect(),
            ),
            ConstValue::Enum {
                enum_name, member, ..
            } => ExprKind::FieldAccess {
                object: Box::new(Expr::new(ExprKind::Ident(enum_name.clone()), span)),
                field: member.clone(),
            },
        };
        Expr::new(kind, span)
    }

    fn element_count(&self) -> usize {
        match self {
            ConstValue::Array(values) | ConstValue::Tuple(values) => {
                values.iter().fold(1usize, |count, value| {
                    count.saturating_add(value.element_count())
                })
            }
            ConstValue::Record(fields) => fields.values().fold(1usize, |count, value| {
                count.saturating_add(value.element_count())
            }),
            ConstValue::Enum { value, .. } => 1usize.saturating_add(value.element_count()),
            _ => 1,
        }
    }

    fn byte_size(&self) -> usize {
        match self {
            ConstValue::Bool(_) => 1,
            ConstValue::Int { .. } | ConstValue::UInt { .. } => 16,
            ConstValue::Float { text, .. } => 8 + text.len(),
            ConstValue::Decimal(_) => 20,
            ConstValue::Char(_) => 4,
            ConstValue::String(value) => value.len(),
            ConstValue::Void => 0,
            ConstValue::Array(values) | ConstValue::Tuple(values) => {
                values.iter().fold(24usize, |bytes, value| {
                    bytes.saturating_add(value.byte_size())
                })
            }
            ConstValue::Record(fields) => fields.iter().fold(24usize, |bytes, (name, value)| {
                bytes
                    .saturating_add(name.len())
                    .saturating_add(value.byte_size())
            }),
            ConstValue::Enum {
                enum_name,
                member,
                value,
            } => enum_name
                .len()
                .saturating_add(member.len())
                .saturating_add(value.byte_size()),
        }
    }

    fn is_numeric_zero(&self) -> bool {
        match self {
            ConstValue::Int { value, .. } => *value == 0,
            ConstValue::UInt { value, .. } => *value == 0,
            ConstValue::Float { value, .. } => *value == 0.0,
            ConstValue::Decimal(value) => value.coefficient == 0,
            _ => false,
        }
    }
}

fn typed_literal(kind: ExprKind, ty: &Type, default: Type, span: Span) -> ExprKind {
    if *ty == default {
        kind
    } else {
        ExprKind::TypeAssert {
            value: Box::new(Expr::new(kind, span)),
            ty: ty.clone(),
        }
    }
}

fn decimal_literal_text(value: Decimal) -> String {
    if value.exponent == 0 {
        value.coefficient.to_string()
    } else {
        format!("{}e{}", value.coefficient, value.exponent)
    }
}

fn float_literal_text(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else {
        value.to_string()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComptimeEvaluation {
    pub expression_span: Span,
    pub value: ConstValue,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CtfeResult {
    pub evaluations: Vec<ComptimeEvaluation>,
    pub diagnostics: Vec<ComptimeDiagnostic>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EvaluationLimits {
    pub steps: u64,
    pub call_depth: usize,
    pub value_elements: usize,
    pub value_bytes: usize,
}

impl Default for EvaluationLimits {
    fn default() -> Self {
        Self {
            steps: DEFAULT_STEP_LIMIT,
            call_depth: DEFAULT_CALL_DEPTH_LIMIT,
            value_elements: DEFAULT_VALUE_ELEMENT_LIMIT,
            value_bytes: DEFAULT_VALUE_BYTE_LIMIT,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct EvaluationBudget {
    limits: EvaluationLimits,
    steps: u64,
    call_depth: usize,
    temporary_bytes: usize,
}

impl EvaluationBudget {
    pub(crate) fn new(limits: EvaluationLimits) -> Self {
        Self {
            limits,
            steps: 0,
            call_depth: 0,
            temporary_bytes: 0,
        }
    }

    pub(crate) fn consume_step(&mut self) -> bool {
        let Some(next) = self.steps.checked_add(1) else {
            return false;
        };
        if next > self.limits.steps {
            return false;
        }
        self.steps = next;
        true
    }

    pub(crate) fn enter_call(&mut self) -> bool {
        let Some(next) = self.call_depth.checked_add(1) else {
            return false;
        };
        if next > self.limits.call_depth {
            return false;
        }
        self.call_depth = next;
        true
    }

    pub(crate) fn leave_call(&mut self) {
        self.call_depth = self.call_depth.saturating_sub(1);
    }

    pub(crate) fn reserve_temporary(&mut self, bytes: usize) -> bool {
        let Some(next) = self.temporary_bytes.checked_add(bytes) else {
            return false;
        };
        if next > self.limits.value_bytes {
            return false;
        }
        self.temporary_bytes = next;
        true
    }

    #[cfg(any(test, kani))]
    pub(crate) fn steps(&self) -> u64 {
        self.steps
    }

    #[cfg(any(test, kani))]
    pub(crate) fn call_depth(&self) -> usize {
        self.call_depth
    }

    #[cfg(any(test, kani))]
    pub(crate) fn temporary_bytes(&self) -> usize {
        self.temporary_bytes
    }
}

#[derive(Debug, Clone)]
struct CallFrame {
    name: String,
    call_span: Span,
    declaration_span: Span,
}

#[derive(Debug, Clone)]
struct EvalError {
    code: &'static str,
    message: String,
    span: Span,
    frames: Vec<CallFrame>,
    help: Vec<String>,
}

impl EvalError {
    fn diagnostic(self, wrapper_span: Span) -> ComptimeDiagnostic {
        let mut related = vec![ComptimeRelatedInformation {
            span: wrapper_span,
            message: "compile-time evaluation is required by this wrapper".to_string(),
        }];
        related.extend(
            self.frames
                .iter()
                .rev()
                .map(|frame| ComptimeRelatedInformation {
                    span: frame.call_span,
                    message: format!("comptime call to `{}` originates here", frame.name),
                }),
        );
        related.extend(
            self.frames
                .iter()
                .rev()
                .map(|frame| ComptimeRelatedInformation {
                    span: frame.declaration_span,
                    message: format!("comptime function `{}` is declared here", frame.name),
                }),
        );
        ComptimeDiagnostic {
            code: self.code,
            message: self.message,
            span: self.span,
            related,
            help: self.help,
            remove_wrapper: false,
        }
    }
}

#[derive(Clone, Copy)]
struct FunctionDef<'a> {
    params: &'a [Param],
    ret_type: &'a Type,
    body: &'a [Stmt],
    declaration_span: Span,
}

#[derive(Clone, Copy)]
struct GlobalDef<'a> {
    ty: &'a Type,
    value: &'a Expr,
    declaration_span: Span,
}

enum Flow {
    Next,
    Return(ConstValue),
    Break(Option<ConstValue>),
    Continue,
}

#[derive(Debug, Clone)]
struct LocalBinding {
    value: ConstValue,
    mutable: bool,
}

#[derive(Debug, Clone)]
enum PlaceSegment {
    Field(String),
    Index(usize),
}

struct Evaluator<'a> {
    functions: HashMap<&'a str, FunctionDef<'a>>,
    globals: HashMap<&'a str, GlobalDef<'a>>,
    enums: HashMap<&'a str, ResolvedEnum>,
    interfaces: HashMap<&'a str, &'a [Field]>,
    eligible: HashSet<&'a str>,
    global_values: HashMap<String, ConstValue>,
    resolving_globals: Vec<String>,
    scopes: Vec<HashMap<String, LocalBinding>>,
    call_stack: Vec<CallFrame>,
    budget: EvaluationBudget,
}

impl<'a> Evaluator<'a> {
    fn new(program: &'a Program, facts: &'a ComptimeFacts, limits: EvaluationLimits) -> Self {
        let mut functions = HashMap::default();
        let mut globals = HashMap::default();
        let mut enums = HashMap::default();
        let mut interfaces = HashMap::default();
        for item in &program.items {
            match item {
                Item::FnDecl {
                    name,
                    params,
                    ret_type,
                    body,
                    span,
                    ..
                } => {
                    functions.insert(
                        name.as_str(),
                        FunctionDef {
                            params,
                            ret_type,
                            body,
                            declaration_span: *span,
                        },
                    );
                }
                Item::Stmt(Stmt {
                    kind:
                        StmtKind::VarDecl {
                            mutable: false,
                            name,
                            ty,
                            value: Some(value),
                            ..
                        },
                    span,
                }) => {
                    globals.insert(
                        name.as_str(),
                        GlobalDef {
                            ty,
                            value,
                            declaration_span: *span,
                        },
                    );
                }
                Item::EnumDecl {
                    name,
                    repr,
                    members,
                    ..
                } => {
                    enums.insert(name.as_str(), resolve_enum(repr, members).0);
                }
                Item::InterfaceDecl { name, fields, .. } => {
                    interfaces.insert(name.as_str(), fields.as_slice());
                }
                _ => {}
            }
        }
        let eligible = facts
            .entities
            .iter()
            .filter(|entity| entity.is_comptime && entity.eligible)
            .map(|entity| entity.name.as_str())
            .collect();
        Self {
            functions,
            globals,
            enums,
            interfaces,
            eligible,
            global_values: HashMap::default(),
            resolving_globals: Vec::new(),
            scopes: vec![HashMap::default()],
            call_stack: Vec::new(),
            budget: EvaluationBudget::new(limits),
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<ConstValue, EvalError> {
        self.step(expr.span)?;
        let value = match expr.kind() {
            ExprKind::BoolLit(value) => ConstValue::Bool(*value),
            ExprKind::IntLit(value) => self.checked_int(*value, Type::I32, expr.span)?,
            ExprKind::UIntLit(value) => ConstValue::UInt {
                value: *value,
                ty: Type::U128,
            },
            ExprKind::FloatLit(value) => ConstValue::Float {
                value: value.value(),
                text: value.text().to_string(),
                ty: Type::F64,
            },
            ExprKind::DecimalLit(value) => {
                ConstValue::Decimal(Decimal::parse(value.text()).ok_or_else(|| {
                    self.error(
                        UNSUPPORTED_VALUE,
                        "decimal literal is outside the decimal range",
                        expr.span,
                        "use a decimal literal within the documented decimal range",
                    )
                })?)
            }
            ExprKind::StrLit(value) => {
                let mut chars = value.chars();
                if let (Some(character), None) = (chars.next(), chars.next()) {
                    ConstValue::Char(character)
                } else {
                    ConstValue::String(value.clone())
                }
            }
            ExprKind::Ident(name) => self.lookup(name, expr.span)?,
            ExprKind::BinOp { op, left, right } => self.eval_binary(op, left, right, expr.span)?,
            ExprKind::ChainedComparison { terms, ops } => {
                self.eval_chained_comparison(terms, ops, expr.span)?
            }
            ExprKind::Not(inner) => {
                ConstValue::Bool(!self.eval_expr(inner)?.into_bool(self, inner.span)?)
            }
            ExprKind::Neg(inner) => self.eval_neg(inner, expr.span)?,
            ExprKind::BitNot(inner) => self.eval_bit_not(inner, expr.span)?,
            ExprKind::PrefixIncr(inner) => self.eval_increment(inner, true, true, expr.span)?,
            ExprKind::PrefixDecr(inner) => self.eval_increment(inner, false, true, expr.span)?,
            ExprKind::PostfixIncr(inner) => self.eval_increment(inner, true, false, expr.span)?,
            ExprKind::PostfixDecr(inner) => self.eval_increment(inner, false, false, expr.span)?,
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                if self.eval_expr(cond)?.into_bool(self, cond.span)? {
                    self.eval_expr(then_expr)?
                } else {
                    self.eval_expr(else_expr)?
                }
            }
            ExprKind::TypeAssert { value, ty } => {
                let value = self.eval_expr(value)?;
                self.coerce(value, ty, expr.span)?
            }
            ExprKind::ConstAssert(inner) | ExprKind::Comptime(inner) => self.eval_expr(inner)?,
            ExprKind::FnCall { callee, args, .. } => self.eval_call(callee, args, expr.span)?,
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => match self.eval_if(cond, then_block, else_block.as_deref())? {
                Flow::Next => ConstValue::Void,
                Flow::Return(value) => value,
                Flow::Break(_) | Flow::Continue => {
                    return Err(self.error(
                        UNSUPPORTED_VALUE,
                        "loop transfer cannot escape an expression block during comptime evaluation",
                        expr.span,
                        "keep `break` and `continue` inside an enclosing loop",
                    ));
                }
            },
            ExprKind::Loop { body } => match self.eval_loop(body)? {
                Flow::Return(value) => value,
                Flow::Break(Some(value)) => value,
                Flow::Break(None) | Flow::Continue => {
                    return Err(self.error(
                        UNSUPPORTED_VALUE,
                        "a `loop` expression without a `break` value cannot produce a comptime value",
                        expr.span,
                        "exit the loop with `break <value>`",
                    ));
                }
                Flow::Next => unreachable!("`eval_loop` only exits through `break` or `return`"),
            },
            ExprKind::TemplateLit(parts) => self.eval_template(parts, expr.span)?,
            ExprKind::ArrayLit(values) => self.eval_array(values, expr.span)?,
            ExprKind::ArrayRepeat { value, count } => {
                self.eval_array_repeat(value, count, expr.span)?
            }
            ExprKind::ObjectLit(fields) => self.eval_record(fields, expr.span)?,
            ExprKind::ObjectUpdate { base, fields } => {
                self.eval_object_update(base, fields, expr.span)?
            }
            ExprKind::FieldAccess { object, field } => {
                self.eval_field_access(object, field, expr.span)?
            }
            ExprKind::IndexAccess { object, index } => {
                self.eval_index_access(object, index, expr.span)?
            }
            ExprKind::In { key, object } => self.eval_in(key, object, expr.span)?,
            ExprKind::Range { .. } => {
                return Err(self.error(
                    UNSUPPORTED_VALUE,
                    "a range is iterable during comptime evaluation but is not materializable",
                    expr.span,
                    "consume the range in a `for ... of` loop",
                ));
            }
            ExprKind::OptionalFieldAccess { .. } => {
                return Err(self.error(
                    UNSUPPORTED_VALUE,
                    "optional values are not materializable comptime values",
                    expr.span,
                    "use a present record value and direct field access",
                ));
            }
            ExprKind::Null => {
                return Err(self.error(
                    UNSUPPORTED_VALUE,
                    "absence is not a materializable comptime value",
                    expr.span,
                    "return a present scalar value",
                ));
            }
            _ => {
                return Err(self.error(
                    UNSUPPORTED_VALUE,
                    "this expression is not supported by scalar comptime evaluation",
                    expr.span,
                    "use literals, immutable bindings, conditionals, scalar operators, and eligible comptime calls",
                ));
            }
        };
        self.check_value_limits(&value, expr.span)?;
        Ok(value)
    }

    fn eval_template(
        &mut self,
        parts: &[TemplatePart],
        _span: Span,
    ) -> Result<ConstValue, EvalError> {
        let mut output = String::new();
        for part in parts {
            match part {
                TemplatePart::Text(text) => output.push_str(text),
                TemplatePart::Expr(value, None) => {
                    output.push_str(&self.eval_expr(value)?.plain_text(self, value.span)?);
                }
                TemplatePart::Expr(value, Some(FormatSpec::FixedPoint(precision))) => {
                    let value_span = value.span;
                    let value = self.eval_expr(value)?;
                    let ConstValue::Float { value, .. } = value else {
                        return Err(self.error(
                            UNSUPPORTED_VALUE,
                            "fixed-point comptime formatting requires a floating-point value",
                            value_span,
                            "interpolate a floating-point value with this format specifier",
                        ));
                    };
                    output.push_str(&format!(
                        "{value:.precision$}",
                        precision = *precision as usize
                    ));
                }
                TemplatePart::Expr(value, Some(FormatSpec::ZeroPadded(width))) => {
                    let value_span = value.span;
                    let value = self.eval_expr(value)?;
                    let text = match value {
                        ConstValue::Int { value, .. } => {
                            format!("{value:0width$}", width = *width as usize)
                        }
                        ConstValue::UInt { value, .. } => {
                            format!("{value:0width$}", width = *width as usize)
                        }
                        _ => {
                            return Err(self.error(
                                UNSUPPORTED_VALUE,
                                "zero-padded comptime formatting requires an integer value",
                                value_span,
                                "interpolate an integer value with this format specifier",
                            ));
                        }
                    };
                    output.push_str(&text);
                }
            }
        }
        Ok(ConstValue::String(output))
    }

    fn eval_array(&mut self, values: &[Expr], span: Span) -> Result<ConstValue, EvalError> {
        let mut result = Vec::new();
        for value in values {
            if let ExprKind::Spread(inner) = value.kind() {
                match self.eval_expr(inner)? {
                    ConstValue::Array(values) | ConstValue::Tuple(values) => result.extend(values),
                    _ => {
                        return Err(self.error(
                            UNSUPPORTED_VALUE,
                            "array spread requires a compile-time array or tuple",
                            inner.span,
                            "spread an array or tuple value",
                        ));
                    }
                }
            } else {
                result.push(self.eval_expr(value)?);
            }
        }
        let value = ConstValue::Array(result);
        self.ensure_materializable(&value, span)?;
        Ok(value)
    }

    fn eval_array_repeat(
        &mut self,
        value: &Expr,
        count: &Expr,
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        let count_val = self.eval_expr(count)?;
        let count_num = match &count_val {
            ConstValue::Int { value, .. } => usize::try_from(*value).ok(),
            ConstValue::UInt { value, .. } => usize::try_from(*value).ok(),
            _ => None,
        }
        .filter(|count| (1..=DEFAULT_VALUE_ELEMENT_LIMIT).contains(count))
        .ok_or_else(|| {
            self.error(
                VALUE_LIMIT,
                format!("array repeat count must be between 1 and {DEFAULT_VALUE_ELEMENT_LIMIT}"),
                count.span,
                "use a positive count within the compile-time value limit",
            )
        })?;
        let element = self.eval_expr(value)?;
        let result = vec![element; count_num];
        let value = ConstValue::Array(result);
        self.ensure_materializable(&value, span)?;
        Ok(value)
    }

    fn eval_record(
        &mut self,
        fields: &[(String, Expr)],
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        let mut result = BTreeMap::new();
        for (name, value) in fields {
            result.insert(name.clone(), self.eval_expr(value)?);
        }
        let value = ConstValue::Record(result);
        self.ensure_materializable(&value, span)?;
        Ok(value)
    }

    fn eval_object_update(
        &mut self,
        base: &Expr,
        fields: &[(String, Expr)],
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        let ConstValue::Record(mut result) = self.eval_expr(base)? else {
            return Err(self.error(
                UNSUPPORTED_VALUE,
                "object update requires a compile-time record",
                base.span,
                "update a record value",
            ));
        };
        for (name, value) in fields {
            result.insert(name.clone(), self.eval_expr(value)?);
        }
        let value = ConstValue::Record(result);
        self.ensure_materializable(&value, span)?;
        Ok(value)
    }

    fn eval_field_access(
        &mut self,
        object: &Expr,
        field: &str,
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        if let ExprKind::Ident(enum_name) = object.kind()
            && let Some(enum_info) = self.enums.get(enum_name.as_str())
        {
            let member_value = enum_info
                .members
                .iter()
                .find(|(member, _)| member == field)
                .map(|(_, value)| value.clone())
                .ok_or_else(|| {
                    self.error(
                        UNSUPPORTED_VALUE,
                        format!("enum `{enum_name}` has no member `{field}`"),
                        span,
                        format!("use a declared member of `{enum_name}`"),
                    )
                })?;
            let value = self.eval_expr(&member_value)?;
            return Ok(ConstValue::Enum {
                enum_name: enum_name.clone(),
                member: field.to_string(),
                value: Box::new(value),
            });
        }
        match self.eval_expr(object)? {
            ConstValue::Record(fields) => fields.get(field).cloned().ok_or_else(|| {
                self.error(
                    UNSUPPORTED_VALUE,
                    format!("compile-time record has no field `{field}`"),
                    span,
                    format!("read an existing record field instead of `{field}`"),
                )
            }),
            ConstValue::Array(values) | ConstValue::Tuple(values) if field == "length" => {
                self.checked_uint(values.len() as u128, Type::Usize, span)
            }
            ConstValue::String(value) if field == "length" => {
                self.checked_uint(value.chars().count() as u128, Type::Usize, span)
            }
            _ => Err(self.error(
                UNSUPPORTED_VALUE,
                format!("field `{field}` is not available on this compile-time value"),
                span,
                format!("read `{field}` from a record that declares it"),
            )),
        }
    }

    fn eval_index_access(
        &mut self,
        object: &Expr,
        index: &Expr,
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        let object = self.eval_expr(object)?;
        let index_value = self.eval_expr(index)?;
        let index = self.value_index(index_value, index.span)?;
        match object {
            ConstValue::Array(values) | ConstValue::Tuple(values) => values
                .get(index)
                .cloned()
                .ok_or_else(|| self.index_error(index, values.len(), span)),
            ConstValue::String(value) => value
                .chars()
                .nth(index)
                .map(ConstValue::Char)
                .ok_or_else(|| self.index_error(index, value.chars().count(), span)),
            _ => Err(self.error(
                UNSUPPORTED_VALUE,
                "compile-time indexing requires an array, tuple, or string",
                span,
                "index a supported structural value",
            )),
        }
    }

    fn eval_in(&mut self, key: &Expr, object: &Expr, span: Span) -> Result<ConstValue, EvalError> {
        let key_span = key.span;
        let key = self.eval_expr(key)?;
        let key = match key {
            ConstValue::String(key) => key,
            ConstValue::Char(key) => key.to_string(),
            _ => {
                return Err(self.error(
                    UNSUPPORTED_VALUE,
                    "compile-time property membership requires a string key",
                    key_span,
                    "use a string field name",
                ));
            }
        };
        let ConstValue::Record(fields) = self.eval_expr(object)? else {
            return Err(self.error(
                UNSUPPORTED_VALUE,
                "compile-time property membership requires a record",
                object.span,
                "test membership on a record value",
            ));
        };
        let _ = span;
        Ok(ConstValue::Bool(fields.contains_key(&key)))
    }

    fn eval_increment(
        &mut self,
        inner: &Expr,
        increment: bool,
        prefix: bool,
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        let ExprKind::Ident(name) = inner.kind() else {
            return Err(self.error(
                UNSUPPORTED_VALUE,
                "comptime increment and decrement require a local binding",
                inner.span,
                "increment or decrement a mutable local binding",
            ));
        };
        let current = self.local_binding(name, inner.span)?.value.clone();
        let next = self.increment_value(current.clone(), increment, span)?;
        self.assign_local(name, next.clone(), span)?;
        Ok(if prefix { next } else { current })
    }

    fn eval_chained_comparison(
        &mut self,
        terms: &[Expr],
        ops: &[BinOp],
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        let Some(first) = terms.first() else {
            return Ok(ConstValue::Bool(true));
        };
        let mut left = self.eval_expr(first)?;
        for (op, term) in ops.iter().zip(terms.iter().skip(1)) {
            let right = self.eval_expr(term)?;
            if !self.compare_values(op, &left, &right, span)? {
                return Ok(ConstValue::Bool(false));
            }
            left = right;
        }
        Ok(ConstValue::Bool(true))
    }

    fn eval_binary(
        &mut self,
        op: &BinOp,
        left_expr: &Expr,
        right_expr: &Expr,
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        if matches!(op, BinOp::And) {
            let left = self.eval_expr(left_expr)?.into_bool(self, left_expr.span)?;
            return if left {
                Ok(ConstValue::Bool(
                    self.eval_expr(right_expr)?
                        .into_bool(self, right_expr.span)?,
                ))
            } else {
                Ok(ConstValue::Bool(false))
            };
        }
        if matches!(op, BinOp::Or) {
            let left = self.eval_expr(left_expr)?.into_bool(self, left_expr.span)?;
            return if left {
                Ok(ConstValue::Bool(true))
            } else {
                Ok(ConstValue::Bool(
                    self.eval_expr(right_expr)?
                        .into_bool(self, right_expr.span)?,
                ))
            };
        }
        let left = self.eval_expr(left_expr)?;
        let right = self.eval_expr(right_expr)?;
        self.eval_binary_values(op, left, right, span, right_expr.span)
    }

    fn eval_binary_values(
        &self,
        op: &BinOp,
        left: ConstValue,
        right: ConstValue,
        span: Span,
        right_span: Span,
    ) -> Result<ConstValue, EvalError> {
        if matches!(op, BinOp::Div | BinOp::Mod) && right.is_numeric_zero() {
            return Err(self.division_by_zero(right_span));
        }
        if matches!(
            op,
            BinOp::Eq | BinOp::Neq | BinOp::Gt | BinOp::Lt | BinOp::Gte | BinOp::Lte
        ) {
            let result = self.compare_values(op, &left, &right, span)?;
            return Ok(ConstValue::Bool(result));
        }
        match (left, right) {
            (ConstValue::Bool(left), ConstValue::Bool(right)) => match op {
                BinOp::And => Ok(ConstValue::Bool(left && right)),
                BinOp::Or => Ok(ConstValue::Bool(left || right)),
                _ => Err(self.error(
                    UNSUPPORTED_VALUE,
                    "operator operands are not supported by comptime evaluation",
                    span,
                    "use operands with compatible compile-time types",
                )),
            },
            (ConstValue::Int { value: left, ty }, ConstValue::Int { value: right, .. }) => {
                self.eval_signed_binary(op, left, right, ty, span)
            }
            (ConstValue::UInt { value: left, ty }, ConstValue::UInt { value: right, .. }) => {
                self.eval_unsigned_binary(op, left, right, ty, span)
            }
            (
                ConstValue::Float {
                    value: left, ty, ..
                },
                ConstValue::Float { value: right, .. },
            ) => self.eval_float_binary(op, left, right, ty, span),
            (ConstValue::Decimal(left), ConstValue::Decimal(right)) => {
                self.eval_decimal_binary(op, left, right, span)
            }
            (ConstValue::String(left), ConstValue::String(right)) if matches!(op, BinOp::Add) => {
                Ok(ConstValue::String(left + &right))
            }
            (ConstValue::Char(left), ConstValue::Char(right)) if matches!(op, BinOp::Add) => {
                Ok(ConstValue::String(format!("{left}{right}")))
            }
            _ => Err(self.error(
                UNSUPPORTED_VALUE,
                "operator operands are not supported by comptime evaluation",
                span,
                "use operands with compatible compile-time types",
            )),
        }
    }

    fn eval_signed_binary(
        &self,
        op: &BinOp,
        left: i128,
        right: i128,
        ty: Type,
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        let value = match op {
            BinOp::Add => left.checked_add(right),
            BinOp::Sub => left.checked_sub(right),
            BinOp::Mul => left.checked_mul(right),
            BinOp::Div if right == 0 => return Err(self.division_by_zero(span)),
            BinOp::Div => left.checked_div(right),
            BinOp::Mod if right == 0 => return Err(self.division_by_zero(span)),
            BinOp::Mod => left.checked_rem(right),
            BinOp::Pow => u32::try_from(right)
                .ok()
                .and_then(|power| left.checked_pow(power)),
            BinOp::BitAnd => Some(left & right),
            BinOp::BitOr => Some(left | right),
            BinOp::BitXor => Some(left ^ right),
            BinOp::Shl => u32::try_from(right)
                .ok()
                .and_then(|shift| left.checked_shl(shift)),
            BinOp::Shr => u32::try_from(right)
                .ok()
                .and_then(|shift| left.checked_shr(shift)),
            BinOp::UShr => {
                let bits = integer_bits(&ty).unwrap_or(128);
                u32::try_from(right)
                    .ok()
                    .filter(|shift| *shift < bits)
                    .and_then(|shift| {
                        let mask = if bits == 128 {
                            u128::MAX
                        } else {
                            (1_u128 << bits) - 1
                        };
                        i128::try_from(((left as u128) & mask) >> shift).ok()
                    })
            }
            _ => None,
        }
        .ok_or_else(|| self.integer_error(op, span))?;
        self.checked_int(value, ty, span)
    }

    fn eval_unsigned_binary(
        &self,
        op: &BinOp,
        left: u128,
        right: u128,
        ty: Type,
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        let value = match op {
            BinOp::Add => left.checked_add(right),
            BinOp::Sub => left.checked_sub(right),
            BinOp::Mul => left.checked_mul(right),
            BinOp::Div if right == 0 => return Err(self.division_by_zero(span)),
            BinOp::Div => left.checked_div(right),
            BinOp::Mod if right == 0 => return Err(self.division_by_zero(span)),
            BinOp::Mod => left.checked_rem(right),
            BinOp::Pow => u32::try_from(right)
                .ok()
                .and_then(|power| left.checked_pow(power)),
            BinOp::BitAnd => Some(left & right),
            BinOp::BitOr => Some(left | right),
            BinOp::BitXor => Some(left ^ right),
            BinOp::Shl => u32::try_from(right)
                .ok()
                .and_then(|shift| left.checked_shl(shift)),
            BinOp::Shr | BinOp::UShr => u32::try_from(right)
                .ok()
                .and_then(|shift| left.checked_shr(shift)),
            _ => None,
        }
        .ok_or_else(|| self.integer_error(op, span))?;
        self.checked_uint(value, ty, span)
    }

    fn eval_float_binary(
        &self,
        op: &BinOp,
        left: f64,
        right: f64,
        ty: Type,
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        if matches!(ty, Type::F128) {
            return Err(self.error(
                UNSUPPORTED_VALUE,
                "f128 arithmetic requires the exact CTFE floating-point backend",
                span,
                "use f32, f64, or decimal arithmetic in comptime expressions for now",
            ));
        }
        let value = if matches!(ty, Type::F32) {
            let left = left as f32;
            let right = right as f32;
            (match op {
                BinOp::Add => left + right,
                BinOp::Sub => left - right,
                BinOp::Mul => left * right,
                BinOp::Div => left / right,
                BinOp::Mod => left % right,
                BinOp::Pow => left.powf(right),
                _ => return Err(self.unsupported_operator(op, span)),
            }) as f64
        } else {
            match op {
                BinOp::Add => left + right,
                BinOp::Sub => left - right,
                BinOp::Mul => left * right,
                BinOp::Div => left / right,
                BinOp::Mod => left % right,
                BinOp::Pow => left.powf(right),
                _ => return Err(self.unsupported_operator(op, span)),
            }
        };
        Ok(ConstValue::Float {
            value,
            text: float_literal_text(value),
            ty,
        })
    }

    fn eval_decimal_binary(
        &self,
        op: &BinOp,
        left: Decimal,
        right: Decimal,
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        let result = match op {
            BinOp::Add => left.add(right),
            BinOp::Sub => left.sub(right),
            BinOp::Mul => left.mul(right),
            BinOp::Div => left.div(right),
            BinOp::Pow => left.pow(right),
            _ => return Err(self.unsupported_operator(op, span)),
        }
        .map_err(|error| self.decimal_error(error, span))?;
        Ok(ConstValue::Decimal(result))
    }

    fn compare_values(
        &self,
        op: &BinOp,
        left: &ConstValue,
        right: &ConstValue,
        span: Span,
    ) -> Result<bool, EvalError> {
        let left = enum_payload(left);
        let right = enum_payload(right);
        if matches!(op, BinOp::Eq | BinOp::Neq) {
            let equal = left == right;
            return Ok(if matches!(op, BinOp::Eq) {
                equal
            } else {
                !equal
            });
        }
        let ordering = match (left, right) {
            (ConstValue::Int { value: left, .. }, ConstValue::Int { value: right, .. }) => {
                Some(left.cmp(right))
            }
            (ConstValue::UInt { value: left, .. }, ConstValue::UInt { value: right, .. }) => {
                Some(left.cmp(right))
            }
            (ConstValue::Float { value: left, .. }, ConstValue::Float { value: right, .. }) => {
                left.partial_cmp(right)
            }
            (ConstValue::Decimal(left), ConstValue::Decimal(right)) => Some(
                left.compare(*right)
                    .map_err(|error| self.decimal_error(error, span))?,
            ),
            (ConstValue::Bool(left), ConstValue::Bool(right)) => Some(left.cmp(right)),
            (ConstValue::String(left), ConstValue::String(right)) => Some(left.cmp(right)),
            (ConstValue::Char(left), ConstValue::Char(right)) => Some(left.cmp(right)),
            _ => None,
        }
        .ok_or_else(|| {
            self.error(
                UNSUPPORTED_VALUE,
                "comparison operands are not compatible scalar comptime values",
                span,
                "compare values with the same scalar type",
            )
        })?;
        Ok(match op {
            BinOp::Eq => ordering.is_eq(),
            BinOp::Neq => !ordering.is_eq(),
            BinOp::Gt => ordering.is_gt(),
            BinOp::Lt => ordering.is_lt(),
            BinOp::Gte => !ordering.is_lt(),
            BinOp::Lte => !ordering.is_gt(),
            _ => return Err(self.unsupported_operator(op, span)),
        })
    }

    fn eval_neg(&mut self, inner: &Expr, span: Span) -> Result<ConstValue, EvalError> {
        match self.eval_expr(inner)? {
            ConstValue::Int { value, ty } => {
                let value = value
                    .checked_neg()
                    .ok_or_else(|| self.integer_error(&BinOp::Sub, span))?;
                self.checked_int(value, ty, span)
            }
            ConstValue::Float { value, ty, .. } => Ok(ConstValue::Float {
                value: -value,
                text: (-value).to_string(),
                ty,
            }),
            ConstValue::Decimal(value) => {
                let coefficient = value
                    .coefficient
                    .checked_neg()
                    .ok_or_else(|| self.decimal_error(DecimalError::OutOfRange, span))?;
                Ok(ConstValue::Decimal(Decimal {
                    coefficient,
                    exponent: value.exponent,
                }))
            }
            _ => Err(self.error(
                UNSUPPORTED_VALUE,
                "unary minus requires a signed numeric comptime value",
                span,
                "use a signed integer, float, or decimal operand",
            )),
        }
    }

    fn eval_bit_not(&mut self, inner: &Expr, span: Span) -> Result<ConstValue, EvalError> {
        match self.eval_expr(inner)? {
            ConstValue::Int { value, ty } => self.checked_int(!value, ty, span),
            ConstValue::UInt { value, ty } => self.checked_uint(!value, ty, span),
            _ => Err(self.error(
                UNSUPPORTED_VALUE,
                "bitwise not requires an integer comptime value",
                span,
                "use an integer operand",
            )),
        }
    }

    fn eval_call(
        &mut self,
        callee: &str,
        args: &[Expr],
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        if callee == "isNaN" {
            let value = args.first().ok_or_else(|| {
                self.error(
                    UNSUPPORTED_VALUE,
                    "`isNaN` requires a floating-point argument".to_string(),
                    span,
                    "pass a floating-point value to `isNaN`",
                )
            })?;
            let value = self.eval_expr(value)?;
            return match value {
                ConstValue::Float { value, .. } => Ok(ConstValue::Bool(value.is_nan())),
                value => Err(self.error(
                    UNSUPPORTED_VALUE,
                    format!("`isNaN` requires a floating-point argument, found `{value:?}`"),
                    span,
                    "pass a floating-point value to `isNaN`",
                )),
            };
        }
        if !self.eligible.contains(callee) {
            return Err(self.error(
                INVALID_TARGET,
                format!("`{callee}` is not an eligible comptime function"),
                span,
                format!("qualify and make `{callee}` eligible for comptime evaluation"),
            ));
        }
        let function = *self.functions.get(callee).ok_or_else(|| {
            self.error(
                INVALID_TARGET,
                format!("comptime function `{callee}` is not available in this module"),
                span,
                format!("define `{callee}` in the same module"),
            )
        })?;
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.eval_expr(arg)?);
        }
        if !self.budget.enter_call() {
            return Err(self.error(
                DEPTH_LIMIT,
                format!(
                    "comptime call depth exceeded the limit of {}",
                    self.budget.limits.call_depth
                ),
                span,
                format!(
                    "reduce recursion or split the computation so it stays below {} calls",
                    self.budget.limits.call_depth
                ),
            ));
        }
        self.call_stack.push(CallFrame {
            name: callee.to_string(),
            call_span: span,
            declaration_span: function.declaration_span,
        });
        self.scopes.push(HashMap::default());
        let result = (|| {
            for (index, param) in function.params.iter().enumerate() {
                let value = if let Some(value) = values.get(index).cloned() {
                    value
                } else if let Some(default) = &param.default_value {
                    self.eval_expr(default)?
                } else {
                    return Err(self.error(
                        UNSUPPORTED_VALUE,
                        format!("comptime call to `{callee}` is missing a required parameter"),
                        span,
                        format!("pass every required compile-time argument to `{callee}`"),
                    ));
                };
                let value = self.coerce(value, &param.ty, param.span)?;
                self.bind_param(&param.pattern, value, param.mutable, param.span)?;
            }
            self.eval_block(function.body, false)
                .and_then(|flow| match flow {
                    Flow::Return(value) => {
                        self.coerce(value, function.ret_type, function.declaration_span)
                    }
                    Flow::Next if matches!(function.ret_type, Type::Void) => Ok(ConstValue::Void),
                    Flow::Break(_) | Flow::Continue => Err(self.error(
                        UNSUPPORTED_VALUE,
                        format!(
                            "comptime function `{callee}` completed with an invalid loop transfer"
                        ),
                        function.declaration_span,
                        "keep `break` and `continue` inside their loop",
                    )),
                    Flow::Next => Err(self.error(
                        UNSUPPORTED_VALUE,
                        format!("comptime function `{callee}` completed without returning a value"),
                        function.declaration_span,
                        format!("return a `{}` value from `{callee}`", function.ret_type),
                    )),
                })
        })();
        self.scopes.pop();
        self.call_stack.pop();
        self.budget.leave_call();
        result
    }

    fn eval_block(&mut self, statements: &[Stmt], scoped: bool) -> Result<Flow, EvalError> {
        if scoped {
            self.scopes.push(HashMap::default());
        }
        let result = (|| {
            for statement in statements {
                self.step(statement.span)?;
                match self.eval_stmt(statement)? {
                    Flow::Next => {}
                    transfer => return Ok(transfer),
                }
            }
            Ok(Flow::Next)
        })();
        if scoped {
            self.scopes.pop();
        }
        result
    }

    fn eval_stmt(&mut self, statement: &Stmt) -> Result<Flow, EvalError> {
        match statement.kind() {
            StmtKind::VarDecl {
                mutable,
                name,
                ty,
                value,
                ..
            } => {
                let value = value.as_ref().ok_or_else(|| {
                    self.error(
                        NON_CONSTANT,
                        format!("compile-time local `{name}` has no initializer"),
                        statement.span,
                        format!("initialize `{name}` with a compile-time value"),
                    )
                })?;
                let value = self.eval_expr(value)?;
                let value = self.coerce(value, ty, statement.span)?;
                self.bind_local(name, value, *mutable);
                Ok(Flow::Next)
            }
            StmtKind::ArrayDestructure {
                bindings, value, ..
            } => {
                let value = self.eval_expr(value)?;
                self.bind_array_destructure(bindings, value, statement.span)?;
                Ok(Flow::Next)
            }
            StmtKind::ObjectDestructure {
                bindings, value, ..
            } => {
                let value = self.eval_expr(value)?;
                self.bind_object_destructure(bindings, value, statement.span)?;
                Ok(Flow::Next)
            }
            StmtKind::Assign { name, op, value } => {
                let right = self.eval_expr(value)?;
                let next = if matches!(op, AssignOp::Assign) {
                    right
                } else {
                    let left = self.local_binding(name, statement.span)?.value.clone();
                    let op = assignment_binop(op).ok_or_else(|| {
                        self.error(
                            UNSUPPORTED_VALUE,
                            "this assignment operator is not supported during comptime evaluation",
                            statement.span,
                            "use a supported arithmetic, logical, or bitwise assignment",
                        )
                    })?;
                    self.eval_binary_values(&op, left, right, statement.span, value.span)?
                };
                self.assign_local(name, next, statement.span)?;
                Ok(Flow::Next)
            }
            StmtKind::IndexAssign { name, index, value } => {
                let index_value = self.eval_expr(index)?;
                let index = self.value_index(index_value, index.span)?;
                let value = self.eval_expr(value)?;
                self.assign_index(name, index, value, statement.span)?;
                Ok(Flow::Next)
            }
            StmtKind::FieldAssign {
                object,
                field,
                op,
                value,
            } => {
                let right = self.eval_expr(value)?;
                self.assign_field(object, field, op, right, statement.span)?;
                Ok(Flow::Next)
            }
            StmtKind::Return(value) => Ok(Flow::Return(match value {
                Some(value) => self.eval_expr(value)?,
                None => ConstValue::Void,
            })),
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        cond,
                        then_block,
                        else_block,
                    },
                ..
            }) => self.eval_if(cond, then_block, else_block.as_deref()),
            StmtKind::Expr(Expr {
                kind: ExprKind::Loop { body },
                ..
            }) => match self.eval_loop(body)? {
                Flow::Break(_) => Ok(Flow::Next),
                Flow::Next => unreachable!("`eval_loop` only exits through `break` or `return`"),
                returned @ Flow::Return(_) => Ok(returned),
                Flow::Continue => unreachable!("`continue` is consumed by `eval_loop`"),
            },
            StmtKind::Expr(value) => {
                self.eval_expr(value)?;
                Ok(Flow::Next)
            }
            StmtKind::Break(value) => Ok(Flow::Break(
                value
                    .as_ref()
                    .map(|value| self.eval_expr(value))
                    .transpose()?,
            )),
            StmtKind::Continue => Ok(Flow::Continue),
            StmtKind::While { cond, body } => self.eval_while(cond, body),
            StmtKind::DoWhile { body, cond } => self.eval_do_while(body, cond),
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => self.eval_for(init, cond, update, body),
            StmtKind::ForOf {
                mutable,
                name,
                iterable,
                body,
            } => self.eval_for_of(*mutable, name, iterable, body),
            StmtKind::ForIn { name, object, body } => self.eval_for_in(name, object, body),
            StmtKind::Switch {
                expr,
                cases,
                default,
                default_index,
            } => self.eval_switch(expr, cases, default.as_deref(), *default_index),
            _ => Err(self.error(
                UNSUPPORTED_VALUE,
                "this statement is not supported by comptime evaluation",
                statement.span,
                "use deterministic control flow without exceptions",
            )),
        }
    }

    fn eval_if(
        &mut self,
        cond: &Expr,
        then_block: &[Stmt],
        else_block: Option<&[Stmt]>,
    ) -> Result<Flow, EvalError> {
        if self.eval_expr(cond)?.into_bool(self, cond.span)? {
            self.eval_block(then_block, true)
        } else if let Some(else_block) = else_block {
            self.eval_block(else_block, true)
        } else {
            Ok(Flow::Next)
        }
    }

    fn eval_while(&mut self, cond: &Expr, body: &[Stmt]) -> Result<Flow, EvalError> {
        while self.eval_expr(cond)?.into_bool(self, cond.span)? {
            match self.eval_block(body, true)? {
                Flow::Next | Flow::Continue => {}
                Flow::Break(_) => return Ok(Flow::Next),
                returned @ Flow::Return(_) => return Ok(returned),
            }
        }
        Ok(Flow::Next)
    }

    fn eval_do_while(&mut self, body: &[Stmt], cond: &Expr) -> Result<Flow, EvalError> {
        loop {
            match self.eval_block(body, true)? {
                Flow::Next | Flow::Continue => {}
                Flow::Break(_) => return Ok(Flow::Next),
                returned @ Flow::Return(_) => return Ok(returned),
            }
            if !self.eval_expr(cond)?.into_bool(self, cond.span)? {
                return Ok(Flow::Next);
            }
        }
    }

    fn eval_loop(&mut self, body: &[Stmt]) -> Result<Flow, EvalError> {
        loop {
            match self.eval_block(body, true)? {
                Flow::Next | Flow::Continue => {}
                Flow::Break(value) => return Ok(Flow::Break(value)),
                returned @ Flow::Return(_) => return Ok(returned),
            }
        }
    }

    fn eval_for(
        &mut self,
        init: &Stmt,
        cond: &Expr,
        update: &Stmt,
        body: &[Stmt],
    ) -> Result<Flow, EvalError> {
        self.scopes.push(HashMap::default());
        let result = (|| {
            if !matches!(self.eval_stmt(init)?, Flow::Next) {
                return Err(self.error(
                    UNSUPPORTED_VALUE,
                    "comptime `for` initializer cannot transfer control",
                    init.span,
                    "use a variable declaration or assignment as the initializer",
                ));
            }
            while self.eval_expr(cond)?.into_bool(self, cond.span)? {
                match self.eval_block(body, true)? {
                    Flow::Next | Flow::Continue => {}
                    Flow::Break(_) => return Ok(Flow::Next),
                    returned @ Flow::Return(_) => return Ok(returned),
                }
                if !matches!(self.eval_stmt(update)?, Flow::Next) {
                    return Err(self.error(
                        UNSUPPORTED_VALUE,
                        "comptime `for` update cannot transfer control",
                        update.span,
                        "use an assignment or increment as the update",
                    ));
                }
            }
            Ok(Flow::Next)
        })();
        self.scopes.pop();
        result
    }

    fn eval_for_of(
        &mut self,
        mutable: bool,
        name: &str,
        iterable: &Expr,
        body: &[Stmt],
    ) -> Result<Flow, EvalError> {
        let values = if let ExprKind::Range {
            start,
            end,
            inclusive,
        } = iterable.kind()
        {
            self.eval_range_values(start, end, *inclusive, iterable.span)?
        } else {
            match self.eval_expr(iterable)? {
                ConstValue::Array(values) | ConstValue::Tuple(values) => values,
                ConstValue::String(value) => value.chars().map(ConstValue::Char).collect(),
                _ => {
                    return Err(self.error(
                        UNSUPPORTED_VALUE,
                        "comptime `for ... of` requires an array, tuple, string, or range",
                        iterable.span,
                        "iterate over a supported compile-time value",
                    ));
                }
            }
        };
        for value in values {
            self.scopes.push(HashMap::default());
            self.bind_local(name, value, mutable);
            let flow = self.eval_block(body, false);
            self.scopes.pop();
            match flow? {
                Flow::Next | Flow::Continue => {}
                Flow::Break(_) => return Ok(Flow::Next),
                returned @ Flow::Return(_) => return Ok(returned),
            }
        }
        Ok(Flow::Next)
    }

    fn eval_for_in(&mut self, name: &str, object: &Expr, body: &[Stmt]) -> Result<Flow, EvalError> {
        let ConstValue::Record(fields) = self.eval_expr(object)? else {
            return Err(self.error(
                UNSUPPORTED_VALUE,
                "comptime `for ... in` requires a record",
                object.span,
                "iterate over a record value",
            ));
        };
        for key in fields.into_keys() {
            self.scopes.push(HashMap::default());
            self.bind_local(name, ConstValue::String(key), false);
            let flow = self.eval_block(body, false);
            self.scopes.pop();
            match flow? {
                Flow::Next | Flow::Continue => {}
                Flow::Break(_) => return Ok(Flow::Next),
                returned @ Flow::Return(_) => return Ok(returned),
            }
        }
        Ok(Flow::Next)
    }

    fn eval_switch(
        &mut self,
        expr: &Expr,
        cases: &[crate::ast::SwitchCase],
        default: Option<&[Stmt]>,
        default_index: Option<usize>,
    ) -> Result<Flow, EvalError> {
        let subject = self.eval_expr(expr)?;
        let mut selected = None;
        for (index, case) in cases.iter().enumerate() {
            let value = self.eval_expr(&case.value)?;
            if self.compare_values(&BinOp::Eq, &subject, &value, case.value.span)? {
                selected = Some(index + usize::from(default_index.is_some_and(|at| index >= at)));
                break;
            }
        }
        let start = selected.or(default_index);
        let Some(start) = start else {
            return Ok(Flow::Next);
        };
        let total = cases.len() + usize::from(default.is_some());
        for position in start..total {
            let body = if default_index == Some(position) {
                default.unwrap_or_default()
            } else {
                let case_index =
                    position - usize::from(default_index.is_some_and(|at| position > at));
                &cases[case_index].body
            };
            match self.eval_block(body, true)? {
                Flow::Next => {}
                Flow::Break(_) => return Ok(Flow::Next),
                transfer => return Ok(transfer),
            }
        }
        Ok(Flow::Next)
    }

    fn eval_range_values(
        &mut self,
        start: &Expr,
        end: &Expr,
        inclusive: bool,
        span: Span,
    ) -> Result<Vec<ConstValue>, EvalError> {
        let start = self.eval_expr(start)?;
        let end = self.eval_expr(end)?;
        match (start, end) {
            (ConstValue::Int { value: start, ty }, ConstValue::Int { value: end, .. }) => {
                let mut values = Vec::new();
                let mut current = start;
                while current < end || (inclusive && current == end) {
                    self.push_range_value(
                        &mut values,
                        self.checked_int(current, ty.clone(), span)?,
                        span,
                    )?;
                    current = current
                        .checked_add(1)
                        .ok_or_else(|| self.integer_error(&BinOp::Add, span))?;
                }
                Ok(values)
            }
            (ConstValue::UInt { value: start, ty }, ConstValue::UInt { value: end, .. }) => {
                let mut values = Vec::new();
                let mut current = start;
                while current < end || (inclusive && current == end) {
                    self.push_range_value(
                        &mut values,
                        self.checked_uint(current, ty.clone(), span)?,
                        span,
                    )?;
                    current = current
                        .checked_add(1)
                        .ok_or_else(|| self.integer_error(&BinOp::Add, span))?;
                }
                Ok(values)
            }
            _ => Err(self.error(
                UNSUPPORTED_VALUE,
                "comptime ranges require integer bounds of the same type",
                span,
                "use integer bounds of the same type for this range",
            )),
        }
    }

    fn push_range_value(
        &mut self,
        values: &mut Vec<ConstValue>,
        value: ConstValue,
        span: Span,
    ) -> Result<(), EvalError> {
        self.step(span)?;
        if values.len() >= self.budget.limits.value_elements
            || !self.budget.reserve_temporary(value.byte_size())
        {
            return Err(self.error(
                VALUE_LIMIT,
                "comptime range exceeded the configured value limit",
                span,
                "use a smaller compile-time range",
            ));
        }
        values.push(value);
        Ok(())
    }

    fn bind_local(&mut self, name: &str, value: ConstValue, mutable: bool) {
        self.scopes
            .last_mut()
            .expect("CTFE always has a lexical scope")
            .insert(name.to_string(), LocalBinding { value, mutable });
    }

    fn local_binding(&self, name: &str, span: Span) -> Result<&LocalBinding, EvalError> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .ok_or_else(|| {
                self.error(
                    NON_CONSTANT,
                    format!("`{name}` is not a local compile-time binding"),
                    span,
                    format!("declare `{name}` inside the comptime function"),
                )
            })
    }

    fn assign_local(&mut self, name: &str, value: ConstValue, span: Span) -> Result<(), EvalError> {
        let Some(index) = self
            .scopes
            .iter()
            .rposition(|scope| scope.contains_key(name))
        else {
            return Err(self.error(
                FORBIDDEN_EFFECT,
                format!("comptime evaluation cannot write external binding `{name}`"),
                span,
                format!("declare mutable local `{name}` inside the comptime function"),
            ));
        };
        if !self.scopes[index][name].mutable {
            return Err(self.error(
                FORBIDDEN_EFFECT,
                format!("compile-time binding `{name}` is immutable"),
                span,
                format!("declare `{name}` with `let` if local mutation is required"),
            ));
        }
        self.ensure_materializable(&value, span)?;
        self.scopes[index]
            .get_mut(name)
            .expect("binding exists")
            .value = value;
        Ok(())
    }

    fn bind_param(
        &mut self,
        pattern: &ParamPattern,
        value: ConstValue,
        mutable: bool,
        span: Span,
    ) -> Result<(), EvalError> {
        match pattern {
            ParamPattern::Name(name) => self.bind_local(name, value, mutable),
            ParamPattern::Array(names) => {
                let (ConstValue::Array(values) | ConstValue::Tuple(values)) = value else {
                    return Err(self.error(
                        UNSUPPORTED_VALUE,
                        "array parameter destructuring requires an array or tuple",
                        span,
                        "pass an array or tuple to this parameter",
                    ));
                };
                if values.len() < names.len() {
                    return Err(self.index_error(names.len() - 1, values.len(), span));
                }
                for (name, value) in names.iter().zip(values) {
                    self.bind_local(name, value, mutable);
                }
            }
            ParamPattern::Object(names) => {
                let ConstValue::Record(fields) = value else {
                    return Err(self.error(
                        UNSUPPORTED_VALUE,
                        "object parameter destructuring requires a record",
                        span,
                        "pass a record to this parameter",
                    ));
                };
                for name in names {
                    let value = fields.get(name).cloned().ok_or_else(|| {
                        self.error(
                            UNSUPPORTED_VALUE,
                            format!("compile-time record has no field `{name}`"),
                            span,
                            format!("pass a record containing `{name}`"),
                        )
                    })?;
                    self.bind_local(name, value, mutable);
                }
            }
        }
        Ok(())
    }

    fn bind_array_destructure(
        &mut self,
        bindings: &[DestructureBinding],
        value: ConstValue,
        span: Span,
    ) -> Result<(), EvalError> {
        let (ConstValue::Array(values) | ConstValue::Tuple(values)) = value else {
            return Err(self.error(
                UNSUPPORTED_VALUE,
                "array destructuring requires an array or tuple",
                span,
                "destructure an array or tuple value",
            ));
        };
        if values.len() < bindings.len() {
            return Err(self.index_error(bindings.len() - 1, values.len(), span));
        }
        for (binding, value) in bindings.iter().zip(values) {
            if !binding.discarded {
                let value = self.coerce(value, &binding.ty, span)?;
                self.bind_local(&binding.name, value, binding.mutable);
            }
        }
        Ok(())
    }

    fn bind_object_destructure(
        &mut self,
        bindings: &[DestructureBinding],
        value: ConstValue,
        span: Span,
    ) -> Result<(), EvalError> {
        let ConstValue::Record(fields) = value else {
            return Err(self.error(
                UNSUPPORTED_VALUE,
                "object destructuring requires a record",
                span,
                "destructure a record value",
            ));
        };
        for binding in bindings {
            if binding.discarded {
                continue;
            }
            let value = fields.get(&binding.name).cloned().ok_or_else(|| {
                self.error(
                    UNSUPPORTED_VALUE,
                    format!("compile-time record has no field `{}`", binding.name),
                    span,
                    format!(
                        "destructure an existing field instead of `{}`",
                        binding.name
                    ),
                )
            })?;
            let value = self.coerce(value, &binding.ty, span)?;
            self.bind_local(&binding.name, value, binding.mutable);
        }
        Ok(())
    }

    fn increment_value(
        &self,
        value: ConstValue,
        increment: bool,
        span: Span,
    ) -> Result<ConstValue, EvalError> {
        let one = match &value {
            ConstValue::Int { ty, .. } => ConstValue::Int {
                value: 1,
                ty: ty.clone(),
            },
            ConstValue::UInt { ty, .. } => ConstValue::UInt {
                value: 1,
                ty: ty.clone(),
            },
            ConstValue::Float { ty, .. } => ConstValue::Float {
                value: 1.0,
                text: "1.0".to_string(),
                ty: ty.clone(),
            },
            ConstValue::Decimal(_) => ConstValue::Decimal(Decimal::from_integer(1)),
            _ => {
                return Err(self.error(
                    UNSUPPORTED_VALUE,
                    "comptime increment and decrement require a numeric binding",
                    span,
                    "use an integer, float, or decimal binding",
                ));
            }
        };
        self.eval_binary_values(
            if increment { &BinOp::Add } else { &BinOp::Sub },
            value,
            one,
            span,
            span,
        )
    }

    fn value_index(&self, value: ConstValue, span: Span) -> Result<usize, EvalError> {
        match value {
            ConstValue::Int { value, .. } => usize::try_from(value),
            ConstValue::UInt { value, .. } => usize::try_from(value),
            _ => {
                return Err(self.error(
                    UNSUPPORTED_VALUE,
                    "compile-time index must be a non-negative integer",
                    span,
                    "use a non-negative integer index",
                ));
            }
        }
        .map_err(|_| {
            self.error(
                UNSUPPORTED_VALUE,
                "compile-time index is outside the platform-independent index range",
                span,
                "use a smaller non-negative index",
            )
        })
    }

    fn index_error(&self, index: usize, len: usize, span: Span) -> EvalError {
        self.error(
            UNSUPPORTED_VALUE,
            format!("compile-time index `{index}` is out of bounds for length `{len}`"),
            span,
            format!("use an index between `0` and `{}`", len.saturating_sub(1)),
        )
    }

    fn assign_index(
        &mut self,
        name: &str,
        index: usize,
        value: ConstValue,
        span: Span,
    ) -> Result<(), EvalError> {
        let mut root = self.local_binding(name, span)?.value.clone();
        let len = match &root {
            ConstValue::Array(values) | ConstValue::Tuple(values) => values.len(),
            _ => {
                return Err(self.error(
                    UNSUPPORTED_VALUE,
                    format!("compile-time binding `{name}` is not index-assignable"),
                    span,
                    format!("declare `{name}` as a mutable array or tuple"),
                ));
            }
        };
        let values = match &mut root {
            ConstValue::Array(values) | ConstValue::Tuple(values) => values,
            _ => unreachable!(),
        };
        let Some(slot) = values.get_mut(index) else {
            return Err(self.index_error(index, len, span));
        };
        *slot = value;
        self.assign_local(name, root, span)
    }

    fn assign_field(
        &mut self,
        object: &Expr,
        field: &str,
        op: &AssignOp,
        right: ConstValue,
        span: Span,
    ) -> Result<(), EvalError> {
        let root_name = place_root_ident(object).ok_or_else(|| {
            self.error(
                FORBIDDEN_EFFECT,
                "comptime field assignment requires a mutable local root",
                object.span,
                "assign through a mutable local record",
            )
        })?;
        let mut segments = Vec::new();
        self.collect_place_segments(object, &mut segments)?;
        segments.push(PlaceSegment::Field(field.to_string()));
        let mut root = self.local_binding(root_name, span)?.value.clone();
        let next = if matches!(op, AssignOp::Assign) {
            right
        } else {
            let current = value_at_path(&root, &segments).cloned().ok_or_else(|| {
                self.error(
                    UNSUPPORTED_VALUE,
                    format!("compile-time place has no field `{field}`"),
                    span,
                    format!("assign an existing field instead of `{field}`"),
                )
            })?;
            let binary = assignment_binop(op).ok_or_else(|| {
                self.error(
                    UNSUPPORTED_VALUE,
                    "this field assignment operator is not supported during comptime evaluation",
                    span,
                    "use a supported arithmetic, logical, or bitwise assignment",
                )
            })?;
            self.eval_binary_values(&binary, current, right, span, span)?
        };
        if !set_value_at_path(&mut root, &segments, next) {
            return Err(self.error(
                UNSUPPORTED_VALUE,
                format!("compile-time place has no field `{field}`"),
                span,
                format!("assign an existing field instead of `{field}`"),
            ));
        }
        self.assign_local(root_name, root, span)
    }

    fn collect_place_segments(
        &mut self,
        expr: &Expr,
        segments: &mut Vec<PlaceSegment>,
    ) -> Result<(), EvalError> {
        match expr.kind() {
            ExprKind::Ident(_) => Ok(()),
            ExprKind::FieldAccess { object, field } => {
                self.collect_place_segments(object, segments)?;
                segments.push(PlaceSegment::Field(field.clone()));
                Ok(())
            }
            ExprKind::IndexAccess { object, index } => {
                self.collect_place_segments(object, segments)?;
                let index_value = self.eval_expr(index)?;
                let index = self.value_index(index_value, index.span)?;
                segments.push(PlaceSegment::Index(index));
                Ok(())
            }
            _ => Err(self.error(
                FORBIDDEN_EFFECT,
                "comptime mutation requires a local record or array place",
                expr.span,
                "mutate a value rooted in a mutable local binding",
            )),
        }
    }

    fn lookup(&mut self, name: &str, span: Span) -> Result<ConstValue, EvalError> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Ok(value.value.clone());
            }
        }
        if let Some(value) = self.global_values.get(name) {
            return Ok(value.clone());
        }
        let global = *self.globals.get(name).ok_or_else(|| {
            self.error(
                NON_CONSTANT,
                format!("`{name}` is not available as a compile-time constant"),
                span,
                format!("declare `{name}` as an immutable compile-time constant"),
            )
        })?;
        if self.resolving_globals.iter().any(|current| current == name) {
            return Err(self.error(
                CYCLE,
                format!("compile-time constant `{name}` participates in an initialization cycle"),
                span,
                format!("break the initialization cycle that reaches `{name}`"),
            ));
        }
        self.resolving_globals.push(name.to_string());
        let result = self
            .eval_expr(global.value)
            .and_then(|value| self.coerce(value, global.ty, global.declaration_span));
        self.resolving_globals.pop();
        let value = result?;
        self.global_values.insert(name.to_string(), value.clone());
        Ok(value)
    }

    fn coerce(&self, value: ConstValue, ty: &Type, span: Span) -> Result<ConstValue, EvalError> {
        match (value, ty) {
            (value, Type::Infer) => Ok(value),
            (ConstValue::Int { value, .. }, ty) if is_signed_integer(ty) => {
                self.checked_int(value, ty.clone(), span)
            }
            (ConstValue::UInt { value, .. }, ty) if is_unsigned_integer(ty) => {
                self.checked_uint(value, ty.clone(), span)
            }
            (ConstValue::Int { value, .. }, ty) if is_unsigned_integer(ty) && value >= 0 => {
                self.checked_uint(value as u128, ty.clone(), span)
            }
            (ConstValue::UInt { value, .. }, ty) if is_signed_integer(ty) => {
                let value =
                    i128::try_from(value).map_err(|_| self.integer_error(&BinOp::Add, span))?;
                self.checked_int(value, ty.clone(), span)
            }
            (ConstValue::Float { value, .. }, Type::F32) => {
                let value = (value as f32) as f64;
                Ok(ConstValue::Float {
                    value,
                    text: value.to_string(),
                    ty: Type::F32,
                })
            }
            (ConstValue::Float { value, text, .. }, Type::F64) => Ok(ConstValue::Float {
                value,
                text,
                ty: Type::F64,
            }),
            (ConstValue::Float { value, text, .. }, Type::F128) => Ok(ConstValue::Float {
                value,
                text,
                ty: Type::F128,
            }),
            (ConstValue::Decimal(value), Type::Decimal) => Ok(ConstValue::Decimal(value)),
            (ConstValue::Bool(value), Type::Bool) => Ok(ConstValue::Bool(value)),
            (ConstValue::Char(value), Type::Char) => Ok(ConstValue::Char(value)),
            (ConstValue::String(value), Type::Str) => Ok(ConstValue::String(value)),
            (ConstValue::Char(value), Type::Str) => Ok(ConstValue::String(value.to_string())),
            (ConstValue::Void, Type::Void) => Ok(ConstValue::Void),
            (ConstValue::Array(values), Type::Array(element)) => Ok(ConstValue::Array(
                values
                    .into_iter()
                    .map(|value| self.coerce(value, element, span))
                    .collect::<Result<Vec<_>, _>>()?,
            )),
            (ConstValue::Array(values) | ConstValue::Tuple(values), Type::Tuple(types)) => {
                if values.len() != types.len() {
                    return Err(self.error(
                        UNSUPPORTED_VALUE,
                        format!(
                            "comptime tuple has {} elements but `{ty}` requires {}",
                            values.len(),
                            types.len()
                        ),
                        span,
                        format!("produce exactly {} tuple elements", types.len()),
                    ));
                }
                Ok(ConstValue::Tuple(
                    values
                        .into_iter()
                        .zip(types)
                        .map(|(value, ty)| self.coerce(value, ty, span))
                        .collect::<Result<Vec<_>, _>>()?,
                ))
            }
            (ConstValue::Record(fields), Type::Object(expected)) => Ok(ConstValue::Record(
                self.coerce_record(fields, expected, span)?,
            )),
            (
                ConstValue::Enum {
                    enum_name,
                    member,
                    value,
                },
                Type::Named(expected),
            ) if &enum_name == expected => Ok(ConstValue::Enum {
                enum_name,
                member,
                value,
            }),
            (ConstValue::Record(fields), Type::Named(name))
                if self.interfaces.contains_key(name.as_str()) =>
            {
                Ok(ConstValue::Record(self.coerce_record(
                    fields,
                    self.interfaces[name.as_str()],
                    span,
                )?))
            }
            (value, Type::Readonly(inner)) => self.coerce(value, inner, span),
            (_, ty) => Err(self.error(
                UNSUPPORTED_VALUE,
                format!("comptime value cannot be materialized as `{ty}`"),
                span,
                format!("return a scalar value assignable to `{ty}`"),
            )),
        }
    }

    fn coerce_record(
        &self,
        mut values: BTreeMap<String, ConstValue>,
        fields: &[Field],
        span: Span,
    ) -> Result<BTreeMap<String, ConstValue>, EvalError> {
        for field in fields {
            let Some(value) = values.remove(&field.name) else {
                if field.optional {
                    continue;
                }
                return Err(self.error(
                    UNSUPPORTED_VALUE,
                    format!("comptime record is missing required field `{}`", field.name),
                    span,
                    format!("provide compile-time field `{}`", field.name),
                ));
            };
            let ty = if field.optional {
                match &field.ty {
                    Type::Optional(inner) => inner.as_ref(),
                    ty => ty,
                }
            } else {
                &field.ty
            };
            values.insert(field.name.clone(), self.coerce(value, ty, span)?);
        }
        Ok(values)
    }

    fn checked_int(&self, value: i128, ty: Type, span: Span) -> Result<ConstValue, EvalError> {
        let (min, max) = signed_bounds(&ty).ok_or_else(|| self.integer_error(&BinOp::Add, span))?;
        if value < min || value > max {
            return Err(self.error(
                UNSUPPORTED_VALUE,
                format!("integer comptime result `{value}` does not fit in `{ty}`"),
                span,
                format!("keep the result between `{min}` and `{max}`"),
            ));
        }
        Ok(ConstValue::Int { value, ty })
    }

    fn checked_uint(&self, value: u128, ty: Type, span: Span) -> Result<ConstValue, EvalError> {
        let max = unsigned_max(&ty).ok_or_else(|| self.integer_error(&BinOp::Add, span))?;
        if value > max {
            return Err(self.error(
                UNSUPPORTED_VALUE,
                format!("integer comptime result `{value}` does not fit in `{ty}`"),
                span,
                format!("keep the result between `0` and `{max}`"),
            ));
        }
        Ok(ConstValue::UInt { value, ty })
    }

    fn ensure_materializable(&self, value: &ConstValue, span: Span) -> Result<(), EvalError> {
        if value.element_count() > self.budget.limits.value_elements {
            return Err(self.error(
                VALUE_LIMIT,
                format!(
                    "comptime value exceeded the limit of {} structural elements",
                    self.budget.limits.value_elements
                ),
                span,
                "produce a smaller compile-time value",
            ));
        }
        if value.byte_size() > self.budget.limits.value_bytes {
            return Err(self.error(
                VALUE_LIMIT,
                format!(
                    "comptime value exceeded the limit of {} bytes",
                    self.budget.limits.value_bytes
                ),
                span,
                "produce a smaller compile-time value",
            ));
        }
        Ok(())
    }

    fn check_value_limits(&mut self, value: &ConstValue, span: Span) -> Result<(), EvalError> {
        self.ensure_materializable(value, span)?;
        let bytes = value.byte_size();
        if !self.budget.reserve_temporary(bytes) {
            return Err(self.error(
                VALUE_LIMIT,
                format!(
                    "comptime temporary values exceeded the cumulative limit of {} bytes",
                    self.budget.limits.value_bytes
                ),
                span,
                "reduce intermediate allocations or split the computation into smaller comptime expressions",
            ));
        }
        Ok(())
    }

    fn step(&mut self, span: Span) -> Result<(), EvalError> {
        if self.budget.consume_step() {
            Ok(())
        } else {
            Err(self.error(
                STEP_LIMIT,
                format!(
                    "comptime evaluation exceeded the limit of {} steps",
                    self.budget.limits.steps
                ),
                span,
                "simplify the computation or split it into smaller comptime expressions",
            ))
        }
    }

    fn error(
        &self,
        code: &'static str,
        message: impl Into<String>,
        span: Span,
        help: impl Into<String>,
    ) -> EvalError {
        EvalError {
            code,
            message: message.into(),
            span,
            frames: self.call_stack.clone(),
            help: vec![help.into()],
        }
    }

    fn division_by_zero(&self, span: Span) -> EvalError {
        self.error(
            UNSUPPORTED_VALUE,
            "division by zero during comptime evaluation",
            span,
            "make the divisor non-zero",
        )
    }

    fn integer_error(&self, op: &BinOp, span: Span) -> EvalError {
        let message = if matches!(op, BinOp::Pow) {
            "integer exponent is negative or the comptime result overflowed"
        } else {
            "integer overflow during comptime evaluation"
        };
        self.error(
            UNSUPPORTED_VALUE,
            message,
            span,
            "keep the operation within the bounds of its integer type",
        )
    }

    fn decimal_error(&self, error: DecimalError, span: Span) -> EvalError {
        let message = match error {
            DecimalError::DivisionByZero => "division by zero during decimal comptime evaluation",
            DecimalError::InvalidExponent => {
                "decimal exponent is invalid during comptime evaluation"
            }
            DecimalError::OutOfRange => "decimal comptime result is out of range",
            DecimalError::FractionalComponent => {
                "decimal comptime result has an unsupported fractional component"
            }
        };
        self.error(
            UNSUPPORTED_VALUE,
            message,
            span,
            "keep the decimal operation within the documented decimal range",
        )
    }

    fn unsupported_operator(&self, op: &BinOp, span: Span) -> EvalError {
        self.error(
            UNSUPPORTED_VALUE,
            format!("operator `{op:?}` is not supported for these comptime operands"),
            span,
            "use an operator supported by the operand type",
        )
    }
}

trait ConstValueExt {
    fn into_bool(self, evaluator: &Evaluator<'_>, span: Span) -> Result<bool, EvalError>;
    fn plain_text(self, evaluator: &Evaluator<'_>, span: Span) -> Result<String, EvalError>;
}

impl ConstValueExt for ConstValue {
    fn into_bool(self, evaluator: &Evaluator<'_>, span: Span) -> Result<bool, EvalError> {
        match self {
            ConstValue::Bool(value) => Ok(value),
            _ => Err(evaluator.error(
                UNSUPPORTED_VALUE,
                "comptime condition must be boolean",
                span,
                "use a boolean condition",
            )),
        }
    }

    fn plain_text(self, evaluator: &Evaluator<'_>, span: Span) -> Result<String, EvalError> {
        match self {
            ConstValue::Bool(value) => Ok(value.to_string()),
            ConstValue::Int { value, .. } => Ok(value.to_string()),
            ConstValue::UInt { value, .. } => Ok(value.to_string()),
            ConstValue::Float { text, .. } => Ok(text),
            ConstValue::Decimal(value) => Ok(decimal_literal_text(value)),
            ConstValue::Char(value) => Ok(value.to_string()),
            ConstValue::String(value) => Ok(value),
            _ => Err(evaluator.error(
                UNSUPPORTED_VALUE,
                "value cannot be interpolated during scalar comptime evaluation",
                span,
                "interpolate a scalar value",
            )),
        }
    }
}

pub fn evaluate(program: &Program, facts: &ComptimeFacts) -> CtfeResult {
    evaluate_with_limits(program, facts, EvaluationLimits::default())
}

pub(crate) fn evaluate_with_limits(
    program: &Program,
    facts: &ComptimeFacts,
    limits: EvaluationLimits,
) -> CtfeResult {
    let mut wrappers = Vec::new();
    collect_program_wrappers(program, &mut wrappers);
    let blocked = facts
        .diagnostics
        .iter()
        .flat_map(|diagnostic| diagnostic.related.iter())
        .filter(|related| related.message.contains("required by this wrapper"))
        .map(|related| related.span)
        .collect::<HashSet<_>>();
    let mut result = CtfeResult::default();
    for (wrapper_span, inner) in wrappers {
        if blocked.contains(&wrapper_span) {
            continue;
        }
        let mut evaluator = Evaluator::new(program, facts, limits);
        match evaluator.eval_expr(inner) {
            Ok(value) => result.evaluations.push(ComptimeEvaluation {
                expression_span: wrapper_span,
                value,
            }),
            Err(error) => result.diagnostics.push(error.diagnostic(wrapper_span)),
        }
    }
    result
        .diagnostics
        .sort_by_key(|diagnostic| (diagnostic.span.start, diagnostic.span.end, diagnostic.code));
    result
}

pub fn apply_evaluations(program: &mut Program, evaluations: &[ComptimeEvaluation]) {
    let values = evaluations
        .iter()
        .map(|evaluation| (evaluation.expression_span, &evaluation.value))
        .collect::<HashMap<_, _>>();
    for item in &mut program.items {
        rewrite_item(item, &values);
    }
}

fn collect_program_wrappers<'a>(program: &'a Program, wrappers: &mut Vec<(Span, &'a Expr)>) {
    for item in &program.items {
        match item {
            Item::FnDecl {
                is_comptime: false,
                body,
                ..
            } => collect_stmt_wrappers(body, wrappers),
            Item::FnDecl { .. } => {}
            Item::Stmt(statement) => collect_one_stmt_wrapper(statement, wrappers),
            Item::EnumDecl { members, .. } => {
                for member in members {
                    if let Some(value) = &member.value {
                        collect_expr_wrappers(value, wrappers);
                    }
                }
            }
            _ => {}
        }
    }
}

fn collect_stmt_wrappers<'a>(statements: &'a [Stmt], wrappers: &mut Vec<(Span, &'a Expr)>) {
    for statement in statements {
        collect_one_stmt_wrapper(statement, wrappers);
    }
}

fn collect_one_stmt_wrapper<'a>(statement: &'a Stmt, wrappers: &mut Vec<(Span, &'a Expr)>) {
    visit_stmt_expressions(statement, &mut |expr| collect_expr_wrappers(expr, wrappers));
}

fn collect_expr_wrappers<'a>(expr: &'a Expr, wrappers: &mut Vec<(Span, &'a Expr)>) {
    if let ExprKind::Comptime(inner) = expr.kind() {
        wrappers.push((expr.span, inner));
        return;
    }
    visit_expr_children(expr, &mut |child| collect_expr_wrappers(child, wrappers));
    match expr.kind() {
        ExprKind::If {
            then_block,
            else_block,
            ..
        } => {
            collect_stmt_wrappers(then_block, wrappers);
            if let Some(block) = else_block {
                collect_stmt_wrappers(block, wrappers);
            }
        }
        ExprKind::ArrowFn { body, .. } => collect_stmt_wrappers(body, wrappers),
        ExprKind::Loop { body } => collect_stmt_wrappers(body, wrappers),
        _ => {}
    }
}

fn rewrite_item(item: &mut Item, values: &HashMap<Span, &ConstValue>) {
    match item {
        Item::FnDecl { body, params, .. } => {
            for param in params {
                if let Some(value) = &mut param.default_value {
                    rewrite_expr(value, values);
                }
            }
            rewrite_statements(body, values);
        }
        Item::Stmt(statement) => rewrite_statement(statement, values),
        Item::EnumDecl { members, .. } => {
            for member in members {
                if let Some(value) = &mut member.value {
                    rewrite_expr(value, values);
                }
            }
        }
        Item::BehaviorInterfaceDecl { methods, .. } => {
            for method in methods {
                rewrite_statements(&mut method.body, values);
            }
        }
        Item::InterfaceDecl { .. } | Item::TypeAlias { .. } => {}
    }
}

fn rewrite_statements(statements: &mut [Stmt], values: &HashMap<Span, &ConstValue>) {
    for statement in statements {
        rewrite_statement(statement, values);
    }
}

fn rewrite_statement(statement: &mut Stmt, values: &HashMap<Span, &ConstValue>) {
    match &mut statement.kind {
        StmtKind::VarDecl { value, .. } => {
            if let Some(value) = value {
                rewrite_expr(value, values);
            }
        }
        StmtKind::ArrayDestructure { value, .. }
        | StmtKind::ObjectDestructure { value, .. }
        | StmtKind::Assign { value, .. }
        | StmtKind::Throw(value)
        | StmtKind::Expr(value) => rewrite_expr(value, values),
        StmtKind::IndexAssign { index, value, .. } => {
            rewrite_expr(index, values);
            rewrite_expr(value, values);
        }
        StmtKind::FieldAssign { object, value, .. } => {
            rewrite_expr(object, values);
            rewrite_expr(value, values);
        }
        StmtKind::Return(value) => {
            if let Some(value) = value {
                rewrite_expr(value, values);
            }
        }
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            rewrite_statements(try_body, values);
            rewrite_statements(catch_body, values);
            if let Some(body) = finally_body {
                rewrite_statements(body, values);
            }
        }
        StmtKind::Defer { body, .. } => rewrite_statements(body, values),
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            rewrite_expr(cond, values);
            rewrite_statements(body, values);
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            rewrite_statement(init, values);
            rewrite_expr(cond, values);
            rewrite_statement(update, values);
            rewrite_statements(body, values);
        }
        StmtKind::ForOf { iterable, body, .. } => {
            rewrite_expr(iterable, values);
            rewrite_statements(body, values);
        }
        StmtKind::ForIn { object, body, .. } => {
            rewrite_expr(object, values);
            rewrite_statements(body, values);
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            rewrite_expr(expr, values);
            for case in cases {
                rewrite_expr(&mut case.value, values);
                rewrite_statements(&mut case.body, values);
            }
            if let Some(body) = default {
                rewrite_statements(body, values);
            }
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            rewrite_expr(expr, values);
            for arm in arms {
                if let Some(guard) = &mut arm.guard {
                    rewrite_expr(guard, values);
                }
                rewrite_statements(&mut arm.body, values);
            }
            if let Some(body) = default {
                rewrite_statements(body, values);
            }
        }
        StmtKind::Break(value) => {
            if let Some(value) = value {
                rewrite_expr(value, values);
            }
        }
        StmtKind::Continue => {}
    }
}

fn rewrite_expr(expr: &mut Expr, values: &HashMap<Span, &ConstValue>) {
    if matches!(expr.kind(), ExprKind::Comptime(_))
        && let Some(value) = values.get(&expr.span)
    {
        *expr = value.materialized_expr(expr.span);
        return;
    }
    visit_expr_children_mut(expr, &mut |child| rewrite_expr(child, values));
    if let ExprKind::ArrowFn { body, params, .. } = &mut expr.kind {
        for param in params {
            if let Some(value) = &mut param.default_value {
                rewrite_expr(value, values);
            }
        }
        rewrite_statements(body, values);
    }
    if let ExprKind::If {
        then_block,
        else_block,
        ..
    } = &mut expr.kind
    {
        rewrite_statements(then_block, values);
        if let Some(block) = else_block {
            rewrite_statements(block, values);
        }
    }
    if let ExprKind::Loop { body } = &mut expr.kind {
        rewrite_statements(body, values);
    }
}

fn visit_expr_children<'a>(expr: &'a Expr, visitor: &mut impl FnMut(&'a Expr)) {
    match expr.kind() {
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            if let Some(initial) = initial {
                visitor(initial);
            }
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let TemplatePart::Expr(value, _) = part {
                    visitor(value);
                }
            }
        }
        ExprKind::ObjectLit(fields) => {
            for (_, value) in fields {
                visitor(value);
            }
        }
        ExprKind::ObjectUpdate { base, fields } => {
            visitor(base);
            for (_, value) in fields {
                visitor(value);
            }
        }
        ExprKind::ArrayLit(values) | ExprKind::ChainedComparison { terms: values, .. } => {
            for value in values {
                visitor(value);
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            visitor(value);
            visitor(count);
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            visitor(object);
        }
        ExprKind::IndexAccess { object, index }
        | ExprKind::BinOp {
            left: object,
            right: index,
            ..
        }
        | ExprKind::Pipeline {
            left: object,
            right: index,
        }
        | ExprKind::In {
            key: object,
            object: index,
        }
        | ExprKind::Range {
            start: object,
            end: index,
            ..
        } => {
            visitor(object);
            visitor(index);
        }
        ExprKind::Not(value)
        | ExprKind::Neg(value)
        | ExprKind::BitNot(value)
        | ExprKind::PrefixIncr(value)
        | ExprKind::PrefixDecr(value)
        | ExprKind::PostfixIncr(value)
        | ExprKind::PostfixDecr(value)
        | ExprKind::Unwrap(value)
        | ExprKind::Typeof(value)
        | ExprKind::Await(value)
        | ExprKind::Try(value)
        | ExprKind::Spread(value)
        | ExprKind::ConstAssert(value)
        | ExprKind::Comptime(value) => visitor(value),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            visitor(cond);
            visitor(then_expr);
            visitor(else_expr);
        }
        ExprKind::TypeAssert { value, .. } => visitor(value),
        ExprKind::Is { expr, .. } => visitor(expr),
        ExprKind::OptionalMethodCall { object, args, .. }
        | ExprKind::MethodCall { object, args, .. } => {
            visitor(object);
            for arg in args {
                visitor(arg);
            }
        }
        ExprKind::FnCall { args, .. } => {
            for arg in args {
                visitor(arg);
            }
        }
        ExprKind::CallValue { callee, args, .. } => {
            visitor(callee);
            for arg in args {
                visitor(arg);
            }
        }
        ExprKind::If { cond, .. } => visitor(cond),
        ExprKind::Loop { .. }
        | ExprKind::ArrowFn { .. }
        | ExprKind::StrLit(_)
        | ExprKind::RegexLit { .. }
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::DecimalLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::None
        | ExprKind::Ident(_)
        | ExprKind::Topic
        | ExprKind::Borrow { .. } => {}
    }
}

pub(crate) fn visit_expr_children_mut(expr: &mut Expr, visitor: &mut impl FnMut(&mut Expr)) {
    match &mut expr.kind {
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            if let Some(initial) = initial {
                visitor(initial);
            }
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let TemplatePart::Expr(value, _) = part {
                    visitor(value);
                }
            }
        }
        ExprKind::ObjectLit(fields) => {
            for (_, value) in fields {
                visitor(value);
            }
        }
        ExprKind::ObjectUpdate { base, fields } => {
            visitor(base);
            for (_, value) in fields {
                visitor(value);
            }
        }
        ExprKind::ArrayLit(values) | ExprKind::ChainedComparison { terms: values, .. } => {
            for value in values {
                visitor(value);
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            visitor(value);
            visitor(count);
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            visitor(object);
        }
        ExprKind::IndexAccess { object, index }
        | ExprKind::BinOp {
            left: object,
            right: index,
            ..
        }
        | ExprKind::Pipeline {
            left: object,
            right: index,
        }
        | ExprKind::In {
            key: object,
            object: index,
        }
        | ExprKind::Range {
            start: object,
            end: index,
            ..
        } => {
            visitor(object);
            visitor(index);
        }
        ExprKind::Not(value)
        | ExprKind::Neg(value)
        | ExprKind::BitNot(value)
        | ExprKind::PrefixIncr(value)
        | ExprKind::PrefixDecr(value)
        | ExprKind::PostfixIncr(value)
        | ExprKind::PostfixDecr(value)
        | ExprKind::Unwrap(value)
        | ExprKind::Typeof(value)
        | ExprKind::Await(value)
        | ExprKind::Try(value)
        | ExprKind::Spread(value)
        | ExprKind::ConstAssert(value)
        | ExprKind::Comptime(value) => visitor(value),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            visitor(cond);
            visitor(then_expr);
            visitor(else_expr);
        }
        ExprKind::TypeAssert { value, .. } => visitor(value),
        ExprKind::Is { expr, .. } => visitor(expr),
        ExprKind::OptionalMethodCall { object, args, .. }
        | ExprKind::MethodCall { object, args, .. } => {
            visitor(object);
            for arg in args {
                visitor(arg);
            }
        }
        ExprKind::FnCall { args, .. } => {
            for arg in args {
                visitor(arg);
            }
        }
        ExprKind::CallValue { callee, args, .. } => {
            visitor(callee);
            for arg in args {
                visitor(arg);
            }
        }
        ExprKind::If { cond, .. } => visitor(cond),
        ExprKind::Loop { .. }
        | ExprKind::ArrowFn { .. }
        | ExprKind::StrLit(_)
        | ExprKind::RegexLit { .. }
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::DecimalLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::None
        | ExprKind::Ident(_)
        | ExprKind::Topic
        | ExprKind::Borrow { .. } => {}
    }
}

fn visit_stmt_expressions<'a>(statement: &'a Stmt, visitor: &mut impl FnMut(&'a Expr)) {
    match statement.kind() {
        StmtKind::VarDecl { value, .. } => {
            if let Some(value) = value {
                visitor(value);
            }
        }
        StmtKind::ArrayDestructure { value, .. }
        | StmtKind::ObjectDestructure { value, .. }
        | StmtKind::Assign { value, .. }
        | StmtKind::Throw(value)
        | StmtKind::Expr(value) => visitor(value),
        StmtKind::IndexAssign { index, value, .. } => {
            visitor(index);
            visitor(value);
        }
        StmtKind::FieldAssign { object, value, .. } => {
            visitor(object);
            visitor(value);
        }
        StmtKind::Return(value) => {
            if let Some(value) = value {
                visitor(value);
            }
        }
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            for statement in try_body.iter().chain(catch_body) {
                visit_stmt_expressions(statement, visitor);
            }
            if let Some(body) = finally_body {
                for statement in body {
                    visit_stmt_expressions(statement, visitor);
                }
            }
        }
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            visitor(cond);
            for statement in body {
                visit_stmt_expressions(statement, visitor);
            }
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            visit_stmt_expressions(init, visitor);
            visitor(cond);
            visit_stmt_expressions(update, visitor);
            for statement in body {
                visit_stmt_expressions(statement, visitor);
            }
        }
        StmtKind::ForOf { iterable, body, .. } => {
            visitor(iterable);
            for statement in body {
                visit_stmt_expressions(statement, visitor);
            }
        }
        StmtKind::ForIn { object, body, .. } => {
            visitor(object);
            for statement in body {
                visit_stmt_expressions(statement, visitor);
            }
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            visitor(expr);
            for case in cases {
                visitor(&case.value);
                for statement in &case.body {
                    visit_stmt_expressions(statement, visitor);
                }
            }
            if let Some(body) = default {
                for statement in body {
                    visit_stmt_expressions(statement, visitor);
                }
            }
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            visitor(expr);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    visitor(guard);
                }
                for statement in &arm.body {
                    visit_stmt_expressions(statement, visitor);
                }
            }
            if let Some(body) = default {
                for statement in body {
                    visit_stmt_expressions(statement, visitor);
                }
            }
        }
        StmtKind::Break(value) => {
            if let Some(value) = value {
                visitor(value);
            }
        }
        StmtKind::Defer { body, .. } => {
            for statement in body {
                visit_stmt_expressions(statement, visitor);
            }
        }
        StmtKind::Continue => {}
    }
}

fn assignment_binop(op: &AssignOp) -> Option<BinOp> {
    Some(match op {
        AssignOp::AddAssign => BinOp::Add,
        AssignOp::SubAssign => BinOp::Sub,
        AssignOp::MulAssign => BinOp::Mul,
        AssignOp::DivAssign => BinOp::Div,
        AssignOp::ModAssign => BinOp::Mod,
        AssignOp::PowAssign => BinOp::Pow,
        AssignOp::AndAssign => BinOp::And,
        AssignOp::OrAssign => BinOp::Or,
        AssignOp::BitAndAssign => BinOp::BitAnd,
        AssignOp::BitOrAssign => BinOp::BitOr,
        AssignOp::BitXorAssign => BinOp::BitXor,
        AssignOp::ShlAssign => BinOp::Shl,
        AssignOp::ShrAssign => BinOp::Shr,
        AssignOp::UShrAssign => BinOp::UShr,
        AssignOp::Assign | AssignOp::NullCoalesceAssign => return None,
    })
}

fn enum_payload(value: &ConstValue) -> &ConstValue {
    match value {
        ConstValue::Enum { value, .. } => enum_payload(value),
        value => value,
    }
}

fn value_at_path<'a>(value: &'a ConstValue, path: &[PlaceSegment]) -> Option<&'a ConstValue> {
    let Some((first, rest)) = path.split_first() else {
        return Some(value);
    };
    let next = match (value, first) {
        (ConstValue::Record(fields), PlaceSegment::Field(field)) => fields.get(field)?,
        (ConstValue::Array(values) | ConstValue::Tuple(values), PlaceSegment::Index(index)) => {
            values.get(*index)?
        }
        _ => return None,
    };
    value_at_path(next, rest)
}

fn set_value_at_path(value: &mut ConstValue, path: &[PlaceSegment], next: ConstValue) -> bool {
    let Some((first, rest)) = path.split_first() else {
        *value = next;
        return true;
    };
    let child = match (value, first) {
        (ConstValue::Record(fields), PlaceSegment::Field(field)) => fields.get_mut(field),
        (ConstValue::Array(values) | ConstValue::Tuple(values), PlaceSegment::Index(index)) => {
            values.get_mut(*index)
        }
        _ => None,
    };
    child.is_some_and(|child| set_value_at_path(child, rest, next))
}

fn is_signed_integer(ty: &Type) -> bool {
    matches!(
        ty,
        Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::Isize
    )
}

fn is_unsigned_integer(ty: &Type) -> bool {
    matches!(
        ty,
        Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::Usize
    )
}

fn integer_bits(ty: &Type) -> Option<u32> {
    match ty {
        Type::I8 | Type::U8 => Some(8),
        Type::I16 | Type::U16 => Some(16),
        Type::I32 | Type::U32 => Some(32),
        Type::I64 | Type::U64 => Some(64),
        Type::I128 | Type::U128 => Some(128),
        Type::Isize | Type::Usize => Some(usize::BITS),
        _ => None,
    }
}

fn signed_bounds(ty: &Type) -> Option<(i128, i128)> {
    let bits = integer_bits(ty)?;
    if !is_signed_integer(ty) {
        return None;
    }
    if bits == 128 {
        Some((i128::MIN, i128::MAX))
    } else {
        let max = (1_i128 << (bits - 1)) - 1;
        Some((-max - 1, max))
    }
}

fn unsigned_max(ty: &Type) -> Option<u128> {
    let bits = integer_bits(ty)?;
    if !is_unsigned_integer(ty) {
        return None;
    }
    Some(if bits == 128 {
        u128::MAX
    } else {
        (1_u128 << bits) - 1
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comptime;
    use crate::parser::Parser;

    fn parsed(source: &str) -> Program {
        Parser::from_source(source)
            .unwrap()
            .parse_program()
            .unwrap()
    }

    fn evaluated(source: &str) -> CtfeResult {
        let program = parsed(source);
        let facts = comptime::analyze(&program, source);
        assert!(
            facts.diagnostics.is_empty(),
            "phase 2: {:#?}",
            facts.diagnostics
        );
        evaluate(&program, &facts)
    }

    #[test]
    fn evaluates_scalar_bindings_conditionals_calls_and_decimal_arithmetic() {
        let source = "const base: i32 = 20; comptime function double(value: i32): i32 { const factor: i32 = 2; if (value > 0) { return value * factor; } return 0; } comptime const triple = (value: i32): i32 => value * 3; const integer: i32 = comptime(double(base + 1)); const arrowValue: i32 = comptime(triple(14)); const decimalValue: decimal = comptime(1.25d + 2.75d);";
        let result = evaluated(source);
        assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
        assert_eq!(result.evaluations.len(), 3);
        assert_eq!(
            result.evaluations[0].value,
            ConstValue::Int {
                value: 42,
                ty: Type::I32
            }
        );
        assert_eq!(
            result.evaluations[1].value,
            ConstValue::Int {
                value: 42,
                ty: Type::I32
            }
        );
        assert_eq!(
            result.evaluations[2].value,
            ConstValue::Decimal(Decimal::from_integer(4))
        );
    }

    #[test]
    fn evaluates_terminating_recursion_within_the_deterministic_budget() {
        let source = "comptime function factorial(value: i32): i32 { if (value <= 1) { return 1; } return value * factorial(value - 1); } const result: i32 = comptime(factorial(5));";
        let result = evaluated(source);
        assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
        assert_eq!(
            result.evaluations[0].value,
            ConstValue::Int {
                value: 120,
                ty: Type::I32
            }
        );
    }

    #[test]
    fn projection_replaces_wrapper_with_a_literal_and_preserves_its_span() {
        let source = "comptime function double(value: i32): i32 { return value * 2; } const answer: i32 = comptime(double(21));";
        let mut program = parsed(source);
        let facts = comptime::analyze(&program, source);
        let result = evaluate(&program, &facts);
        assert!(result.diagnostics.is_empty());
        let wrapper_span = result.evaluations[0].expression_span;
        apply_evaluations(&mut program, &result.evaluations);
        let Item::Stmt(Stmt {
            kind: StmtKind::VarDecl {
                value: Some(value), ..
            },
            ..
        }) = &program.items[1]
        else {
            panic!("expected projected binding");
        };
        assert_eq!(value.span, wrapper_span);
        assert!(matches!(value.kind(), ExprKind::IntLit(42)));
    }

    #[test]
    fn deterministic_step_and_depth_limits_report_call_stacks_and_unwind() {
        let source = "comptime function recurse(value: i32): i32 { return recurse(value + 1); } const answer: i32 = comptime(recurse(0));";
        let program = parsed(source);
        let facts = comptime::analyze(&program, source);
        let result = evaluate_with_limits(
            &program,
            &facts,
            EvaluationLimits {
                steps: 100,
                call_depth: 4,
                value_elements: 32,
                value_bytes: 1024,
            },
        );
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DEPTH_LIMIT)
            .unwrap_or_else(|| panic!("{:#?}", result.diagnostics));
        assert!(diagnostic.message.contains('4'));
        assert!(
            diagnostic
                .related
                .iter()
                .any(|related| related.message.contains("recurse"))
        );

        let mut budget = EvaluationBudget::new(EvaluationLimits {
            steps: 1,
            call_depth: 1,
            value_elements: 1,
            value_bytes: 1,
        });
        assert!(budget.consume_step());
        assert!(!budget.consume_step());
        assert_eq!(budget.steps(), 1);
        assert!(budget.enter_call());
        budget.leave_call();
        assert_eq!(budget.call_depth(), 0);
        assert!(budget.reserve_temporary(1));
        assert_eq!(budget.temporary_bytes(), 1);
    }

    #[test]
    fn scalar_ctfe_matches_the_runtime_operator_contract_for_representative_inputs() {
        let cases = [
            (
                "comptime(6 * 7)",
                ConstValue::Int {
                    value: 42,
                    ty: Type::I32,
                },
            ),
            ("comptime((5 > 3) && (2 < 4))", ConstValue::Bool(true)),
            (
                "comptime(2i64 ** 10i64)",
                ConstValue::Int {
                    value: 1024,
                    ty: Type::I64,
                },
            ),
            (
                "comptime(1.5f32 + 2.25f32)",
                ConstValue::Float {
                    value: 3.75,
                    text: "3.75".to_string(),
                    ty: Type::F32,
                },
            ),
        ];
        for (expression, expected) in cases {
            let source = format!("const result = {expression};");
            let result = evaluated(&source);
            assert_eq!(result.evaluations[0].value, expected, "{expression}");
        }
    }

    #[test]
    fn evaluates_special_floating_point_values_and_is_nan() {
        let result = evaluated(
            "const invalid: boolean = comptime(isNaN(NaN)); const positive: f64 = comptime(Infinity * 2.0);",
        );
        assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
        assert_eq!(result.evaluations[0].value, ConstValue::Bool(true));
        assert!(
            matches!(result.evaluations[1].value, ConstValue::Float { value, ref text, ty: Type::F64 } if value == f64::INFINITY && text == "Infinity")
        );
    }

    #[test]
    fn evaluates_loops_break_continue_ranges_and_switch_fallthrough() {
        let source = "comptime function calculate(limit: i32): i32 { let total: i32 = 0; let index: i32 = 0; while (index < limit) { index++; if (index === 2) { continue; } total += index; if (total > 10) { break; } } do { total += 1; } while (false); for (let current: i32 = 0; current < 2; current++) { total += current; } for (const current of 0..3) { total += current; } switch (limit) { case 5: total += 10; default: total += 1; } return total; } const result: i32 = comptime(calculate(5));";
        let result = evaluated(source);
        assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
        assert_eq!(
            result.evaluations[0].value,
            ConstValue::Int {
                value: 29,
                ty: Type::I32
            }
        );
    }

    #[test]
    fn evaluates_loop_statements_and_break_values() {
        let source = "comptime function loopStatement(): i32 { let attempts: i32 = 0; loop { attempts++; if (attempts > 2) { break; } } return attempts; } comptime function loopValue(): i32 { let total: i32 = 0; for (let i: i32 = 0; i < 3; i++) { total += i; if (total > 5) { break; } } const matched: i32 = loop { break total; }; const named: i32 = loop { if (matched === 0) { break 1; } break matched + 1; }; return named; } const first: i32 = comptime(loopStatement()); const second: i32 = comptime(loopValue());";
        let result = evaluated(source);
        assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
        assert_eq!(result.evaluations.len(), 2);
        assert_eq!(
            result.evaluations[0].value,
            ConstValue::Int {
                value: 3,
                ty: Type::I32
            }
        );
        assert_eq!(
            result.evaluations[1].value,
            ConstValue::Int {
                value: 4,
                ty: Type::I32
            }
        );
    }

    #[test]
    fn evaluates_and_materializes_structured_values_and_local_mutation() {
        let source = "enum Mode { Fast = 7 } interface Config { label: string; values: i32[]; mode: Mode; } comptime function build(): Config { let result: Config = { label: `ready`, values: [1, 2], mode: Mode.Fast }; let values: i32[] = result.values; values[0] = 4; result.values = values; result.label = `ready-${result.values.length}`; return { ...result, values: [...result.values, 9] }; } comptime function pair(): [i32, string] { return [42, \"answer\"]; } const config: Config = comptime(build()); const tuple: [i32, string] = comptime(pair());";
        let mut program = parsed(source);
        let facts = comptime::analyze(&program, source);
        assert!(
            facts.diagnostics.is_empty(),
            "phase 2: {:#?}",
            facts.diagnostics
        );
        let result = evaluate(&program, &facts);
        assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);

        let expected = ConstValue::Record(BTreeMap::from([
            (
                "label".to_string(),
                ConstValue::String("ready-2".to_string()),
            ),
            (
                "mode".to_string(),
                ConstValue::Enum {
                    enum_name: "Mode".to_string(),
                    member: "Fast".to_string(),
                    value: Box::new(ConstValue::Int {
                        value: 7,
                        ty: Type::I32,
                    }),
                },
            ),
            (
                "values".to_string(),
                ConstValue::Array(vec![
                    ConstValue::Int {
                        value: 4,
                        ty: Type::I32,
                    },
                    ConstValue::Int {
                        value: 2,
                        ty: Type::I32,
                    },
                    ConstValue::Int {
                        value: 9,
                        ty: Type::I32,
                    },
                ]),
            ),
        ]));
        assert_eq!(result.evaluations[0].value, expected);
        assert!(matches!(
            result.evaluations[1].value,
            ConstValue::Tuple(ref values) if values.len() == 2
        ));

        apply_evaluations(&mut program, &result.evaluations);
        let Item::Stmt(Stmt {
            kind: StmtKind::VarDecl {
                value: Some(value), ..
            },
            ..
        }) = &program.items[4]
        else {
            panic!("expected projected structured binding");
        };
        let ExprKind::ObjectLit(fields) = value.kind() else {
            panic!("expected a materialized object literal");
        };
        assert!(fields.iter().any(|(name, value)| {
            name == "mode"
                && matches!(
                    value.kind(),
                    ExprKind::FieldAccess { object, field }
                        if field == "Fast" && matches!(object.kind(), ExprKind::Ident(name) if name == "Mode")
                )
        }));
    }

    #[test]
    fn reports_structural_and_cumulative_temporary_value_limits() {
        let source = "const result: i32[] = comptime([1, 2, 3]);";
        let program = parsed(source);
        let facts = comptime::analyze(&program, source);
        assert!(facts.diagnostics.is_empty(), "{:#?}", facts.diagnostics);

        let structural = evaluate_with_limits(
            &program,
            &facts,
            EvaluationLimits {
                steps: 100,
                call_depth: 10,
                value_elements: 2,
                value_bytes: 1024,
            },
        );
        assert!(
            structural
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == VALUE_LIMIT
                    && diagnostic.message.contains("structural elements")
                    && !diagnostic.help.is_empty()
                    && !diagnostic.related.is_empty())
        );

        let cumulative = evaluate_with_limits(
            &program,
            &facts,
            EvaluationLimits {
                steps: 100,
                call_depth: 10,
                value_elements: 10,
                value_bytes: 80,
            },
        );
        assert!(
            cumulative
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == VALUE_LIMIT
                    && diagnostic.message.contains("temporary values"))
        );
    }

    #[test]
    fn rejects_values_without_a_materializable_literal_form() {
        let source = "comptime function bounds(): Range<i32> { return 0..4; } const result = comptime(bounds());";
        let result = evaluated(source);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == UNSUPPORTED_VALUE
                    && diagnostic.message.contains("range"))
        );
    }
}
