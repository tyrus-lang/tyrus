use crate::ast::ExprKind;
use crate::ast::{
    AssignOp, BehaviorInterfaceInfo, BinOp, EnumPayload, Expr, Field, FloatLiteral, FormatSpec,
    Item, MatchPattern, OnFail, OnFailHandler, Param, ParamPattern, Program, ResolvedEnum, Stmt,
    TemplatePart, Type, ValidatorDecorator, contains_outer_topic, desugar_pipeline, resolve_enum,
};
use crate::ast::{DeferKind, StmtKind};
use crate::async_types::{
    owned_copy_array_frame_method, owned_string_array_frame_method, owned_string_frame_method,
    static_string_array_frame_literal,
};
use crate::borrow_checker::closure_capture_names;
use crate::collections::{HashMap, HashSet};
use crate::numeric_types::{
    integer_type_bounds, is_float_type, is_integer_type, is_unsigned_integer_type,
    numeric_constant_type,
};
use crate::span::Span;
use crate::type_resolve::{ObjectProjectionMode, project_object_fields};
use crate::validators::{
    ValidatorDiagnosticSeverity, regex_runtime_pattern_with_context, validate_decorator,
    validate_decorator_set, validate_literal_value, validator_display,
};
use std::cell::RefCell;
use std::rc::Rc;

#[path = "api/api.rs"]
mod api;
#[path = "async_checks/async_checks.rs"]
mod async_checks;
#[path = "calls/calls.rs"]
mod calls;
#[path = "expressions/expressions.rs"]
mod expressions;
#[path = "http/http.rs"]
mod http;
#[path = "methods/methods.rs"]
mod methods;
#[path = "statements/statements.rs"]
mod statements;
#[path = "support/support.rs"]
mod support;
#[path = "validators/validators.rs"]
mod validators;

#[path = "count/count.rs"]
mod count;
#[path = "functional_arrays/functional_arrays.rs"]
mod functional_arrays;

use functional_arrays::is_functional_array_method;
pub(super) use support::*;
pub use support::{
    MAX_ARRAY_REPEAT_COUNT, TupleSequence, const_assertion_type, integer_constant_value,
    is_array_mutating_method, is_mutating_method, tuple_sequence,
};
pub(crate) use support::{
    is_mutex_guard_typed, is_rwlock_read_guard_typed, is_rwlock_write_guard_typed,
};

pub use functional_arrays::is_functional_array_method as is_functional_array_method_surface;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckDiagnostic {
    pub code: &'static str,
    pub severity: CheckSeverity,
    pub message: String,
    pub span: Option<Span>,
    pub subject: Option<String>,
    pub help: Vec<String>,
    pub prefer_declaration: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferredExpression {
    pub ordinal: u32,
    pub ty: String,
}

#[derive(Debug, Clone, Default)]
pub struct CheckResult {
    pub diagnostics: Vec<CheckDiagnostic>,
    pub expressions: Vec<InferredExpression>,
    pub contextual_parameters: Vec<InferredParameter>,
    pub onfail_bindings: Vec<InferredOnFailBinding>,
    pub resolved_symbols: HashMap<String, Type>,
    pub resolved_bindings: Vec<ResolvedBinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedBinding {
    pub name: String,
    pub span: Span,
    pub ty: Type,
}

#[derive(Clone, Copy)]
pub struct ImportedProgramSymbols<'a> {
    pub types: &'a HashMap<String, Type>,
    pub interfaces: &'a HashMap<String, Vec<Field>>,
    pub behaviors: &'a HashMap<String, BehaviorInterfaceInfo>,
    pub type_aliases: &'a HashMap<String, Type>,
    pub enums: &'a HashMap<String, ResolvedEnum>,
    pub throws: &'a HashMap<String, Vec<Type>>,
    pub db_models: &'a HashMap<String, crate::ast::DbModelMetadata>,
    pub validators: &'a HashMap<String, Type>,
    pub reachable_interfaces: &'a HashMap<String, Vec<Field>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InferredParameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InferredOnFailBinding {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
enum ValueType {
    Known(Type),
    IntegerLiteral,
    NegativeIntegerLiteral,
    UnsignedIntegerLiteral,
    FloatLiteral,
    Null,
    Unknown,
    Error,
    Object(Vec<(String, ValueType)>),
    Array(Vec<ValueType>),
    Union(Vec<ValueType>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ObjectBindingInference {
    Unique(String),
    Ambiguous(Vec<String>),
    None,
}

#[derive(Debug, Clone, Default)]
struct HttpRouteContract {
    request_fields: Vec<Field>,
    responses: Vec<(u16, Type)>,
    params_schema: Option<String>,
}

#[derive(Debug, Clone)]
struct FunctionBodyMetadata {
    params: Vec<Param>,
    body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
struct ExtensionMethod {
    owner: String,
    target: Type,
    name: String,
    params: Vec<Param>,
    ret_type: Type,
}

#[derive(Debug, Clone)]
struct StaticProjectionKey {
    name: String,
    span: Span,
}

fn specialize_extension_receiver(params: &[Param], target: &Type) -> Vec<Param> {
    let mut params = params.to_vec();
    if let Some(receiver) = params.first_mut()
        && matches!(receiver.ty, Type::Infer)
    {
        receiver.ty = target.clone();
    }
    params
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HttpReplyFlowPath {
    sent: bool,
    streamed: bool,
    open: bool,
}

#[derive(Debug, Clone)]
struct DiscriminantNarrowing {
    name: String,
    field: String,
    literal: Expr,
    matches_in_then: bool,
}

#[derive(Debug, Clone)]
struct ResultNarrowing {
    name: String,
    success_in_then: bool,
}

#[derive(Debug, Clone, Default)]
struct ValidatorFacts {
    numeric_min: Option<f64>,
    numeric_max: Option<f64>,
    length_min: Option<i64>,
    length_max: Option<i64>,
    allowed_literals: Option<Vec<Expr>>,
    disallowed_literals: Vec<Expr>,
    non_null: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FactCondition {
    AlwaysTrue,
    AlwaysFalse,
}

impl ValidatorFacts {
    fn is_empty(&self) -> bool {
        self.numeric_min.is_none()
            && self.numeric_max.is_none()
            && self.length_min.is_none()
            && self.length_max.is_none()
            && self.allowed_literals.is_none()
            && self.disallowed_literals.is_empty()
            && !self.non_null
    }
}

impl ValueType {
    fn display(&self) -> String {
        match self {
            Self::Known(ty) => ty.to_string(),
            Self::IntegerLiteral => "integer literal".to_string(),
            Self::NegativeIntegerLiteral => "negative integer literal".to_string(),
            Self::UnsignedIntegerLiteral => "unsigned integer literal".to_string(),
            Self::FloatLiteral => "float literal".to_string(),
            Self::Null => "null".to_string(),
            Self::Unknown => "unknown".to_string(),
            Self::Error => "<error>".to_string(),
            Self::Object(_) => "object".to_string(),
            Self::Array(values) => values.first().map_or_else(
                || "never[]".to_string(),
                |value| format!("{}[]", value.display()),
            ),
            Self::Union(variants) => variants
                .iter()
                .rev()
                .map(ValueType::display)
                .collect::<Vec<_>>()
                .join(" | "),
        }
    }

    fn known(ty: &Type) -> Self {
        Self::Known(ty.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SymbolKind {
    Variable,
    Parameter,
    Function,
    Import,
}

#[derive(Debug, Clone)]
struct Symbol {
    ty: ValueType,
    mutable: bool,
    kind: SymbolKind,
    used: bool,
    array_len: Option<usize>,
}

#[derive(Clone, PartialEq)]
enum ControlContext {
    Loop,
    Switch,
    LoopExpr {
        value_type: Rc<RefCell<Option<Type>>>,
    },
    Branch(&'static str),
}

#[derive(Debug, Clone)]
struct FinalizerContext {
    kind: DeferKind,
    keyword_span: Span,
    captures: HashSet<String>,
}

#[derive(Debug, Clone)]
struct FinalizerRegistration {
    kind: DeferKind,
    keyword_span: Span,
    captures: HashSet<String>,
}

pub struct TypeChecker {
    cancellation: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    confined_closures: crate::escape_analysis::ConfinedClosures,
    item_span: Option<Span>,
    span_stack: Vec<Span>,
    scopes: Vec<HashMap<String, Symbol>>,
    constant_integers: Vec<HashMap<String, Option<i128>>>,
    narrowed_paths: Vec<HashSet<String>>,
    declaration_depths: Vec<HashMap<String, usize>>,
    import_targets: HashMap<String, String>,
    source_text: Option<String>,
    db_schema: Option<crate::db::DbSchemaSnapshot>,
    db_schema_error: Option<String>,
    db_backend: Option<crate::db::DatabaseBackend>,
    validate_db_schema: bool,
    db_models: HashMap<String, crate::ast::DbModelMetadata>,
    expression_types_by_span: HashMap<Span, Type>,
    user_validators: HashMap<String, Type>,
    entry_point: bool,
    pending_entry_point_return_check: Option<(bool, Span)>,
    unresolved_namespaces: Vec<String>,
    validator_facts: Vec<HashMap<String, ValidatorFacts>>,
    for_index_hint: Option<Type>,
    diagnostics: Vec<CheckDiagnostic>,
    expected_return: Option<Type>,
    inferred_returns: Vec<Vec<ValueType>>,
    inferred_onfail_results: Vec<Vec<(Type, Span)>>,
    expressions: Vec<InferredExpression>,
    contextual_parameters: Vec<InferredParameter>,
    onfail_bindings: Vec<InferredOnFailBinding>,
    resolved_bindings: Vec<ResolvedBinding>,
    interfaces: HashMap<String, Vec<crate::ast::Field>>,
    data_interfaces: HashSet<String>,
    reachable_only_types: HashSet<String>,
    type_aliases: HashMap<String, Type>,
    generic_defs: crate::type_resolve::GenericDefs,
    type_param_constraints: Vec<HashMap<String, Type>>,
    type_param_names: Vec<HashSet<String>>,
    enums: HashMap<String, ResolvedEnum>,
    fn_param_metadata: HashMap<String, Vec<Param>>,
    fn_body_metadata: HashMap<String, FunctionBodyMetadata>,
    extension_methods: Vec<ExtensionMethod>,
    behavior_targets: HashMap<String, Vec<Type>>,
    fn_throws: HashMap<String, Vec<Type>>,
    async_fns: std::collections::HashSet<String>,
    current_throws: Vec<Vec<Type>>,
    control_stack: Vec<ControlContext>,
    try_catch_type: Vec<Type>,
    awaited_operand: Option<Span>,
    try_body_suspends: Vec<bool>,
    try_catch_suspends: Vec<bool>,
    try_finally_suspends: Vec<bool>,
    error_catch_bindings: Vec<(String, usize)>,
    finalizer_scopes: Vec<Vec<FinalizerRegistration>>,
    finalizer_contexts: Vec<FinalizerContext>,
    branch_depth: usize,
    fn_depth: usize,
    env_fields: Option<Vec<Field>>,
    topic_stack: Vec<ValueType>,
    async_depth: usize,
    async_function_stack: Vec<bool>,
    /// Names of `JoinHandle<T>` locals already consumed by a `.join()` call,
    /// one set per scope (mirrors `scopes`, pushed/popped alongside it).
    /// `JoinHandle<T>` carries no ownership-move tracking of its own, unlike
    /// every other non-`Copy` value, which `borrow_checker.rs` tracks, since
    /// that pass never learns `spawn`'s real return type (see the comment on
    /// its `.join()` handling in `check_method_call`, `ExprKind::MethodCall`),
    /// so this check lives here instead, where `spawn(...)`'s type is
    /// already fully resolved. A name reused here after being marked is
    /// `E3033`: codegen forwards `.join()`'s
    /// single `*mut TyrusPromise` reference rather than retaining a second
    /// one, so a second `.join()`/an `.abort()` afterward is a real
    /// use-after-free, not just a style issue.
    /// The value is the span of the call that consumed the name. An expression
    /// can be checked more than once, an `await` nested inside another
    /// expression is visited both while its enclosing expression is inferred
    /// and while it is checked, so recording *where* a handle was consumed is
    /// what separates the same call seen twice from two different calls.
    joined_handle_names: Vec<HashMap<String, Span>>,
    /// Names of `Promise<T>` bindings already consumed by an `await`, one map
    /// per scope, with the span of the `await` that consumed each.
    ///
    /// A `Promise<T>` is consumed exactly once, the same way a `JoinHandle<T>`
    /// is: `await` takes the runtime's single reference and releases it, so a
    /// second `await` on the same binding is a use-after-free rather than a
    /// second read of a settled value. Without this it compiled and died on
    /// SIGSEGV. The span serves the same purpose it does for handles, see
    /// `is_join_handle_consumed`.
    awaited_promise_names: Vec<HashMap<String, Span>>,
    mutex_guard_names: Vec<HashSet<String>>,
    exhaustive_matches: HashSet<Span>,
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeChecker {
    fn check_entry_point(&mut self, program: &Program) {
        if !self.entry_point {
            return;
        }
        let main = program.items.iter().find_map(|item| match item {
            Item::FnDecl {
                name,
                is_async,
                params,
                ret_type,
                span,
                ..
            } if name == "main" => Some((*is_async, params, ret_type, *span)),
            _ => None,
        });
        let Some((is_async, params, ret_type, span)) = main else {
            self.error(
                "E0601",
                "the entry module must declare `main`".to_string(),
                None,
            );
            if let Some(last) = self.diagnostics.last_mut() {
                last.help = vec![
                    "add `const main = (): void => { ... };`, or `const main = async (): Promise<void> => { ... };` for an async entry point"
                        .to_string(),
                ];
            }
            return;
        };
        if !params.is_empty() {
            self.error_at_with_help(
                "E0580",
                "`main` has an unsupported signature: it takes no parameters; read the command line with `process.argv`".to_string(),
                span,
                vec![
                    "write `const main = (): void => { ... };` or `const main = async (): Promise<void> => { ... };`"
                        .to_string(),
                ],
            );
            return;
        }
        if matches!(ret_type, Type::Infer) {
            self.pending_entry_point_return_check = Some((is_async, span));
            return;
        }
        let returns_void = matches!(ret_type, Type::Void) && !is_async;
        let returns_promise_void = is_async
            && matches!(ret_type, Type::Apply { name, args } if name == "Promise" && args.as_slice() == [Type::Void]);
        if returns_void || returns_promise_void {
            return;
        }
        self.error_at_with_help(
            "E0580",
            "`main` has an unsupported signature: it must return `void`, or `Promise<void>` when declared `async`".to_string(),
            span,
            vec![
                "write `const main = (): void => { ... };` or `const main = async (): Promise<void> => { ... };`"
                    .to_string(),
            ],
        );
    }

    fn check_entry_point_inferred_return(&mut self) {
        let Some((is_async, span)) = self.pending_entry_point_return_check.take() else {
            return;
        };
        let ret = self
            .scopes
            .first()
            .and_then(|scope| match scope.get("main") {
                Some(Symbol {
                    ty: ValueType::Known(Type::Function { ret, .. }),
                    ..
                }) => Some(ret.as_ref().clone()),
                _ => None,
            });
        let returns_void = matches!(ret, Some(Type::Void)) && !is_async;
        let returns_promise_void = is_async
            && matches!(&ret, Some(Type::Apply { name, args }) if name == "Promise" && args.as_slice() == [Type::Void]);
        if returns_void || returns_promise_void {
            return;
        }
        self.error_at_with_help(
            "E0580",
            "`main` has an unsupported signature: it must return `void`, or `Promise<void>` when declared `async`".to_string(),
            span,
            vec![
                "write `const main = (): void => { ... };` or `const main = async (): Promise<void> => { ... };`"
                    .to_string(),
            ],
        );
    }

    fn check_item(&mut self, item: &Item) {
        if self.is_cancelled() {
            return;
        }
        match item {
            Item::InterfaceDecl {
                type_params,
                extends,
                fields,
                span,
                ..
            } => {
                self.push_type_param_constraints(type_params);
                for supertype in extends {
                    self.check_type_name(supertype, *span);
                }
                for field in fields {
                    let anchor = if field.span.is_dummy() {
                        *span
                    } else {
                        field.span
                    };
                    self.check_type_name(&field.ty, anchor);
                }
                self.pop_type_param_constraints();
            }
            Item::BehaviorInterfaceDecl {
                name,
                type_params,
                targets,
                extends,
                methods,
                span,
                ..
            } => {
                self.push_type_param_constraints(type_params);
                for target in targets {
                    self.check_type_name(target, *span);
                }
                for supertype in extends {
                    self.check_type_name(supertype, *span);
                }
                let effective_targets = self
                    .behavior_targets
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| targets.clone());
                for (target_index, target) in effective_targets.iter().enumerate() {
                    for method in methods {
                        let synthetic = Item::FnDecl {
                            name: format!("__tyrus_at_{name}_{}_{target_index}", method.name),
                            is_async: false,
                            is_comptime: false,
                            is_validator: false,
                            type_params: method.type_params.clone(),
                            params: specialize_extension_receiver(&method.params, target),
                            ret_type: method.ret_type.clone(),
                            throws: method.throws.clone(),
                            body: method.body.clone(),
                            span: method.span,
                        };
                        self.check_item(&synthetic);
                    }
                }
                self.pop_type_param_constraints();
            }
            Item::EnumDecl { .. } => {}
            Item::TypeAlias {
                name,
                ty,
                merged_fields: _,
                type_params,
                ..
            } => {
                let resolved_ty = self.resolve_type_query(ty);
                let resolved_ty = if type_params.is_empty() {
                    let mut resolve_diags = Vec::new();
                    let resolved = self.generic_defs.resolve(&resolved_ty, &mut resolve_diags);
                    for diagnostic in resolve_diags {
                        self.error(
                            diagnostic.code,
                            diagnostic.message,
                            diagnostic.subject.as_deref(),
                        );
                    }
                    resolved
                } else {
                    resolved_ty
                };
                self.type_aliases.insert(name.clone(), resolved_ty.clone());
                self.declare(
                    name,
                    Symbol {
                        ty: ValueType::known(&resolved_ty),
                        mutable: false,
                        kind: SymbolKind::Variable,
                        used: false,
                        array_len: None,
                    },
                );
            }
            Item::FnDecl {
                name,
                is_async,
                is_comptime,
                type_params,
                params,
                ret_type,
                throws,
                body,
                is_validator,
                span,
                ..
            } => {
                if *is_validator {
                    self.validate_user_validator_signature(
                        name,
                        *is_async,
                        *is_comptime,
                        type_params,
                        params,
                        ret_type,
                        throws,
                        *span,
                    );
                }
                self.push_type_param_constraints(type_params);
                if name == "main" && !throws.is_empty() {
                    self.error(
                        "E3012",
                        "`main` cannot declare `throws`; handle every reachable error before the application boundary".to_string(),
                        Some("main"),
                    );
                }
                let mut effects = throws.clone();
                if params.iter().any(|param| !param.decorators.is_empty())
                    && !effects
                        .iter()
                        .any(|ty| ty == &Type::Named("ValidationError".to_string()))
                {
                    effects.push(Type::Named("ValidationError".to_string()));
                }
                self.current_throws.push(effects);
                let enclosing_tries = self.take_enclosing_try_handlers();
                if matches!(ret_type, Type::Infer) {
                    self.inferred_returns.push(Vec::new());
                    self.inferred_onfail_results.push(Vec::new());
                    self.check_function(params, ret_type, body, *is_async);
                    let inferred = self.inferred_returns.pop().unwrap_or_default();
                    let onfail_results = self.inferred_onfail_results.pop().unwrap_or_default();
                    let inferred_ret = infer_callback_return(&inferred).unwrap_or(Type::Void);
                    self.validate_inferred_onfail_results(&onfail_results, &inferred_ret);
                    let inferred_ret = if *is_async && promise_payload(&inferred_ret).is_none() {
                        promise_of(inferred_ret)
                    } else {
                        inferred_ret
                    };
                    let fn_type = Type::Function {
                        params: params.iter().map(|p| p.ty.clone()).collect(),
                        ret: Box::new(inferred_ret),
                    };
                    if let Some(scope) = self.scopes.last_mut()
                        && let Some(symbol) = scope.get_mut(name)
                    {
                        symbol.ty = ValueType::Known(fn_type);
                    }
                } else {
                    self.check_function(params, ret_type, body, *is_async);
                }
                self.restore_enclosing_try_handlers(enclosing_tries);
                self.current_throws.pop();
                self.pop_type_param_constraints();
            }
            Item::Stmt(Stmt {
                kind:
                    StmtKind::VarDecl {
                        mutable,
                        name,
                        ty,
                        value,
                        ..
                    },
                span,
            }) => {
                self.check_type_name(ty, *span);
                if let Some(value) = value {
                    let hoisted = self.scopes.last_mut().and_then(|scope| scope.remove(name));
                    let expected_ty =
                        (!matches!(ty, Type::Infer)).then(|| self.resolve_type_annotation(ty));
                    let actual = if matches!(ty, Type::Infer) {
                        self.check_expr(value)
                    } else {
                        self.check_expr_as(value, expected_ty.as_ref().unwrap())
                    };
                    let resolved_ty = expected_ty
                        .unwrap_or_else(|| self.resolve_declared_type(name, ty, value, &actual));
                    self.require_assignable_on_declaration(&resolved_ty, &actual, name, "E0308");
                    self.require_integer_literal_fits(&resolved_ty, value);
                    if matches!(ty, Type::Infer)
                        && let ExprKind::ObjectLit(fields) = value.kind()
                    {
                        self.validate_object_literal_decorators(&resolved_ty, fields);
                    }
                    let array_len = self.initial_array_len(&resolved_ty, value);
                    if let (Some(scope), Some(mut symbol)) = (self.scopes.last_mut(), hoisted) {
                        symbol.ty = ValueType::known(&resolved_ty);
                        symbol.array_len = array_len;
                        if matches!(actual, ValueType::Known(Type::Function { .. })) {
                            symbol.kind = SymbolKind::Function;
                        }
                        scope.insert(name.clone(), symbol);
                    }
                    self.record_constant_integer(name, *mutable, Some(value));
                }
            }
            Item::Stmt(statement) => self.check_stmt(statement),
        }
    }

    fn resolve_type_query(&mut self, ty: &Type) -> Type {
        match ty {
            Type::TypeQuery(expr) => {
                let value_ty = self.check_expr(expr);
                value_type_to_type(&value_ty).unwrap_or(Type::Unknown)
            }
            Type::Readonly(inner) => Type::Readonly(Box::new(self.resolve_type_query(inner))),
            Type::Array(inner) => Type::Array(Box::new(self.resolve_type_query(inner))),
            Type::Ref(inner) => Type::Ref(Box::new(self.resolve_type_query(inner))),
            Type::MutRef(inner) => Type::MutRef(Box::new(self.resolve_type_query(inner))),
            Type::Optional(inner) => Type::Optional(Box::new(self.resolve_type_query(inner))),
            Type::Function { params, ret } => Type::Function {
                params: params.iter().map(|p| self.resolve_type_query(p)).collect(),
                ret: Box::new(self.resolve_type_query(ret)),
            },
            Type::GenericFunction {
                type_params,
                params,
                ret,
            } => Type::GenericFunction {
                type_params: type_params.clone(),
                params: params.iter().map(|p| self.resolve_type_query(p)).collect(),
                ret: Box::new(self.resolve_type_query(ret)),
            },
            Type::Apply { name, args } => Type::Apply {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|arg| self.resolve_type_query(arg))
                    .collect(),
            },
            _ => ty.clone(),
        }
    }

    fn push_type_param_constraints(&mut self, type_params: &[crate::ast::TypeParam]) {
        let scope = type_params
            .iter()
            .filter_map(|param| {
                param
                    .constraint
                    .clone()
                    .map(|constraint| (param.name.clone(), constraint))
            })
            .collect();
        self.type_param_constraints.push(scope);
        self.type_param_names.push(
            type_params
                .iter()
                .map(|param| param.name.clone())
                .collect::<HashSet<_>>(),
        );
    }

    fn pop_type_param_constraints(&mut self) {
        self.type_param_constraints.pop();
        self.type_param_names.pop();
    }

    fn type_param_constraint(&self, name: &str) -> Option<Type> {
        self.type_param_constraints
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn check_function(&mut self, params: &[Param], ret_type: &Type, body: &[Stmt], is_async: bool) {
        for param in params {
            self.check_type_name(&param.ty, param.span);
        }
        let anchor = self.current_span().unwrap_or(Span::DUMMY);
        self.check_type_name(ret_type, anchor);
        let ret_type = self.resolve_type_annotation(ret_type);
        let expected_return = if is_async {
            if matches!(&ret_type, Type::Infer) {
                Type::Infer
            } else if let Some(payload) = promise_payload(&ret_type) {
                payload.clone()
            } else {
                let ret_type_subject = ret_type.to_string();
                self.error(
                    "E3003",
                    format!("async function must return `Promise<T>`, found `{ret_type}`"),
                    Some(&ret_type_subject),
                );
                ret_type.clone()
            }
        } else {
            ret_type.clone()
        };
        let previous_return = self.expected_return.replace(expected_return.clone());
        self.push_scope();
        self.fn_depth += 1;
        self.async_function_stack.push(is_async);
        if is_async {
            self.async_depth += 1;
        }
        for param in params {
            self.reject_bare_literal_annotation(
                &param.ty,
                param.pattern.binding_names().first().copied(),
            );
            let annotation_type = self.resolve_type_annotation(&param.ty);
            let param_type = if param.optional && param.default_value.is_none() && !param.rest {
                Type::Optional(Box::new(annotation_type.clone()))
            } else {
                annotation_type.clone()
            };
            match &param.pattern {
                ParamPattern::Name(name) => {
                    let facts = facts_from_decorators(&param.decorators);
                    let local_type = type_narrowed_by_validator_facts(&param_type, &facts);
                    self.declare(
                        name,
                        Symbol {
                            ty: ValueType::known(&local_type),
                            mutable: param.mutable,
                            kind: SymbolKind::Parameter,
                            used: false,
                            array_len: None,
                        },
                    );
                    self.declare_validator_facts(name, facts);
                    if let Some(default) = &param.default_value {
                        let actual = self.check_expr_as(default, &annotation_type);
                        self.require_assignable(&annotation_type, &actual, name, "E0308");
                    }
                }
                ParamPattern::Array(names) => {
                    let element_ty = match &param_type {
                        Type::Array(element) => *element.clone(),
                        other => {
                            self.error(
                                "E0308",
                                format!("cannot destructure array parameter from type `{other}`"),
                                None,
                            );
                            Type::Void
                        }
                    };
                    for name in names {
                        self.declare(
                            name,
                            Symbol {
                                ty: ValueType::known(&element_ty),
                                mutable: param.mutable,
                                kind: SymbolKind::Parameter,
                                used: false,
                                array_len: None,
                            },
                        );
                    }
                }
                ParamPattern::Object(names) => {
                    let fields = match &param_type {
                        Type::Named(name) => self.interfaces.get(name).cloned().unwrap_or_default(),
                        Type::Object(fields) => fields.clone(),
                        other => {
                            self.error(
                                "E0308",
                                format!("cannot destructure object parameter from type `{other}`"),
                                None,
                            );
                            vec![]
                        }
                    };

                    for name in names {
                        let field = fields.iter().find(|f| f.name == *name).cloned();
                        let field_ty = field.as_ref().map(field_value_type).unwrap_or_else(|| {
                            self.error(
                                "E0425",
                                format!("field `{name}` not found in parameter type"),
                                None,
                            );
                            Type::Void
                        });
                        let facts = field
                            .as_ref()
                            .map(|field| facts_from_decorators(&field.decorators))
                            .unwrap_or_default();
                        let local_type = type_narrowed_by_validator_facts(&field_ty, &facts);
                        self.declare(
                            name,
                            Symbol {
                                ty: ValueType::known(&local_type),
                                mutable: param.mutable,
                                kind: SymbolKind::Parameter,
                                used: false,
                                array_len: None,
                            },
                        );
                        self.declare_validator_facts(name, facts);
                    }
                }
            }
        }
        for statement in body {
            self.check_stmt(statement);
        }
        if is_async {
            self.check_async_codegen_boundaries(params, &ret_type, body);
        }
        if !matches!(expected_return, Type::Void | Type::Infer)
            && !block_always_returns(body, &self.exhaustive_matches)
        {
            if is_async {
                let payload = expected_return.to_string();
                self.error(
                    "E3004",
                    format!(
                        "async function returning `{ret_type}` does not return a value assignable to `{payload}` on every path"
                    ),
                    Some(&payload),
                );
            } else {
                self.error(
                    "E0310",
                    format!("function with return type `{ret_type}` does not return on every path"),
                    None,
                );
            }
        }
        if is_async {
            self.async_depth -= 1;
        }
        self.async_function_stack.pop();
        self.fn_depth -= 1;
        self.pop_scope();
        self.expected_return = previous_return;
    }

    fn check_match_exhaustiveness(
        &mut self,
        subject_ty: &ValueType,
        arms: &[crate::ast::MatchArm],
        has_default: bool,
    ) -> bool {
        let Some(variants) = self.match_subject_variants(subject_ty) else {
            return false;
        };
        if variants.is_empty() {
            return false;
        }

        let mut covered = vec![false; variants.len()];
        let mut fully_covered = false;

        for (arm_index, arm) in arms.iter().enumerate() {
            let matched = if matches!(subject_ty, ValueType::Known(Type::Result { .. })) {
                match &arm.pattern {
                    MatchPattern::ResultVariant { success, .. } => vec![usize::from(!success)],
                    MatchPattern::Binding(_) => vec![arm_index.min(1)],
                    _ => match_pattern_variant_indices(&arm.pattern, &variants),
                }
            } else {
                match_pattern_variant_indices(&arm.pattern, &variants)
            };
            let contributes = matched.iter().any(|index| !covered[*index]);

            if fully_covered || matched.is_empty() || !contributes {
                self.warning(
                    "W0002",
                    "unreachable match arm".to_string(),
                    match_pattern_subject(&arm.pattern).as_deref(),
                );
            }

            if arm.guard.is_none() {
                for index in matched {
                    covered[index] = true;
                }
                fully_covered = covered.iter().all(|is_covered| *is_covered);
            }
        }

        if has_default {
            return true;
        }

        let missing = variants
            .iter()
            .zip(&covered)
            .filter(|(_, is_covered)| !**is_covered)
            .map(|(variant, _)| variant.to_string())
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            self.error(
                "E0004",
                format!(
                    "non-exhaustive match: missing pattern(s) for {}",
                    missing.join(", ")
                ),
                None,
            );
            return false;
        }
        true
    }

    fn match_subject_variants(&self, subject_ty: &ValueType) -> Option<Vec<Type>> {
        match subject_ty {
            ValueType::Known(Type::Optional(inner)) => {
                Some(vec![inner.as_ref().clone(), Type::Optional(inner.clone())])
            }
            ValueType::Known(Type::Union(variants)) => Some(variants.clone()),
            ValueType::Known(Type::Result { success, error }) => {
                Some(vec![success.as_ref().clone(), error.as_ref().clone()])
            }
            ValueType::Known(Type::Named(name)) => {
                if let Some(enum_info) = self.enums.get(name)
                    && enum_info.is_algebraic
                {
                    return Some(
                        enum_info
                            .members
                            .iter()
                            .map(|(member_name, _)| Type::Named(format!("{name}.{member_name}")))
                            .collect(),
                    );
                }
                match self.type_aliases.get(name) {
                    Some(Type::Union(variants)) => Some(variants.clone()),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn resolve_type_annotation(&mut self, ty: &Type) -> Type {
        if matches!(ty, Type::Infer) {
            return Type::Infer;
        }
        self.mark_type_names_used(ty);
        let ty = self.resolve_type_query(ty);
        let mut resolve_diags = Vec::new();
        let resolved = self.generic_defs.resolve(&ty, &mut resolve_diags);
        for diagnostic in resolve_diags {
            self.error(
                diagnostic.code,
                diagnostic.message,
                diagnostic.subject.as_deref(),
            );
        }
        self.resolve_type_query(&resolved)
    }

    fn mark_type_names_used(&mut self, ty: &Type) {
        match ty {
            Type::Named(name) | Type::Generic(name) | Type::InferVar(name) => {
                self.mark_used(name);
            }
            Type::Apply { name, args } => {
                self.mark_used(name);
                for arg in args {
                    self.mark_type_names_used(arg);
                }
            }
            Type::Array(inner)
            | Type::Readonly(inner)
            | Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::Optional(inner)
            | Type::KeyOf(inner)
            | Type::Set(inner)
            | Type::BinaryHeap(inner)
            | Type::BinomialHeap(inner)
            | Type::UnionFind(inner)
            | Type::Counter(inner)
            | Type::Deque(inner)
            | Type::LinkedList(inner)
            | Type::DoubleLinkedList(inner)
            | Type::AvlTree(inner)
            | Type::RedBlackTree(inner)
            | Type::SplayTree(inner)
            | Type::Treap(inner)
            | Type::SortedSet(inner)
            | Type::SkipList(inner)
            | Type::Range(inner) => self.mark_type_names_used(inner),
            Type::Throws { function, effects } => {
                self.mark_type_names_used(function);
                for effect in effects {
                    self.mark_type_names_used(effect);
                }
            }
            Type::Result { success, error } => {
                self.mark_type_names_used(success);
                self.mark_type_names_used(error);
            }
            Type::SortedMap(key, value) => {
                self.mark_type_names_used(key);
                self.mark_type_names_used(value);
            }
            Type::Graph(node, weight) => {
                self.mark_type_names_used(node);
                self.mark_type_names_used(weight);
            }
            Type::Tuple(types) | Type::Union(types) => {
                for ty in types {
                    self.mark_type_names_used(ty);
                }
            }
            Type::Function { params, ret } => {
                for param in params {
                    self.mark_type_names_used(param);
                }
                self.mark_type_names_used(ret);
            }
            Type::GenericFunction {
                type_params,
                params,
                ret,
            } => {
                for param in type_params {
                    if let Some(constraint) = &param.constraint {
                        self.mark_type_names_used(constraint);
                    }
                }
                for param in params {
                    self.mark_type_names_used(param);
                }
                self.mark_type_names_used(ret);
            }
            Type::Object(fields) => {
                for field in fields {
                    self.mark_type_names_used(&field.ty);
                }
            }
            Type::Conditional {
                check,
                bound,
                then,
                otherwise,
            } => {
                self.mark_type_names_used(check);
                self.mark_type_names_used(bound);
                self.mark_type_names_used(then);
                self.mark_type_names_used(otherwise);
            }
            Type::Map(key, value) => {
                self.mark_type_names_used(key);
                self.mark_type_names_used(value);
            }
            Type::Infer
            | Type::I8
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
            | Type::Decimal
            | Type::Bool
            | Type::Char
            | Type::Regex
            | Type::Str
            | Type::Void
            | Type::Unknown
            | Type::Never
            | Type::Lit(_)
            | Type::TypeQuery(_)
            | Type::FenwickTree
            | Type::BubbleSort
            | Type::SegmentTree
            | Type::SegmentTreeLazy
            | Type::SparseTable
            | Type::BitSet
            | Type::Trie
            | Type::BloomFilter
            | Type::SuffixTree
            | Type::Lca
            | Type::Hld
            | Type::Centroid => {}
        }
    }

    fn resolve_declared_type(
        &mut self,
        name: &str,
        ty: &Type,
        value: &Expr,
        actual: &ValueType,
    ) -> Type {
        if matches!(actual, ValueType::Error) {
            return Type::Unknown;
        }
        if matches!(ty, Type::Infer) {
            if let ValueType::Object(_) = actual {
                match self.infer_named_object_binding_type(actual) {
                    ObjectBindingInference::Unique(interface_name) => {
                        self.error_on_declaration_with_help(
                            "E0310",
                            format!(
                                "object literal `{name}` requires an explicit interface annotation"
                            ),
                            Some(name),
                            vec![format!("annotate it as `: {interface_name}`")],
                        );
                        return Type::Unknown;
                    }
                    ObjectBindingInference::Ambiguous(candidates) => {
                        let named = candidates
                            .iter()
                            .map(|candidate| format!("`{candidate}`"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        self.error_on_declaration_with_help(
                            "E0310",
                            format!(
                                "object literal `{name}` requires an explicit interface annotation; {} interfaces have this shape: {named}",
                                candidates.len()
                            ),
                            Some(name),
                            candidates
                                .iter()
                                .map(|candidate| format!("annotate it as `: {candidate}`"))
                                .collect(),
                        );
                        return Type::Unknown;
                    }
                    ObjectBindingInference::None => {
                        self.error_on_declaration(
                            "E0310",
                            format!(
                                "object literal `{name}` requires an explicit interface annotation"
                            ),
                            Some(name),
                        );
                        return Type::Unknown;
                    }
                }
            }
            match infer_binding_type(value, actual) {
                Some(inferred) => {
                    self.contextual_parameters.push(InferredParameter {
                        name: name.to_string(),
                        ty: inferred.clone(),
                    });
                    inferred
                }
                None => {
                    let message = if matches!(actual, ValueType::UnsignedIntegerLiteral) {
                        format!(
                            "cannot infer unsigned type for `{name}`; add an explicit `: u128` annotation"
                        )
                    } else {
                        format!(
                            "cannot infer a type for `{name}`; add an explicit `: type` annotation"
                        )
                    };
                    self.error_on_declaration("E0310", message, Some(name));
                    Type::Unknown
                }
            }
        } else {
            let resolved = self.resolve_type_annotation(ty);
            match &resolved {
                Type::Named(alias) => self
                    .type_aliases
                    .get(alias)
                    .cloned()
                    .unwrap_or_else(|| resolved.clone()),
                _ => resolved,
            }
        }
    }

    fn infer_named_object_binding_type(&self, actual: &ValueType) -> ObjectBindingInference {
        let mut matches = self
            .interfaces
            .iter()
            .filter(|(_, fields)| self.object_fields_exactly_match_interface(fields, actual))
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        matches.sort();
        match matches.len() {
            0 => ObjectBindingInference::None,
            1 => ObjectBindingInference::Unique(matches.remove(0)),
            _ => ObjectBindingInference::Ambiguous(matches),
        }
    }

    fn direct_onfail_exception(&self, value: &Expr) -> Option<Type> {
        if matches!(value.kind(), ExprKind::Await(_)) {
            return Some(Type::Named("Error".to_string()));
        }
        let ExprKind::FnCall { callee, .. } = value.kind() else {
            return None;
        };
        let effects = self.fn_throws.get(callee)?;
        (effects.len() == 1).then(|| effects[0].clone())
    }

    fn check_onfail(
        &mut self,
        name: &str,
        onfail: &OnFail,
        actual: ValueType,
        exception: Option<Type>,
    ) -> ValueType {
        if let ValueType::Known(Type::Result { success, error }) = actual {
            return self.check_onfail_result(name, onfail, *success, *error);
        }
        let Some(exception) = exception else {
            if !matches!(actual, ValueType::Error) {
                self.error_at(
                    "E0277",
                    format!(
                        "`onfail` requires a `Result<T, E>` initializer or a direct call with one declared `throws` type, found `{}`",
                        actual.display()
                    ),
                    onfail.span,
                );
            }
            return ValueType::Error;
        };

        self.check_onfail_handler(name, onfail, exception.clone());
        if !matches!(self.expected_return, Some(Type::Void)) {
            self.check_unhandled_effect(
                &exception,
                "implicit error propagation from `onfail`",
                onfail.span,
                true,
            );
        }
        actual
    }

    fn check_onfail_result(
        &mut self,
        name: &str,
        onfail: &OnFail,
        success: Type,
        error: Type,
    ) -> ValueType {
        self.check_onfail_handler(name, onfail, error.clone());
        if self.onfail_handler_always_exits(onfail) {
            return ValueType::Known(success);
        }
        match self.expected_return.as_ref() {
            Some(Type::Void) => {}
            Some(Type::Result { error: expected_error, .. }) if error == **expected_error => {}
            Some(Type::Result { error: expected_error, .. }) => self.error_at_with_help(
                "E0308",
                format!("`onfail` handles `{error}`, but the enclosing function returns `{expected_error}` as its error type"),
                onfail.span,
                vec![format!(
                    "reaching the end of the handler propagates `Err({name})`; end the handler with a `return` of the enclosing function's type instead"
                )],
            ),
            Some(Type::Infer) => {
                if let Some(pending) = self.inferred_onfail_results.last_mut() {
                    pending.push((error.clone(), onfail.span));
                } else {
                    self.error_at(
                        "E0277",
                        "`onfail` on a `Result<T, E>` requires the enclosing function to return `Result<U, E>` or `void`".to_string(),
                        onfail.span,
                    );
                }
            }
            Some(expected) => self.error_at_with_help(
                "E0277",
                format!("the `onfail` handler for `{name}` can reach its end, but the enclosing function returns `{expected}`, so the error cannot propagate implicitly"),
                onfail.span,
                vec![format!(
                    "end the handler with `return <value>;` or `throw`, or make the enclosing function return `Result<U, {error}>` or `void`"
                )],
            ),
            None => {}
        }
        ValueType::Known(success)
    }

    fn onfail_handler_always_exits(&self, onfail: &OnFail) -> bool {
        match &onfail.handler {
            OnFailHandler::Expr(handler) => stmt_always_returns(
                &Stmt::synthetic(StmtKind::Expr(handler.clone())),
                &self.exhaustive_matches,
            ),
            OnFailHandler::Block(handler) => {
                matches!(
                    handler.last().map(Stmt::kind),
                    Some(StmtKind::Continue | StmtKind::Break(_))
                ) || block_always_returns(handler, &self.exhaustive_matches)
            }
        }
    }

    fn validate_inferred_onfail_results(&mut self, pending: &[(Type, Span)], return_type: &Type) {
        for (error, span) in pending {
            match transparent_type(return_type) {
                Type::Void => {}
                Type::Result {
                    error: expected_error,
                    ..
                } if expected_error.as_ref() == error => {}
                Type::Result {
                    error: expected_error,
                    ..
                } => self.error_at(
                    "E0308",
                    format!(
                        "`onfail` handles `{error}`, but the inferred function return uses `{expected_error}` as its error type"
                    ),
                    *span,
                ),
                _ => self.error_at(
                    "E0277",
                    format!(
                        "`onfail` on a `Result<T, E>` requires the inferred enclosing return type to be `Result<U, E>` or `void`, found `{return_type}`"
                    ),
                    *span,
                ),
            }
        }
    }

    fn check_onfail_handler(&mut self, name: &str, onfail: &OnFail, error: Type) {
        if let Type::Named(error_name) = &error
            && let Some(fields) = crate::builtin_interfaces::native_error_fields(error_name)
        {
            self.interfaces.entry(error_name.clone()).or_insert(fields);
        }
        self.onfail_bindings.push(InferredOnFailBinding {
            name: name.to_string(),
            ty: error.clone(),
            span: onfail.span,
        });
        self.push_scope();
        self.declare(
            name,
            Symbol {
                ty: ValueType::Known(error),
                mutable: false,
                kind: SymbolKind::Variable,
                used: false,
                array_len: None,
            },
        );
        match &onfail.handler {
            OnFailHandler::Expr(handler) => {
                self.check_expr(handler);
            }
            OnFailHandler::Block(handler) => self.check_block(handler),
        }
        self.pop_scope();
    }

    fn check_parse_builtin_at_width(&mut self, expr: &Expr, expected: &Type) -> Option<ValueType> {
        let Expr {
            kind: ExprKind::FnCall { callee, args, .. },
            ..
        } = expr
        else {
            return None;
        };
        let Type::Optional(target) = transparent_type(expected) else {
            return None;
        };
        let target = transparent_type(target);
        let supported = match callee.as_str() {
            "parseInt" => is_integer_type(target),
            "parseFloat" | "Number" => matches!(target, Type::F32 | Type::F64),
            "parseDecimal" => matches!(target, Type::Decimal),
            _ => false,
        };
        if !supported || args.len() != 1 {
            return None;
        }
        let actual = self.check_expr(&args[0]);
        self.require_assignable(&Type::Str, &actual, callee, "E0308");
        Some(ValueType::Known(expected.clone()))
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the parameters mirror the arrow-function AST fields and keeping borrowed slices avoids cloning"
    )]
    fn check_arrow(
        &mut self,
        type_params: &[crate::ast::TypeParam],
        params: &[Param],
        declared_return: &Type,
        throws: &[Type],
        body: &[Stmt],
        expected: Option<&Type>,
        is_async: bool,
        closure_span: Span,
    ) -> ValueType {
        if is_async {
            let mut captures = closure_capture_names(params, body)
                .into_iter()
                .filter(|name| !self.enums.contains_key(name))
                .filter_map(|name| {
                    self.lookup(&name).and_then(|symbol| {
                        matches!(symbol.kind, SymbolKind::Variable | SymbolKind::Parameter)
                            .then(|| (name, symbol.ty.clone()))
                    })
                })
                .collect::<Vec<_>>();
            captures.sort_by(|left, right| left.0.cmp(&right.0));
            for (name, ty) in captures {
                let ownership_safe = matches!(ty, ValueType::Known(ref ty) if ty.is_copy() || self.async_owned_value_supported(ty));
                if ownership_safe {
                    continue;
                }
                self.error_at_with_subject(
                    "E3085",
                    format!(
                        "async closure capture `{name}` cannot be copied safely into the owned environment frame"
                    ),
                    closure_span,
                    name,
                );
            }
        }
        if !self.confined_closures.contains_literal(closure_span) {
            let declared_here = closure_capture_names(params, body);
            for (name, write_span) in
                crate::escape_analysis::closure_assigned_captures(params, body)
            {
                if !declared_here.contains(&name) {
                    continue;
                }
                let Some(symbol) = self.lookup(&name) else {
                    continue;
                };
                if !matches!(symbol.kind, SymbolKind::Variable | SymbolKind::Parameter) {
                    continue;
                }
                self.error_at_with_help(
                    "E0436",
                    format!(
                        "cannot assign to `{name}` from inside a closure that outlives its scope"
                    ),
                    write_span,
                    vec![
                        format!(
                            "this closure escapes the function that declared `{name}`, so it captures a copy; the write lands in that copy and nothing can observe it"
                        ),
                        format!(
                            "return the value from the closure instead of writing `{name}`, or hold the state in a `Mutex` both sides can reach"
                        ),
                    ],
                );
                if let Some(last) = self.diagnostics.last_mut() {
                    last.subject = Some(name);
                }
            }
        }

        let mut distant_captures = closure_capture_names(params, body)
            .into_iter()
            .filter(|name| !self.enums.contains_key(name))
            .filter_map(|name| {
                let declared_at = self.declaration_fn_depth(&name)?;
                let crosses = self.fn_depth.checked_sub(declared_at)?;
                (declared_at > 0 && crosses >= 1).then_some((name, crosses + 1))
            })
            .collect::<Vec<_>>();
        distant_captures.sort();
        for (name, levels) in distant_captures {
            self.error_at_with_subject(
                "E0435",
                format!(
                    "closure captures `{name}` from {levels} functions out; a closure can only capture from the function directly around it"
                ),
                closure_span,
                name.clone(),
            );
        }

        let expected_function = match expected {
            Some(Type::Function { params, ret }) => Some((params.as_slice(), ret.as_ref())),
            _ => None,
        };
        let resolved_params = params
            .iter()
            .enumerate()
            .map(|(index, parameter)| {
                let mut parameter = parameter.clone();
                if matches!(parameter.ty, Type::Infer) {
                    if let Some(expected) =
                        expected_function.and_then(|(parameters, _)| parameters.get(index))
                    {
                        parameter.ty = expected.clone();
                    } else {
                        let param_name = parameter
                            .pattern
                            .binding_names()
                            .first()
                            .copied()
                            .unwrap_or("param")
                            .to_string();
                        self.error(
                            "E0282",
                            format!("cannot infer type of parameter `{param_name}`"),
                            Some(&param_name),
                        );
                    }
                }
                parameter
            })
            .collect::<Vec<_>>();
        for (declared, resolved) in params.iter().zip(&resolved_params) {
            if matches!(declared.ty, Type::Infer) && !matches!(resolved.ty, Type::Infer) {
                self.contextual_parameters.push(InferredParameter {
                    name: declared
                        .pattern
                        .binding_names()
                        .first()
                        .copied()
                        .unwrap_or("param")
                        .to_string(),
                    ty: resolved.ty.clone(),
                });
            }
        }
        let contextual_return = expected_function.map(|(_, ret)| ret);
        let return_type = if matches!(declared_return, Type::Infer) {
            contextual_return
                .filter(|ty| !matches!(ty, Type::Infer))
                .cloned()
                .unwrap_or(Type::Infer)
        } else {
            declared_return.clone()
        };

        self.inferred_returns.push(Vec::new());
        let infer_return_type = matches!(return_type, Type::Infer);
        if infer_return_type {
            self.inferred_onfail_results.push(Vec::new());
        }
        let outer_topics = std::mem::take(&mut self.topic_stack);
        let mut effects = throws.to_vec();
        if resolved_params
            .iter()
            .any(|param| !param.decorators.is_empty())
            && !effects
                .iter()
                .any(|ty| ty == &Type::Named("ValidationError".to_string()))
        {
            effects.push(Type::Named("ValidationError".to_string()));
        }
        self.current_throws.push(effects);
        let enclosing_tries = self.take_enclosing_try_handlers();
        self.check_function(&resolved_params, &return_type, body, is_async);
        self.restore_enclosing_try_handlers(enclosing_tries);
        self.current_throws.pop();
        self.topic_stack = outer_topics;
        let inferred = self.inferred_returns.pop().unwrap_or_default();
        let onfail_results = if infer_return_type {
            self.inferred_onfail_results.pop().unwrap_or_default()
        } else {
            Vec::new()
        };
        let return_type = if matches!(return_type, Type::Infer) {
            infer_callback_return(&inferred).unwrap_or(Type::Void)
        } else {
            return_type
        };
        self.validate_inferred_onfail_results(&onfail_results, &return_type);
        let return_type = if is_async {
            if promise_payload(&return_type).is_some() {
                return_type
            } else {
                promise_of(return_type)
            }
        } else {
            return_type
        };
        let function = if type_params.is_empty() {
            Type::Function {
                params: resolved_params
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect(),
                ret: Box::new(return_type),
            }
        } else {
            Type::GenericFunction {
                type_params: type_params.to_vec(),
                params: resolved_params
                    .iter()
                    .map(|parameter| parameter.ty.clone())
                    .collect(),
                ret: Box::new(return_type),
            }
        };
        ValueType::Known(function)
    }

    fn check_in(&mut self, key: &Expr, object: &Expr) -> ValueType {
        let key_ty = self.check_expr(key);
        let object_ty = self.check_expr(object);
        let resolved_object_ty = match &object_ty {
            ValueType::Known(ty) => Some(ty.clone()),
            _ => None,
        };
        match resolved_object_ty.as_ref().map(transparent_type) {
            Some(
                Type::Range(elem)
                | Type::Array(elem)
                | Type::Set(elem)
                | Type::BinaryHeap(elem)
                | Type::BinomialHeap(elem)
                | Type::UnionFind(elem)
                | Type::Counter(elem)
                | Type::Deque(elem),
            ) => {
                if !matches!(key_ty, ValueType::Unknown) && !self.is_assignable(elem, &key_ty) {
                    self.error(
                        "E0308",
                        format!(
                            "left side of `in` must be `{elem}`, found `{}`",
                            key_ty.display()
                        ),
                        expression_subject(key),
                    );
                }
            }
            Some(Type::Map(map_key, _)) => {
                if !matches!(key_ty, ValueType::Unknown) && !self.is_assignable(map_key, &key_ty) {
                    self.error(
                        "E0308",
                        format!(
                            "left side of `in` must be `{map_key}`, found `{}`",
                            key_ty.display()
                        ),
                        expression_subject(key),
                    );
                }
            }
            Some(Type::Named(_) | Type::Object(_)) | None => {
                if !matches!(key_ty, ValueType::Known(Type::Str) | ValueType::Unknown) {
                    self.error(
                        "E0308",
                        format!(
                            "left side of `in` must be a string, found `{}`",
                            key_ty.display()
                        ),
                        expression_subject(key),
                    );
                }
                if let Some(ty) = resolved_object_ty.as_ref()
                    && !matches!(
                        transparent_type(ty),
                        Type::Named(_) | Type::Object(_) | Type::Unknown
                    )
                {
                    self.error(
                            "E0308",
                            format!(
                                "right side of `in` must be an object, array, Range, Map or Set, found `{ty}`"
                            ),
                            expression_subject(object),
                        );
                }
            }
            Some(_) => {
                self.error(
                    "E0308",
                    format!(
                        "right side of `in` must be an object, array, Range, Map or Set, found `{}`",
                        object_ty.display()
                    ),
                    expression_subject(object),
                );
            }
        }
        ValueType::Known(Type::Bool)
    }

    fn check_is(&mut self, expr: &Expr, ty: &Type) -> ValueType {
        let expr_ty = self.check_expr(expr);
        if let ExprKind::Ident(name) = expr.kind() {
            let checks_subtype = self.type_extends_error(ty)
                && matches!(&expr_ty, ValueType::Known(Type::Named(bound)) if bound == "Error");
            let from_catch = self
                .error_catch_bindings
                .iter()
                .rev()
                .find(|(binding, _)| binding == name)
                .is_some_and(|(_, depth)| *depth == self.fn_depth);
            if checks_subtype && !from_catch {
                self.error_at_with_help(
                    "E3297",
                    format!(
                        "`{name} is {ty}` cannot tell the type of an `Error` that did not come from a `catch`"
                    ),
                    expr.span,
                    vec![format!(
                        "check `is {ty}` on the `catch` binding itself, in the function that declares the `catch`"
                    )],
                );
                return ValueType::Known(Type::Bool);
            }
        }
        match &expr_ty {
            ValueType::Known(Type::Union(variants)) => {
                if !variants
                    .iter()
                    .any(|v| self.is_assignable(ty, &ValueType::Known(v.clone())))
                {
                    self.error(
                        "E0308",
                        format!("`{}` can never be of type `{}`", expr_ty.display(), ty),
                        expression_subject(expr),
                    );
                }
            }
            ValueType::Known(Type::Named(name))
                if name == "Error"
                    && !matches!(ty, Type::Named(target) if target == "Error")
                    && !self.type_extends_error(ty) =>
            {
                self.error(
                    "E0308",
                    format!("`Error` can never be of type `{ty}`"),
                    expression_subject(expr),
                );
            }
            ValueType::Unknown => {}
            ValueType::Error => return ValueType::Error,
            _ => {}
        }
        ValueType::Known(Type::Bool)
    }

    fn narrowed_error_subtype(&self, current: &Type, target: &Type) -> Option<Type> {
        (matches!(current, Type::Named(name) if name == "Error") && self.type_extends_error(target))
            .then(|| target.clone())
    }

    fn expected_collection(&self, expected: &Type) -> Type {
        match expected {
            Type::Named(name) => self
                .type_aliases
                .get(name)
                .cloned()
                .unwrap_or_else(|| expected.clone()),
            _ => expected.clone(),
        }
    }

    fn expected_set_element(&self, expected: &Type) -> Option<Type> {
        match transparent_type(&self.expected_collection(expected)) {
            Type::Set(element) => Some(element.as_ref().clone()),
            _ => None,
        }
    }

    fn expected_map_entry(&self, expected: &Type) -> Option<(Type, Type)> {
        match transparent_type(&self.expected_collection(expected)) {
            Type::Map(key, value) => Some((key.as_ref().clone(), value.as_ref().clone())),
            _ => None,
        }
    }

    fn check_new_set(
        &mut self,
        ty: Option<&Type>,
        initial: Option<&Expr>,
        expected: Option<&Type>,
    ) -> ValueType {
        let from_context = expected.and_then(|expected| self.expected_set_element(expected));
        let ty = ty.or(from_context.as_ref());
        let element = match (ty, initial) {
            (Some(t), Some(init)) => {
                self.check_expr_as(init, &Type::Array(Box::new(t.clone())));
                Some(t.clone())
            }
            (Some(t), None) => Some(t.clone()),
            (None, Some(init)) => match self.check_expr(init) {
                ValueType::Array(values) => infer_array_element_type(&values),
                ValueType::Known(Type::Array(element)) => Some(*element),
                _ => {
                    self.error(
                        "E0282",
                        "`Set(...)` initializer must be an array literal".to_string(),
                        None,
                    );
                    None
                }
            },
            (None, None) => None,
        };
        let Some(element) = element else {
            self.error(
                "E0282",
                "cannot infer `Set` element type; add a type argument like `Set<i32>()`"
                    .to_string(),
                None,
            );
            return ValueType::Error;
        };
        if !is_collection_key_type(&element) {
            self.error(
                "E3241",
                format!(
                    "`Set` elements must be a string or a numeric or boolean type, found `{element}`"
                ),
                None,
            );
            return ValueType::Error;
        }
        ValueType::Known(Type::Set(Box::new(element)))
    }

    fn check_new_map(
        &mut self,
        key: Option<&Type>,
        value: Option<&Type>,
        initial: Option<&Expr>,
        expected: Option<&Type>,
    ) -> ValueType {
        let from_context = expected.and_then(|expected| self.expected_map_entry(expected));
        let named = match (key, value) {
            (Some(key), Some(value)) => Some((key.clone(), value.clone())),
            _ => from_context,
        };
        let entries = match (&named, initial) {
            (Some(named), _) => Some(named.clone()),
            (None, Some(init)) => {
                let checked = self.check_expr(init);
                infer_map_entry_types(&checked)
            }
            (None, None) => None,
        };
        let Some((key, value)) = entries else {
            self.error(
                "E0282",
                "cannot infer `Map` types; add type arguments like `Map<string, i32>()`"
                    .to_string(),
                None,
            );
            return ValueType::Error;
        };
        if !is_collection_key_type(&key) {
            self.error(
                "E3240",
                format!("`Map` keys must be a string or a numeric or boolean type, found `{key}`"),
                None,
            );
            return ValueType::Error;
        }
        if named.is_some()
            && let Some(init) = initial
        {
            let entry = Type::Tuple(vec![key.clone(), value.clone()]);
            self.check_expr_as(init, &Type::Array(Box::new(entry)));
        }
        ValueType::Known(Type::Map(Box::new(key), Box::new(value)))
    }

    fn is_producible_typeof_name(&self, name: &str) -> bool {
        if let Some(element) = name.strip_suffix("[]") {
            return self.is_producible_typeof_name(element);
        }
        if let Some(inner) = name.strip_suffix('?') {
            return self.is_producible_typeof_name(inner);
        }
        crate::typeof_category::is_typeof_category(name)
            || self.interfaces.contains_key(name)
            || self.type_aliases.contains_key(name)
            || self.enums.contains_key(name)
    }

    fn unknown_unrepresentable_value(&self, expected: &Type, actual: &ValueType) -> Option<String> {
        let targets_unknown = match transparent_type(expected) {
            Type::Unknown => true,
            Type::Union(variants) => variants
                .iter()
                .any(|variant| matches!(transparent_type(variant), Type::Unknown)),
            _ => false,
        };
        if !targets_unknown {
            return None;
        }
        self.first_unrepresentable_in_unknown(actual)
    }

    fn first_unrepresentable_in_unknown(&self, actual: &ValueType) -> Option<String> {
        match actual {
            ValueType::Known(ty) => {
                let resolved = match transparent_type(ty) {
                    Type::Named(name) => self.type_aliases.get(name).unwrap_or(ty),
                    other => other,
                };
                (!crate::typeof_category::is_unknown_representable(transparent_type(resolved)))
                    .then(|| actual.display())
            }
            ValueType::Union(variants) => variants
                .iter()
                .find_map(|variant| self.first_unrepresentable_in_unknown(variant)),
            ValueType::Null | ValueType::Object(_) | ValueType::Array(_) => Some(actual.display()),
            ValueType::IntegerLiteral
            | ValueType::NegativeIntegerLiteral
            | ValueType::UnsignedIntegerLiteral
            | ValueType::FloatLiteral
            | ValueType::Unknown
            | ValueType::Error => None,
        }
    }

    fn check_chained_comparison(&mut self, terms: &[Expr], ops: &[BinOp]) -> ValueType {
        let term_types: Vec<ValueType> = terms.iter().map(|term| self.check_expr(term)).collect();
        for (i, op) in ops.iter().enumerate() {
            let left = &terms[i];
            let right = &terms[i + 1];
            let left_ty = &term_types[i];
            let right_ty = &term_types[i + 1];
            match op {
                BinOp::Gt | BinOp::Lt | BinOp::Gte | BinOp::Lte => {
                    self.numeric_result("comparison", left_ty, right_ty, left, right);
                }
                BinOp::Eq => {
                    if !types_overlap(left_ty, right_ty) {
                        self.error(
                            "E0308",
                            format!(
                                "cannot compare `{}` with `{}`",
                                left_ty.display(),
                                right_ty.display()
                            ),
                            expression_subject(right),
                        );
                    }
                }
                _ => unreachable!("chained comparisons only ever use <, <=, >, >=, =="),
            }
            self.warn_validator_fact_condition(op, left, right);
        }
        ValueType::Known(Type::Bool)
    }

    fn take_enclosing_try_handlers(&mut self) -> TryHandlers {
        (
            std::mem::take(&mut self.try_catch_type),
            std::mem::take(&mut self.try_body_suspends),
            std::mem::take(&mut self.try_catch_suspends),
            std::mem::take(&mut self.try_finally_suspends),
        )
    }

    fn restore_enclosing_try_handlers(&mut self, handlers: TryHandlers) {
        let (catch_types, body_suspends, catch_suspends, finally_suspends) = handlers;
        self.try_catch_type = catch_types;
        self.try_body_suspends = body_suspends;
        self.try_catch_suspends = catch_suspends;
        self.try_finally_suspends = finally_suspends;
    }

    fn check_unhandled_effect(
        &mut self,
        effect: &Type,
        operation: &str,
        span: Span,
        synchronous: bool,
    ) {
        let effect_is_error = self.type_extends_error(effect);
        let catches = |catch_type: &Type| {
            catch_type == effect
                || matches!(catch_type, Type::Union(types) if types.contains(effect))
                || (effect_is_error && matches!(catch_type, Type::Named(name) if name == "Error"))
        };
        let awaited = self.awaited_operand == Some(span);
        let dispatched = synchronous && !awaited;
        let catching_try = if dispatched || awaited {
            self.try_catch_type.iter().rposition(catches)
        } else {
            self.try_catch_type
                .len()
                .checked_sub(1)
                .filter(|index| catches(&self.try_catch_type[*index]))
        };
        let caught = catching_try.is_some();
        let propagated = self
            .current_throws
            .last()
            .is_some_and(|types| types.contains(effect));
        if dispatched && (caught || propagated) {
            self.refuse_throw_past_awaiting_finally(effect, operation, span, catching_try);
        }
        if !caught && !propagated {
            self.error_at(
                "E3011",
                format!(
                    "{operation} may throw `{effect}`; catch it or add `throws {effect}` to the enclosing function"
                ),
                span,
            );
        }
    }

    fn type_extends_error(&self, ty: &Type) -> bool {
        let resolved = match ty {
            Type::Named(name) => match self.type_aliases.get(name) {
                Some(aliased @ Type::Named(_)) => aliased,
                _ => ty,
            },
            _ => ty,
        };
        matches!(resolved, Type::Named(name) if name != "Error"
            && (crate::builtin_interfaces::native_error_extends_error(name)
                || self.generic_defs.interface_extends(name, "Error")))
    }

    fn union_variants_with_interface_fields(&self, variants: &[Type]) -> Vec<Type> {
        variants
            .iter()
            .map(|variant| match variant {
                Type::Named(name) => self
                    .interfaces
                    .get(name)
                    .map(|fields| Type::Object(fields.clone()))
                    .unwrap_or_else(|| variant.clone()),
                _ => variant.clone(),
            })
            .collect()
    }

    fn static_object_projection_keys(
        &mut self,
        argument: &Expr,
        method: &str,
    ) -> Option<Vec<StaticProjectionKey>> {
        if let ExprKind::ArrayLit(elements) = argument.kind() {
            let mut keys = Vec::with_capacity(elements.len());
            let mut valid = true;
            for element in elements {
                self.check_expr(element);
                if let ExprKind::StrLit(name) = element.kind() {
                    keys.push(StaticProjectionKey {
                        name: name.clone(),
                        span: element.span,
                    });
                } else {
                    self.error_at(
                        "E0308",
                        format!("`{method}` requires string literal field names"),
                        element.span,
                    );
                    valid = false;
                }
            }
            return valid.then_some(keys);
        }

        let argument_ty = self.check_expr(argument);
        let Some(names) = static_object_projection_key_names(&argument_ty) else {
            self.error_at(
                "E0308",
                format!(
                    "`{method}` requires an array whose element type is a finite union of string literal field names"
                ),
                argument.span,
            );
            return None;
        };
        Some(
            names
                .into_iter()
                .map(|name| StaticProjectionKey {
                    name,
                    span: argument.span,
                })
                .collect(),
        )
    }

    fn compared_by_contents(&self, ty: &ValueType) -> Option<(&'static str, bool)> {
        match ty {
            ValueType::Known(Type::Array(_)) => Some(("arrays", true)),
            ValueType::Known(Type::Named(name)) if self.interfaces.contains_key(name) => {
                Some(("objects", self.data_interfaces.contains(name)))
            }
            _ => None,
        }
    }

    fn check_jwt_claim_type(&mut self, claim_type: &Type, method: &str) {
        let fields = match claim_type {
            Type::Named(name) => self.interfaces.get(name).cloned().or_else(|| {
                self.type_aliases.get(name).and_then(|alias| match alias {
                    Type::Object(fields) => Some(fields.clone()),
                    _ => None,
                })
            }),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        };
        let Some(fields) = fields else {
            self.error(
                "E3279",
                format!(
                    "`jwt.{method}` requires a typed object for JWT claims, found `{claim_type}`"
                ),
                Some(method),
            );
            return;
        };
        for field in fields {
            let field_type = if field.optional {
                Type::Optional(Box::new(field.ty.clone()))
            } else {
                field.ty.clone()
            };
            if !jwt_claim_field_type_supported(&field_type) {
                self.error(
                    "E3280",
                    format!(
                        "JWT claim field `{}` uses unsupported type `{}`; supported fields are `string`, `boolean`, `decimal`, finite `f32`/`f64`, and `i32`/`i64`/`u32`/`u64`",
                        field.name, field_type
                    ),
                    Some(method),
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_narrowed_block(
        &mut self,
        statements: &[Stmt],
        null_narrowing: Option<&(String, bool)>,
        result_narrowing: Option<&ResultNarrowing>,
        typeof_narrowing: Option<&(String, String, bool)>,
        discriminant_narrowing: Option<&DiscriminantNarrowing>,
        is_narrowing: Option<&(String, Type, bool)>,
        then_branch: bool,
        definitely_executes: bool,
    ) {
        self.push_scope();
        if let Some((name, non_null_in_then)) = null_narrowing {
            if *non_null_in_then == then_branch && name.contains('.') {
                self.narrow_field_path(name);
            }
            if *non_null_in_then == then_branch
                && let Some(symbol) = self.lookup(name).cloned()
                && let ValueType::Known(Type::Optional(inner)) = symbol.ty.clone()
            {
                self.declare(
                    name,
                    Symbol {
                        ty: ValueType::Known(*inner),
                        ..symbol
                    },
                );
            }
        }
        if let Some(narrowing) = result_narrowing {
            if narrowing.success_in_then == then_branch {
                if let Some(symbol) = self.lookup(&narrowing.name).cloned()
                    && let ValueType::Known(Type::Result { success, .. }) = symbol.ty
                {
                    self.declare(
                        &narrowing.name,
                        Symbol {
                            ty: ValueType::Known(*success),
                            ..symbol
                        },
                    );
                }
            } else if let Some(symbol) = self.lookup(&narrowing.name).cloned()
                && let ValueType::Known(Type::Result { error, .. }) = symbol.ty
            {
                self.declare(
                    &narrowing.name,
                    Symbol {
                        ty: ValueType::Known(*error),
                        ..symbol
                    },
                );
            }
        }
        if let Some((name, type_name, is_type_in_then)) = typeof_narrowing
            && let Some(symbol) = self.lookup(name).cloned()
            && let ValueType::Known(current_ty) = &symbol.ty
            && let Some(narrowed_ty) =
                narrowed_typeof_type(current_ty, type_name, *is_type_in_then == then_branch)
        {
            self.declare(
                name,
                Symbol {
                    ty: ValueType::Known(narrowed_ty),
                    ..symbol
                },
            );
        }
        if let Some(narrowing) = discriminant_narrowing
            && let Some(symbol) = self.lookup(&narrowing.name).cloned()
            && let ValueType::Known(current_ty) = &symbol.ty
            && let Some(narrowed_ty) = self.narrowed_discriminant_type(
                current_ty,
                &narrowing.field,
                &narrowing.literal,
                narrowing.matches_in_then == then_branch,
            )
        {
            self.declare(
                &narrowing.name,
                Symbol {
                    ty: ValueType::Known(narrowed_ty),
                    ..symbol
                },
            );
        }
        if let Some((name, target_type, narrow_in_then)) = is_narrowing {
            if *narrow_in_then == then_branch {
                if let Some(symbol) = self.lookup(name).cloned()
                    && let ValueType::Known(current_ty) = &symbol.ty
                {
                    let narrowed = self
                        .narrowed_error_subtype(current_ty, target_type)
                        .or_else(|| narrowed_is_type(current_ty, target_type, true));
                    if let Some(narrowed_ty) = narrowed {
                        self.declare(
                            name,
                            Symbol {
                                ty: ValueType::Known(narrowed_ty),
                                ..symbol
                            },
                        );
                    }
                }
            } else {
                if let Some(symbol) = self.lookup(name).cloned()
                    && let ValueType::Known(current_ty) = &symbol.ty
                    && let Some(narrowed_ty) = narrowed_is_type(current_ty, target_type, false)
                {
                    self.declare(
                        name,
                        Symbol {
                            ty: ValueType::Known(narrowed_ty),
                            ..symbol
                        },
                    );
                }
            }
        }
        if !definitely_executes {
            self.branch_depth += 1;
        }
        for statement in statements {
            self.check_stmt(statement);
        }
        if !definitely_executes {
            self.branch_depth -= 1;
        }
        self.pop_scope();
    }

    fn narrowed_discriminant_type(
        &self,
        current: &Type,
        field: &str,
        literal: &Expr,
        matches_branch: bool,
    ) -> Option<Type> {
        let variants = self.union_variants(current)?;
        let narrowed = variants
            .into_iter()
            .filter(|variant| {
                variant_discriminant_matches(variant, field, literal) == matches_branch
            })
            .collect::<Vec<_>>();
        type_from_narrowed_variants(narrowed)
    }

    fn union_variants(&self, ty: &Type) -> Option<Vec<Type>> {
        match transparent_type(ty) {
            Type::Union(variants) => Some(variants.clone()),
            Type::Named(name) => match self.type_aliases.get(name).map(transparent_type) {
                Some(Type::Union(variants)) => Some(variants.clone()),
                _ => None,
            },
            _ => None,
        }
    }

    fn check_conversion_builtin(&mut self, callee: &str, args: &[Expr]) -> Option<ValueType> {
        let (arity, arg_ty, ret): (usize, Option<Type>, Type) = match callee {
            "String" => (1, None, Type::Str),
            "Number" | "parseFloat" => (1, Some(Type::Str), Type::Optional(Box::new(Type::F64))),
            "parseInt" => (1, Some(Type::Str), Type::Optional(Box::new(Type::I64))),
            "parseDecimal" => (1, Some(Type::Str), Type::Optional(Box::new(Type::Decimal))),
            _ => return None,
        };
        if args.len() != arity {
            self.error(
                "E0061",
                format!(
                    "`{callee}` expects {arity} argument(s), but {} were provided",
                    args.len()
                ),
                Some(callee),
            );
        }
        for arg in args {
            let actual = self.check_expr(arg);
            match &arg_ty {
                Some(expected) => {
                    self.require_assignable(expected, &actual, callee, "E0308");
                }
                None => {
                    let ok = matches!(
                        transparent_value_type(&actual),
                        Some(Type::Str | Type::Bool)
                    ) || matches!(&actual, ValueType::Known(ty) if is_integer_type(transparent_type(ty)) || is_float_type(transparent_type(ty)))
                        || matches!(
                            actual,
                            ValueType::IntegerLiteral
                                | ValueType::NegativeIntegerLiteral
                                | ValueType::UnsignedIntegerLiteral
                                | ValueType::FloatLiteral
                        );
                    if !ok && !matches!(actual, ValueType::Error | ValueType::Unknown) {
                        self.error(
                            "E0277",
                            format!(
                                "`String` accepts a primitive value, found `{}`",
                                actual.display()
                            ),
                            expression_subject(arg),
                        );
                    }
                }
            }
        }
        Some(ValueType::Known(ret))
    }

    fn check_float_predicate_builtin(&mut self, callee: &str, args: &[Expr]) -> Option<ValueType> {
        if callee != "isNaN" {
            return None;
        }
        if args.len() != 1 {
            self.error(
                "E0061",
                format!(
                    "`isNaN` expects 1 argument, but {} were provided",
                    args.len()
                ),
                Some(callee),
            );
        }
        for arg in args {
            let actual = self.check_expr(arg);
            let is_float = matches!(actual, ValueType::FloatLiteral)
                || matches!(&actual, ValueType::Known(ty) if is_float_type(transparent_type(ty)));
            if !is_float && !matches!(actual, ValueType::Error | ValueType::Unknown) {
                self.error(
                    "E0277",
                    format!(
                        "`isNaN` expects a floating-point value, found `{}`",
                        actual.display()
                    ),
                    expression_subject(arg),
                );
            }
        }
        Some(ValueType::Known(Type::Bool))
    }

    fn check_assert_builtin(&mut self, callee: &str, args: &[Expr]) -> Option<ValueType> {
        match callee {
            "assert" => {
                if args.is_empty() || args.len() > 2 {
                    self.error(
                        "E0061",
                        format!(
                            "`assert` expects 1 or 2 arguments, but {} were provided",
                            args.len()
                        ),
                        Some(callee),
                    );
                }
                if let Some(condition) = args.first() {
                    let actual = self.check_expr(condition);
                    self.require_assignable(&Type::Bool, &actual, callee, "E0308");
                }
                if let Some(message) = args.get(1) {
                    let actual = self.check_expr(message);
                    self.require_assignable(&Type::Str, &actual, callee, "E0308");
                }
                Some(ValueType::Known(Type::Void))
            }
            "assertEqual" => {
                if args.len() != 2 {
                    self.error(
                        "E0061",
                        format!(
                            "`assertEqual` expects 2 arguments, but {} were provided",
                            args.len()
                        ),
                        Some(callee),
                    );
                }
                if let (Some(left_expr), Some(right_expr)) = (args.first(), args.get(1)) {
                    let left = self.check_expr(left_expr);
                    let left_ty = value_type_to_type(&left).unwrap_or(Type::Unknown);
                    if !is_collection_key_type(&left_ty)
                        && !matches!(left, ValueType::Error | ValueType::Unknown)
                    {
                        self.check_expr(right_expr);
                        self.error(
                            "E0277",
                            format!(
                                "`assertEqual` compares scalar values, found `{}`",
                                left.display()
                            ),
                            expression_subject(left_expr),
                        );
                    } else {
                        let actual = self.check_expr_as(right_expr, &left_ty);
                        self.require_assignable(&left_ty, &actual, callee, "E0308");
                    }
                }
                Some(ValueType::Known(Type::Void))
            }
            "assertThrows" => {
                if args.is_empty() || args.len() > 2 {
                    self.error(
                        "E0061",
                        format!(
                            "`assertThrows` expects 1 or 2 arguments, but {} were provided",
                            args.len()
                        ),
                        Some(callee),
                    );
                    return Some(ValueType::Known(Type::Void));
                }
                let thrown = self.assert_throws_thrown_type(&args[0], "assertThrows");
                if let (Some(expected), Some(thrown)) = (args.get(1), thrown.as_ref()) {
                    let actual = self.check_expr(expected);
                    if matches!(thrown, Type::Str) {
                        self.require_assignable(&Type::Str, &actual, callee, "E0308");
                    } else {
                        self.error(
                            "E0308",
                            format!(
                                "`assertThrows` can only compare a message when the value thrown is a `string`, but `{thrown}` is thrown"
                            ),
                            expression_subject(&args[0]),
                        );
                    }
                }
                Some(ValueType::Known(Type::Void))
            }
            _ => None,
        }
    }

    fn check_print_builtin(&mut self, callee: &str, args: &[Expr]) -> Option<ValueType> {
        if !matches!(
            callee,
            "print" | "printError" | "printWarn" | "printRaw" | "printCR"
        ) {
            return None;
        }
        for arg in args {
            let ty = self.check_expr(arg);
            if !is_console_value(&ty) {
                self.error(
                    "E0277",
                    format!("`{callee}` does not support type `{}`", ty.display()),
                    expression_subject(arg),
                );
            }
        }
        Some(ValueType::Known(Type::Void))
    }

    fn assert_throws_thrown_type(&mut self, expr: &Expr, subject: &str) -> Option<Type> {
        let ExprKind::Ident(name) = expr.kind() else {
            self.error_on_expr(
                "E0277",
                format!("`{subject}` expects the name of a function that declares `throws`"),
                expr,
            );
            return None;
        };
        self.mark_used(name);
        if let Some(Type::Function { params, .. }) = self
            .lookup(name)
            .and_then(|symbol| value_type_to_type(&symbol.ty))
            && !params.is_empty()
        {
            self.error_on_expr(
                "E0061",
                format!(
                    "`{subject}` calls `{name}` with no arguments, but it requires {}",
                    params.len()
                ),
                expr,
            );
            return None;
        }
        let effects = self.fn_throws.get(name).cloned().unwrap_or_default();
        match effects.len() {
            0 => {
                self.error_on_expr(
                    "E0277",
                    format!(
                        "`{name}` does not declare `throws`, so it cannot be asserted to throw"
                    ),
                    expr,
                );
                None
            }
            1 => Some(effects[0].clone()),
            _ => {
                self.error_on_expr(
                    "E0277",
                    format!(
                        "`{subject}` needs a single thrown type, but `{name}` declares {}",
                        effects.len()
                    ),
                    expr,
                );
                None
            }
        }
    }

    fn numeric_result(
        &mut self,
        operator: &str,
        left: &ValueType,
        right: &ValueType,
        left_expr: &Expr,
        right_expr: &Expr,
    ) -> ValueType {
        if !is_numeric(left) || !is_numeric(right) {
            return self.binary_mismatch(operator, left, right, left_expr, right_expr);
        }
        match (left, right) {
            (
                ValueType::IntegerLiteral | ValueType::NegativeIntegerLiteral,
                ValueType::IntegerLiteral | ValueType::NegativeIntegerLiteral,
            ) => ValueType::Known(Type::I32),
            (ValueType::UnsignedIntegerLiteral, ValueType::UnsignedIntegerLiteral) => {
                ValueType::Known(Type::U128)
            }
            (ValueType::FloatLiteral, ValueType::FloatLiteral) => ValueType::Known(Type::F64),
            (ValueType::Known(left), ValueType::Known(right))
                if transparent_type(left) == transparent_type(right) =>
            {
                ValueType::Known(transparent_type(left).clone())
            }
            (ValueType::Known(ty), ValueType::IntegerLiteral)
            | (ValueType::IntegerLiteral, ValueType::Known(ty))
                if is_integer_type(transparent_type(ty)) =>
            {
                ValueType::Known(transparent_type(ty).clone())
            }
            (ValueType::Known(ty), ValueType::NegativeIntegerLiteral)
            | (ValueType::NegativeIntegerLiteral, ValueType::Known(ty))
                if matches!(
                    transparent_type(ty),
                    Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::Isize
                ) =>
            {
                ValueType::Known(transparent_type(ty).clone())
            }
            (ValueType::Known(ty), ValueType::UnsignedIntegerLiteral)
            | (ValueType::UnsignedIntegerLiteral, ValueType::Known(ty))
                if matches!(transparent_type(ty), Type::U128) =>
            {
                ValueType::Known(Type::U128)
            }
            (ValueType::Known(ty), ValueType::FloatLiteral)
            | (ValueType::FloatLiteral, ValueType::Known(ty))
                if is_float_type(transparent_type(ty)) =>
            {
                ValueType::Known(transparent_type(ty).clone())
            }
            (ValueType::Unknown, _) | (_, ValueType::Unknown) => ValueType::Unknown,
            (ValueType::Error, _) | (_, ValueType::Error) => ValueType::Error,
            _ => self.binary_mismatch(operator, left, right, left_expr, right_expr),
        }
    }

    fn check_compound_assignment(
        &mut self,
        name: &str,
        target: &Type,
        value: &ValueType,
        op: &crate::ast::AssignOp,
    ) {
        if matches!(
            op,
            crate::ast::AssignOp::AndAssign | crate::ast::AssignOp::OrAssign
        ) {
            if !matches!(target, Type::Bool) {
                self.error(
                    "E0368",
                    format!("logical assignment is not supported for `{name}`"),
                    Some(name),
                );
                return;
            }
            self.require_assignable(&Type::Bool, value, name, "E0308");
            return;
        }
        if matches!(op, crate::ast::AssignOp::NullCoalesceAssign) {
            if !matches!(target, Type::Optional(_)) {
                self.error(
                    "E0368",
                    format!("nullish assignment is not supported for `{name}`"),
                    Some(name),
                );
                return;
            }
            self.require_assignable(target, value, name, "E0308");
            return;
        }
        if matches!(
            op,
            crate::ast::AssignOp::BitAndAssign
                | crate::ast::AssignOp::BitOrAssign
                | crate::ast::AssignOp::BitXorAssign
                | crate::ast::AssignOp::ShlAssign
                | crate::ast::AssignOp::ShrAssign
                | crate::ast::AssignOp::UShrAssign
        ) {
            if !is_integer_type(target) {
                self.error(
                    "E0368",
                    format!("bitwise assignment is not supported for `{name}`"),
                    Some(name),
                );
                return;
            }
            self.require_assignable(target, value, name, "E0308");
            return;
        }
        if matches!(op, crate::ast::AssignOp::AddAssign)
            && is_string(&ValueType::known(target))
            && is_string(value)
        {
            return;
        }
        if !is_numeric(&ValueType::known(target)) || !is_numeric(value) {
            self.error(
                "E0368",
                format!("compound assignment is not supported for `{name}`"),
                Some(name),
            );
            return;
        }
        self.require_assignable(target, value, name, "E0308");
    }

    fn require_assignable(
        &mut self,
        expected: &Type,
        actual: &ValueType,
        subject: &str,
        code: &'static str,
    ) {
        self.require_assignable_with_range_preference(expected, actual, subject, code, false);
    }

    fn require_assignable_on_declaration(
        &mut self,
        expected: &Type,
        actual: &ValueType,
        subject: &str,
        code: &'static str,
    ) {
        self.require_assignable_with_range_preference(expected, actual, subject, code, true);
    }

    fn require_assignable_with_range_preference(
        &mut self,
        expected: &Type,
        actual: &ValueType,
        subject: &str,
        code: &'static str,
        prefer_declaration: bool,
    ) {
        if let ValueType::Known(Type::Throws { effects, .. }) = actual {
            let declared = match expected {
                Type::Throws { effects, .. } => effects.clone(),
                _ => Vec::new(),
            };
            let undeclared = effects
                .iter()
                .filter(|effect| !declared.contains(effect))
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            if !undeclared.is_empty()
                && matches!(
                    transparent_type(expected),
                    Type::Function { .. } | Type::GenericFunction { .. }
                )
            {
                let undeclared = undeclared.join(" | ");
                self.error_on_declaration(
                    "E3298",
                    format!(
                        "a function that throws `{undeclared}` cannot be used as `{expected}`, which does not declare it; write the type as `{expected} throws {undeclared}`"
                    ),
                    Some(subject),
                );
                return;
            }
        }
        if let Some(found) = self.unknown_unrepresentable_value(expected, actual) {
            let suggestion = match actual {
                ValueType::Known(_) => {
                    format!("give `{subject}` a type that describes the value, such as `{found}`")
                }
                _ => format!("give `{subject}` a type that describes the value"),
            };
            self.error_on_declaration_with_help(
                "E0794",
                format!("cannot store a value of type `{found}` in `unknown`"),
                Some(subject),
                vec![
                    format!("`unknown` holds only `string`, `boolean`, `char`, numeric and `decimal` values, and `{found}` is none of them"),
                    suggestion,
                ],
            );
            return;
        }
        if let Some(extra) = self.excess_object_field(expected, actual) {
            self.error(
                "E2353",
                format!("object literal specifies unknown property `{extra}`"),
                Some(&extra),
            );
            return;
        }
        if let Some((field, expected_ty, actual_ty)) =
            self.object_field_type_mismatch(expected, actual)
        {
            self.error(
                code,
                format!(
                    "type mismatch for `{field}`: expected `{expected_ty}`, found `{}`",
                    actual_ty.display()
                ),
                Some(&field),
            );
            return;
        }
        if let Some((target, missing)) = self.missing_object_field(expected, actual) {
            self.error_on_declaration(
                "E2741",
                format!("property `{missing}` is missing in object literal for type `{target}`"),
                Some(subject),
            );
            return;
        }
        if !self.is_assignable(expected, actual) {
            if matches!(actual, ValueType::NegativeIntegerLiteral)
                && is_unsigned_integer_type(transparent_type(expected))
            {
                self.error_on_declaration(
                    code,
                    format!(
                        "negative integer literal cannot be assigned to unsigned type `{expected}`; use `i32`, `i64`, `i128`, or `isize` for negative values"
                    ),
                    Some(subject),
                );
                return;
            }
            if let (Type::Named(expected_name), ValueType::Known(Type::Named(actual_name))) =
                (transparent_type(expected), actual)
                && expected_name != actual_name
                && self.interface_structurally_satisfies(actual_name, expected_name)
            {
                self.error_on_declaration(
                        code,
                        format!(
                            "`{actual_name}` is not assignable to `{expected_name}`: tyrus interfaces are nominal, so distinct interface names are separate types even with the same fields; build a `{expected_name}` value directly (its fields are available on `{actual_name}`), or use one shared interface type for both"
                        ),
                        Some(subject),
                    );
                return;
            }
            let message = format!(
                "type mismatch for `{subject}`: expected `{expected}`, found `{}`",
                actual.display()
            );
            if prefer_declaration {
                self.error_on_declaration(code, message, Some(subject));
            } else {
                self.error(code, message, Some(subject));
            }
        }
    }

    fn interface_structurally_satisfies(&self, actual_name: &str, expected_name: &str) -> bool {
        match (
            self.interfaces.get(expected_name),
            self.interfaces.get(actual_name),
        ) {
            (Some(expected_fields), Some(actual_fields)) => {
                object_type_fields_assignable(expected_fields, actual_fields)
            }
            _ => false,
        }
    }

    fn require_bool(&mut self, actual: &ValueType, context: &str) {
        if !self.is_assignable(&Type::Bool, actual)
            && !is_optional_value(actual)
            && !is_result_value(actual)
        {
            self.error(
                "E0308",
                format!(
                    "{context} must have type `boolean`, found `{}`",
                    actual.display()
                ),
                expression_subject_name(context),
            );
        }
    }

    fn require_mutable_numeric_target(&mut self, target: &Expr, ty: &ValueType) {
        let name = expression_subject(target);
        if let Some(name) = name {
            match self.lookup(name) {
                None => {
                    self.error(
                        "E0425",
                        format!("cannot assign to undeclared identifier `{name}`"),
                        Some(name),
                    );
                }
                Some(sym) if !sym.mutable => {
                    self.error(
                        "E0384",
                        format!("cannot mutate immutable variable `{name}` through `++`/`--`"),
                        Some(name),
                    );
                }
                _ => {}
            }
        }
        if !is_numeric(ty) {
            self.error(
                "E0308",
                format!(
                    "operand of `++`/`--` must be a number, found `{}`",
                    ty.display()
                ),
                expression_subject(target),
            );
        }
    }

    fn is_struct_like(&self, value: &ValueType) -> bool {
        match value {
            ValueType::Known(ty) => match transparent_type(ty) {
                Type::Object(_) => true,
                Type::Named(name) => {
                    self.interfaces.contains_key(name)
                        || matches!(self.type_aliases.get(name), Some(Type::Object(_)))
                }
                _ => false,
            },
            ValueType::Object(_) => true,
            _ => false,
        }
    }

    fn known_array_len(&self, object: &Expr, object_ty: &ValueType) -> Option<usize> {
        if let ExprKind::Ident(name) = object.kind()
            && let Some(len) = self.lookup(name).and_then(|symbol| symbol.array_len)
        {
            return Some(len);
        }
        match object_ty {
            ValueType::Array(values) => Some(values.len()),
            _ => None,
        }
    }

    fn is_length_volatile(&self) -> bool {
        self.branch_depth > 0 || self.fn_depth > 1
    }

    fn known_integer_condition(&self, expr: &Expr) -> Option<bool> {
        match expr.kind() {
            ExprKind::BoolLit(value) => Some(*value),
            ExprKind::BinOp { left, op, right } => {
                let left = self.known_integer_value(left)?;
                let right = self.known_integer_value(right)?;
                match op {
                    crate::ast::BinOp::Eq => Some(left == right),
                    crate::ast::BinOp::Neq => Some(left != right),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn known_integer_value(&self, expr: &Expr) -> Option<i128> {
        match expr.kind() {
            ExprKind::IntLit(value) => Some(*value),
            ExprKind::UIntLit(value) => i128::try_from(*value).ok(),
            ExprKind::Neg(inner) => self.known_integer_value(inner).and_then(i128::checked_neg),
            ExprKind::Ident(name) => self
                .constant_integers
                .iter()
                .rev()
                .find_map(|scope| scope.get(name))
                .copied()
                .flatten(),
            _ => None,
        }
    }

    fn record_constant_integer(&mut self, name: &str, mutable: bool, value: Option<&Expr>) {
        let value = (!mutable)
            .then(|| value.and_then(|value| self.known_integer_value(value)))
            .flatten();
        self.constant_integers
            .last_mut()
            .expect("the root constant scope is always present")
            .insert(name.to_string(), value);
    }

    fn array_length_snapshot(&self) -> Vec<HashMap<String, Option<usize>>> {
        self.scopes
            .iter()
            .map(|scope| {
                scope
                    .iter()
                    .map(|(name, symbol)| (name.clone(), symbol.array_len))
                    .collect()
            })
            .collect()
    }

    fn restore_array_lengths(&mut self, snapshot: &[HashMap<String, Option<usize>>]) {
        debug_assert_eq!(self.scopes.len(), snapshot.len());
        for (scope, lengths) in self.scopes.iter_mut().zip(snapshot) {
            for (name, symbol) in scope {
                if let Some(length) = lengths.get(name) {
                    symbol.array_len = *length;
                }
            }
        }
    }

    fn set_array_len(&mut self, name: &str, len: Option<usize>) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbol) = scope.get_mut(name) {
                symbol.array_len = len;
                return;
            }
        }
    }

    fn invalidate_array_len(&mut self, name: &str) {
        self.set_array_len(name, None);
    }

    fn initial_array_len(&self, declared_ty: &Type, value: &Expr) -> Option<usize> {
        let declared_ty = transparent_type(declared_ty);
        let array_declared = match declared_ty {
            Type::Array(_) => true,
            Type::Optional(inner) => matches!(transparent_type(inner), Type::Array(_)),
            _ => false,
        };
        if !array_declared {
            return None;
        }
        match value.kind() {
            ExprKind::ArrayLit(items)
                if !items
                    .iter()
                    .any(|item| matches!(item.kind(), ExprKind::Spread(_))) =>
            {
                Some(items.len())
            }
            ExprKind::Unwrap(inner) => self.known_array_len(inner, &ValueType::Unknown),
            ExprKind::MethodCall {
                object,
                method,
                args,
                ..
            } => regex_match_result_len(object, method, args).or_else(|| {
                if LENGTH_PRESERVING_RESULT_METHODS.contains(&method.as_str()) {
                    if let ExprKind::Ident(receiver) = object.as_ref().kind() {
                        self.lookup(receiver).and_then(|symbol| symbol.array_len)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }),
            _ => None,
        }
    }

    fn invalidate_loop_mutations(&mut self, body: &[Stmt]) {
        let mut names = HashSet::default();
        walk_block_mutations(body, &mut names);
        for name in names {
            self.invalidate_array_len(&name);
        }
    }

    fn check_constant_index_bounds(&mut self, index: &Expr, len: usize, subject: Option<&str>) {
        let value = match index.kind() {
            ExprKind::IntLit(value) => *value,
            ExprKind::Neg(inner) => match inner.as_ref() {
                Expr {
                    kind: ExprKind::IntLit(value),
                    ..
                } => -value,
                _ => return,
            },
            _ => return,
        };
        if value >= 0 && (value as usize) < len {
            return;
        }
        let detail = if len == 0 {
            "the array is empty".to_string()
        } else {
            format!("valid indices are 0 to {}", len - 1)
        };
        let subject = subject.map(str::to_string);
        self.error_on_declaration(
            "E0606",
            format!("array index {value} is out of bounds: {detail} (length {len})"),
            subject.as_deref(),
        );
    }

    fn check_split_at_bounds(&mut self, index: &Expr, len: usize) {
        let value = match index.kind() {
            ExprKind::IntLit(value) => *value,
            ExprKind::Neg(inner) => match inner.as_ref() {
                Expr {
                    kind: ExprKind::IntLit(value),
                    ..
                } => -value,
                _ => return,
            },
            _ => return,
        };
        if value >= 0 && (value as usize) <= len {
            return;
        }
        self.error_at(
            "E0606",
            format!("splitAt index {value} is out of bounds: valid indices are 0 to {len} (length {len})"),
            index.span,
        );
    }

    fn check_sample_bounds(&mut self, count: &Expr, len: usize) {
        let ExprKind::IntLit(value) = count.kind() else {
            return;
        };
        if *value > 0 && (*value as usize) <= len {
            return;
        }
        self.error_at(
            "E0606",
            format!(
                "sample count {value} is out of bounds: valid counts are 1 to {len} (length {len})"
            ),
            count.span,
        );
    }

    fn check_random_array_minimum_length(&mut self, method: &str, receiver: &Expr, len: usize) {
        let minimum = if method == "shuffle" { 2 } else { 1 };
        if len >= minimum {
            return;
        }
        self.error_at(
            "E0606",
            format!("array.{method} requires at least {minimum} element(s), found {len}"),
            receiver.span,
        );
    }

    fn has_symbol(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|scope| scope.contains_key(name))
    }

    fn mark_used(&mut self, name: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbol) = scope.get_mut(name) {
                symbol.used = true;
                return;
            }
        }
    }

    fn is_style_text_ident(&self, name: &str) -> bool {
        name == "styleText"
            && matches!(
                self.lookup(name).map(|symbol| &symbol.ty),
                Some(ValueType::Known(Type::Named(marker)))
                    if marker == crate::style_text::STYLE_TEXT_MARKER
            )
    }

    fn check_style_text(&mut self, expr: &Expr) -> Option<ValueType> {
        let form = {
            let is_style_text = |name: &str| self.is_style_text_ident(name);
            crate::style_text::as_style_text(expr, &is_style_text)
        };
        let Some(form) = form else {
            let incomplete = {
                let is_style_text = |name: &str| self.is_style_text_ident(name);
                crate::style_text::is_incomplete_style_text(expr, &is_style_text)
            };
            if incomplete {
                self.mark_used("styleText");
                self.error(
                    "E3271",
                    "`styleText` must be called with the text to style".to_string(),
                    Some("styleText"),
                );
                return Some(ValueType::Error);
            }
            return None;
        };
        self.mark_used("styleText");
        match form {
            crate::style_text::StyleTextForm::Styled { styles, text } => {
                for style in &styles {
                    if !crate::style_text::is_known_style(style) {
                        self.error(
                            "E3273",
                            format!("unknown text style `{style}`"),
                            Some(style),
                        );
                    }
                }
                let text_ty = self.check_expr(text);
                if !matches!(transparent_value_type(&text_ty), Some(Type::Str))
                    && !matches!(text_ty, ValueType::Error | ValueType::Unknown)
                {
                    self.error(
                        "E0308",
                        format!(
                            "`styleText` text must be a `string`, found `{}`",
                            text_ty.display()
                        ),
                        expression_subject(text),
                    );
                }
                Some(ValueType::Known(Type::Str))
            }
            crate::style_text::StyleTextForm::Arity => {
                self.error(
                    "E3270",
                    "`styleText` takes a format and the text to style".to_string(),
                    Some("styleText"),
                );
                Some(ValueType::Error)
            }
            crate::style_text::StyleTextForm::NonLiteralStyle => {
                self.error(
                    "E3272",
                    "`styleText` styles must be string literals".to_string(),
                    Some("styleText"),
                );
                Some(ValueType::Error)
            }
        }
    }

    fn is_figlet_ident(&self, name: &str) -> bool {
        name == "figlet"
            && matches!(
                self.lookup(name).map(|symbol| &symbol.ty),
                Some(ValueType::Known(Type::Named(marker)))
                    if marker == crate::figlet::FIGLET_MARKER
            )
    }

    fn check_figlet(&mut self, expr: &Expr) -> Option<ValueType> {
        let form = {
            let is_figlet = |name: &str| self.is_figlet_ident(name);
            crate::figlet::as_figlet(expr, &is_figlet)
        };
        let Some(form) = form else {
            let incomplete = {
                let is_figlet = |name: &str| self.is_figlet_ident(name);
                crate::figlet::is_incomplete_figlet(expr, &is_figlet)
            };
            if incomplete {
                self.mark_used("figlet");
                self.error(
                    "E3275",
                    "`figlet` must be called with the text to render".to_string(),
                    Some("figlet"),
                );
                return Some(ValueType::Error);
            }
            return None;
        };
        self.mark_used("figlet");
        match form {
            crate::figlet::FigletForm::Rendered { text, font } => {
                if let Some(font) = font
                    && !crate::figlet::is_known_font(font)
                {
                    self.error("E3277", format!("unknown figlet font `{font}`"), Some(font));
                }
                let text_ty = self.check_expr(text);
                if !matches!(transparent_value_type(&text_ty), Some(Type::Str))
                    && !matches!(text_ty, ValueType::Error | ValueType::Unknown)
                {
                    self.error(
                        "E0308",
                        format!(
                            "`figlet` text must be a `string`, found `{}`",
                            text_ty.display()
                        ),
                        expression_subject(text),
                    );
                }
                Some(ValueType::Known(Type::Str))
            }
            crate::figlet::FigletForm::Arity => {
                self.error(
                    "E3274",
                    "`figlet` takes the text and an optional font".to_string(),
                    Some("figlet"),
                );
                Some(ValueType::Error)
            }
            crate::figlet::FigletForm::NonLiteralFont(font) => {
                self.error(
                    "E3276",
                    "`figlet` font must be a string literal".to_string(),
                    expression_subject(font),
                );
                Some(ValueType::Error)
            }
        }
    }

    fn is_process_env(&self, expr: &Expr) -> bool {
        matches!(
            expr,
            Expr { kind: ExprKind::FieldAccess { object, field }, .. }
                if field == "env"
                    && matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "process")
        ) && !self.has_symbol("process")
    }

    fn read_symbol(&mut self, name: &str) -> ValueType {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbol) = scope.get_mut(name) {
                symbol.used = true;
                return symbol.ty.clone();
            }
        }
        if name == "process" {
            self.error(
                "E3145",
                "`process` cannot be used as a value; only `process.env`, `process.argv`, `process.pid`, `process.ppid`, `process.exit`, `process.cwd`, `process.uptime`, `process.memoryUsage`, and `process.cpuUsage` are available"
                    .to_string(),
                None,
            );
            return ValueType::Error;
        }
        if let Some(ty) = native_function_type(name) {
            return ValueType::Known(ty);
        }
        if self
            .unresolved_namespaces
            .iter()
            .any(|prefix| name.starts_with(prefix))
        {
            return ValueType::Unknown;
        }
        if let Some(display) = crate::module_loader::namespace_member_display(name) {
            let (namespace, member) = display.split_once('.').unwrap_or((&display, ""));
            self.error(
                "E0425",
                format!("module namespace `{namespace}` has no export `{member}`"),
                Some(name),
            );
            return ValueType::Error;
        }
        if name == "panic" {
            self.error_at_with_help(
                "E0425",
                "`panic` can only be used as a statement, like `panic(\"message\");`".to_string(),
                self.current_span().unwrap_or(Span::DUMMY),
                vec!["move the call onto its own line instead of using its result".to_string()],
            );
            return ValueType::Error;
        }
        self.error_at_with_subject(
            "E0425",
            format!("use of undeclared identifier `{name}`"),
            self.current_span().unwrap_or(Span::DUMMY),
            name.to_string(),
        );
        ValueType::Error
    }

    fn check_type_assertion(&mut self, source: &ValueType, target: &Type, value: &Expr) {
        self.check_type_name(target, value.span);
        if matches!(source, ValueType::Error | ValueType::Unknown)
            || matches!(target, Type::Infer | Type::Generic(_))
        {
            return;
        }
        if matches!(source, ValueType::Object(_))
            && matches!(transparent_type(target), Type::Named(_) | Type::Object(_))
        {
            let target_display = target.to_string();
            self.error_at(
                "E0605",
                format!(
                    "cannot build a `{target_display}` with `as`: an object literal takes its type from the declaration it initializes, and `as` converts a value rather than building one"
                ),
                value.span,
            );
            return;
        }
        let converted = self.enum_repr_of(source).unwrap_or_else(|| source.clone());
        let numeric_source = value_type_is_numeric(&converted);
        let numeric_target = is_numeric_type(transparent_type(target));
        let fallible_target = match transparent_type(target) {
            Type::Optional(inner) => is_numeric_type(transparent_type(inner)),
            _ => false,
        };
        let optional_numeric_source = match &converted {
            ValueType::Known(ty) => match transparent_type(ty) {
                Type::Optional(inner) => is_numeric_type(transparent_type(inner)),
                _ => false,
            },
            _ => false,
        };
        let decimal_source = matches!(
            &converted,
            ValueType::Known(ty) if matches!(transparent_type(ty), Type::Decimal)
        );
        let float_source = matches!(
            &converted,
            ValueType::Known(ty) if is_float_type(transparent_type(ty))
        );
        let decimal_target = match transparent_type(target) {
            Type::Decimal => true,
            Type::Optional(inner) => matches!(transparent_type(inner), Type::Decimal),
            _ => false,
        };
        let float_target = match transparent_type(target) {
            Type::Optional(inner) => is_float_type(transparent_type(inner)),
            ty => is_float_type(ty),
        };
        if (decimal_source && float_target) || (float_source && decimal_target) {
            self.error_at(
                "E0605",
                format!(
                    "cannot convert `{}` to `{}` with `as`: conversions between `decimal` and binary floating-point types are not supported",
                    source.display(),
                    target
                ),
                value.span,
            );
            return;
        }
        let char_source = matches!(&converted, ValueType::Known(ty) if matches!(transparent_type(ty), Type::Char));
        let integer_source = matches!(&converted, ValueType::Known(ty) if is_integer_type(transparent_type(ty)))
            || matches!(
                converted,
                ValueType::IntegerLiteral | ValueType::UnsignedIntegerLiteral
            );
        let char_target = matches!(transparent_type(target), Type::Char);
        let fallible_char_target = match transparent_type(target) {
            Type::Optional(inner) => matches!(transparent_type(inner), Type::Char),
            _ => false,
        };
        let integer_target = is_integer_type(transparent_type(target));
        if (char_source && (integer_target || matches!(transparent_type(target), Type::Str)))
            || (integer_source && (char_target || fallible_char_target))
        {
            return;
        }
        if self.is_assignable(target, source)
            || self.is_assignable(target, &converted)
            || (numeric_source && (numeric_target || fallible_target))
            || (optional_numeric_source && fallible_target)
        {
            return;
        }
        let target_display = target.to_string();
        let source_display = source.display();
        let reason = if numeric_source || numeric_target {
            format!(
                "`as` converts between numbers, and `{source_display}` and `{target_display}` are not both numeric"
            )
        } else {
            format!(
                "`as` converts, it does not reinterpret: a `{source_display}` value is not a `{target_display}`"
            )
        };
        self.error_at(
            "E0605",
            format!("cannot convert `{source_display}` to `{target_display}` with `as`: {reason}"),
            value.span,
        );
    }

    fn check_type_name(&mut self, ty: &Type, span: Span) -> bool {
        let mut missing = Vec::new();
        self.collect_unknown_type_names(ty, &mut missing);
        let resolved = missing.is_empty();
        for name in missing {
            if self.reachable_only_types.contains(&name) {
                self.error_at_with_help(
                    "E0412",
                    format!("cannot find type `{name}` in this scope"),
                    span,
                    vec![format!(
                        "`{name}` reaches this module through an imported interface; add `{name}` to the import from the module that declares it"
                    )],
                );
                continue;
            }
            match crate::module_loader::native_module_exporting(&name) {
                Some(module) => self.error_at_with_help(
                    "E0412",
                    format!("cannot find type `{name}` in this scope"),
                    span,
                    vec![format!(
                        "`{name}` is exported by `{module}`; add `import {{ {name} }} from \"{module}\";`"
                    )],
                ),
                None => self.error_at(
                    "E0412",
                    format!("cannot find type `{name}` in this scope"),
                    span,
                ),
            }
        }
        resolved
    }

    fn collect_unknown_type_names(&self, ty: &Type, missing: &mut Vec<String>) {
        match ty {
            Type::Named(name) | Type::Generic(name) => {
                if !self.type_name_exists(name) || self.reachable_only_types.contains(name) {
                    missing.push(name.clone());
                }
            }
            Type::Apply { name, args } => {
                if !self.type_name_exists(name) || self.reachable_only_types.contains(name) {
                    missing.push(name.clone());
                }
                for arg in args {
                    self.collect_unknown_type_names(arg, missing);
                }
            }
            Type::Array(inner)
            | Type::Optional(inner)
            | Type::Readonly(inner)
            | Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::KeyOf(inner) => self.collect_unknown_type_names(inner, missing),
            Type::Result { success, error } => {
                self.collect_unknown_type_names(success, missing);
                self.collect_unknown_type_names(error, missing);
            }
            Type::Tuple(items) | Type::Union(items) => {
                for item in items {
                    self.collect_unknown_type_names(item, missing);
                }
            }
            Type::Function { params, ret } => {
                for param in params {
                    self.collect_unknown_type_names(param, missing);
                }
                self.collect_unknown_type_names(ret, missing);
            }
            Type::GenericFunction { params, ret, .. } => {
                for param in params {
                    self.collect_unknown_type_names(param, missing);
                }
                self.collect_unknown_type_names(ret, missing);
            }
            Type::Throws { function, effects } => {
                self.collect_unknown_type_names(function, missing);
                for effect in effects {
                    self.collect_unknown_type_names(effect, missing);
                }
            }
            Type::Object(fields) => {
                for field in fields {
                    self.collect_unknown_type_names(&field.ty, missing);
                }
            }
            Type::Conditional {
                check,
                bound,
                then,
                otherwise,
            } => {
                self.collect_unknown_type_names(check, missing);
                self.collect_unknown_type_names(bound, missing);
                self.collect_unknown_type_names(then, missing);
                self.collect_unknown_type_names(otherwise, missing);
            }
            _ => {}
        }
    }

    fn type_name_exists(&self, name: &str) -> bool {
        self.interfaces.contains_key(name)
            || self.enums.contains_key(name)
            || self.type_aliases.contains_key(name)
            || self.generic_defs.is_defined(name)
            || self
                .type_param_names
                .iter()
                .any(|scope| scope.contains(name))
            || is_builtin_type_name(name)
    }

    fn enum_repr_of(&self, value: &ValueType) -> Option<ValueType> {
        let ValueType::Known(Type::Named(name)) = value else {
            return None;
        };
        let resolved = self.enums.get(name)?;
        Some(ValueType::Known(resolved.repr.clone()))
    }

    fn tuple_fills_array(&self, positions: &[Type], element: &Type) -> bool {
        !positions.is_empty()
            && positions
                .iter()
                .all(|position| self.is_assignable(element, &ValueType::Known(position.clone())))
    }

    fn is_assignable(&self, expected: &Type, actual: &ValueType) -> bool {
        if let ValueType::Known(Type::Throws { function, effects }) = actual {
            match expected {
                Type::Throws {
                    function: expected_function,
                    effects: expected_effects,
                } => {
                    return effects
                        .iter()
                        .all(|effect| expected_effects.contains(effect))
                        && self.is_assignable(
                            expected_function,
                            &ValueType::Known((**function).clone()),
                        );
                }
                _ if matches!(
                    transparent_type(expected),
                    Type::Function { .. } | Type::GenericFunction { .. }
                ) =>
                {
                    return false;
                }
                _ => {}
            }
        }
        if let Type::Throws { function, .. } = expected {
            return self.is_assignable(function, actual);
        }
        if let ValueType::Known(Type::Named(actual_name)) = actual
            && let Some(alias) = self.type_aliases.get(actual_name)
            && (transparent_type(expected) == transparent_type(alias)
                || self.is_assignable(expected, &ValueType::Known(alias.clone())))
        {
            return true;
        }
        if matches!(expected, Type::Ref(_) | Type::MutRef(_)) {
            return is_assignable(expected, actual);
        }
        if let Type::Optional(inner) = transparent_type(expected) {
            return match actual {
                ValueType::Null => true,
                ValueType::Known(Type::Optional(_)) => is_assignable(expected, actual),
                _ => self.is_assignable(inner, actual),
            };
        }
        if let Type::Named(name) = transparent_type(expected) {
            if matches!(actual, ValueType::Known(Type::Named(actual)) if actual == name) {
                return true;
            }
            if let Some(alias) = self.type_aliases.get(name) {
                return self.is_assignable(alias, actual);
            }
            return match actual {
                ValueType::Error | ValueType::Unknown => true,
                ValueType::Known(Type::Named(actual)) => {
                    actual == name || crate::builtin_interfaces::interface_extends(actual, name)
                }
                ValueType::Known(Type::Object(fields)) => self
                    .interfaces
                    .get(name)
                    .is_some_and(|expected| object_type_fields_assignable(expected, fields)),
                ValueType::Object(values) => self
                    .interfaces
                    .get(name)
                    .is_some_and(|fields| self.object_fields_assignable(fields, values)),
                ValueType::Union(variants) => {
                    variants.iter().all(|v| self.is_assignable(expected, v))
                }
                _ => false,
            };
        }
        if let Type::Result {
            success: expected_success,
            error: expected_error,
        } = transparent_type(expected)
        {
            return match actual {
                ValueType::Known(Type::Result { success, error }) => {
                    self.is_assignable(expected_success, &ValueType::Known((**success).clone()))
                        && self.is_assignable(expected_error, &ValueType::Known((**error).clone()))
                }
                _ => false,
            };
        }
        if let Type::Union(variants) = transparent_type(expected) {
            if let ValueType::Known(Type::Union(actual_variants)) = actual {
                return actual_variants.iter().all(|actual_variant| {
                    variants.iter().any(|expected_variant| {
                        self.is_assignable(
                            expected_variant,
                            &ValueType::Known(actual_variant.clone()),
                        )
                    })
                });
            }
            return variants
                .iter()
                .any(|variant| self.is_assignable(variant, actual));
        }
        if let Type::Array(element) = transparent_type(expected) {
            let expected_is_readonly = matches!(expected, Type::Readonly(_));
            return match actual {
                ValueType::Error | ValueType::Unknown => true,
                ValueType::Array(values) => values
                    .iter()
                    .all(|value| self.is_assignable(element, value)),
                ValueType::Known(Type::Array(actual)) => {
                    transparent_type(element) == transparent_type(actual)
                }
                ValueType::Known(Type::Tuple(positions)) => {
                    self.tuple_fills_array(positions, element)
                }
                ValueType::Known(Type::Readonly(actual)) => {
                    expected_is_readonly
                        && match transparent_type(actual) {
                            Type::Array(actual_element) => {
                                transparent_type(element) == transparent_type(actual_element)
                            }
                            Type::Tuple(positions) => self.tuple_fills_array(positions, element),
                            _ => false,
                        }
                }
                _ => false,
            };
        }
        if let Type::Tuple(elements) = transparent_type(expected) {
            let expected_is_readonly = matches!(expected, Type::Readonly(_));
            return match actual {
                ValueType::Error | ValueType::Unknown => true,
                ValueType::Array(values) if values.len() == elements.len() => elements
                    .iter()
                    .zip(values)
                    .all(|(expected, actual)| self.is_assignable(expected, actual)),
                ValueType::Known(Type::Tuple(actual_elements))
                    if actual_elements.len() == elements.len() =>
                {
                    self.tuple_elements_assignable(elements, actual_elements)
                }
                ValueType::Known(Type::Readonly(actual)) => {
                    expected_is_readonly
                        && match transparent_type(actual) {
                            Type::Tuple(actual_elements)
                                if actual_elements.len() == elements.len() =>
                            {
                                self.tuple_elements_assignable(elements, actual_elements)
                            }
                            _ => false,
                        }
                }
                _ => false,
            };
        }
        if let Type::Object(expected_fields) = transparent_type(expected) {
            return match actual {
                ValueType::Known(Type::Named(name)) => {
                    self.interfaces.get(name).is_some_and(|actual_fields| {
                        object_type_fields_assignable(expected_fields, actual_fields)
                    })
                }
                ValueType::Known(Type::Object(actual_fields)) => {
                    self.known_object_fields_assignable(expected_fields, actual_fields)
                }
                ValueType::Object(values) => self.object_fields_assignable(expected_fields, values),
                _ => is_assignable(expected, actual),
            };
        }
        if let Some(expected_payload) = promise_payload(expected) {
            return match actual {
                ValueType::Error | ValueType::Unknown => true,
                ValueType::Known(actual_ty) => {
                    promise_payload(actual_ty).is_some_and(|actual_payload| {
                        self.is_assignable(
                            expected_payload,
                            &ValueType::Known(actual_payload.clone()),
                        )
                    })
                }
                _ => false,
            };
        }
        is_assignable(expected, actual)
    }

    fn tuple_elements_assignable(&self, expected: &[Type], actual: &[Type]) -> bool {
        expected.iter().zip(actual).all(|(expected, actual)| {
            self.is_assignable(expected, &ValueType::Known(actual.clone()))
        })
    }

    fn known_object_fields_assignable(
        &self,
        expected_fields: &[Field],
        actual_fields: &[Field],
    ) -> bool {
        expected_fields.iter().all(|expected| {
            actual_fields
                .iter()
                .find(|actual| actual.name == expected.name)
                .map(|actual| {
                    self.is_assignable(
                        &field_value_type(expected),
                        &ValueType::Known(field_value_type(actual)),
                    )
                })
                .unwrap_or(expected.optional)
        }) && actual_fields.iter().all(|actual| {
            expected_fields
                .iter()
                .any(|expected| expected.name == actual.name)
        })
    }

    fn field_is_readonly(&self, object_ty: &ValueType, field: &str) -> bool {
        if field == "value"
            && let ValueType::Known(ty) = object_ty
            && is_rwlock_read_guard_typed(ty)
        {
            return true;
        }
        let fields = match object_ty {
            ValueType::Known(Type::Named(name)) => self.interfaces.get(name),
            ValueType::Known(Type::Object(fields)) => Some(fields),
            _ => None,
        };
        fields
            .and_then(|fields| fields.iter().find(|candidate| candidate.name == field))
            .map(|candidate| candidate.readonly)
            .unwrap_or(false)
    }

    fn check_field_on_value(&mut self, object_ty: ValueType, field: &str) -> ValueType {
        let object_ty = match &object_ty {
            ValueType::Array(_) => match concrete_value_type(&object_ty) {
                Some(settled) => ValueType::Known(settled),
                None => object_ty,
            },
            _ => object_ty,
        };
        if let ValueType::Known(Type::Generic(name)) = &object_ty
            && let Some(constraint) = self.type_param_constraint(name)
        {
            return self.check_field_on_value(ValueType::Known(constraint), field);
        }
        if let ValueType::Known(Type::Readonly(inner)) = &object_ty {
            let field_ty = self.check_field_on_value(ValueType::Known((**inner).clone()), field);
            return match field_ty {
                ValueType::Known(ty) => ValueType::Known(deep_readonly(ty)),
                other => other,
            };
        }
        if let Some(Type::Apply { name, args }) = transparent_value_type(&object_ty) {
            if matches!(name.as_str(), "DuplexStream" | "TransformStream") && args.len() == 2 {
                let field_ty = match (name.as_str(), field) {
                    ("DuplexStream", "readable") => Some(Type::Apply {
                        name: "ReadableStream".to_string(),
                        args: vec![args[0].clone()],
                    }),
                    ("DuplexStream", "writable") => Some(Type::Apply {
                        name: "WritableStream".to_string(),
                        args: vec![args[1].clone()],
                    }),
                    ("TransformStream", "writable") => Some(Type::Apply {
                        name: "WritableStream".to_string(),
                        args: vec![args[0].clone()],
                    }),
                    ("TransformStream", "readable") => Some(Type::Apply {
                        name: "ReadableStream".to_string(),
                        args: vec![args[1].clone()],
                    }),
                    _ => None,
                };
                if let Some(field_ty) = field_ty {
                    return ValueType::Known(field_ty);
                }
            }
            if matches!(name.as_str(), "MutexGuard" | "RwReadGuard" | "RwWriteGuard")
                && args.len() == 1
                && field == "value"
            {
                return ValueType::Known(args[0].clone());
            }
        }
        if matches!(
            transparent_value_type(&object_ty),
            Some(
                Type::Map(_, _)
                    | Type::Set(_)
                    | Type::BinaryHeap(_)
                    | Type::BinomialHeap(_)
                    | Type::UnionFind(_)
                    | Type::Counter(_)
                    | Type::Deque(_)
            )
        ) && field == "size"
        {
            return ValueType::Known(Type::Usize);
        }
        if let Some(Type::Map(_, _)) = transparent_value_type(&object_ty) {
            self.error(
                    "E0609",
                    format!(
                        "no property `{field}` on a map; use `size`, or the `get`/`set`/`getOrInsert`/`has`/`delete`/`keys`/`values`/`entries` methods"
                    ),
                    Some(field),
                );
            return ValueType::Error;
        }
        if let Some(
            Type::Set(_)
            | Type::BinaryHeap(_)
            | Type::BinomialHeap(_)
            | Type::UnionFind(_)
            | Type::Counter(_)
            | Type::Deque(_)
            | Type::LinkedList(_)
            | Type::DoubleLinkedList(_)
            | Type::AvlTree(_)
            | Type::RedBlackTree(_)
            | Type::SplayTree(_)
            | Type::Treap(_)
            | Type::SortedSet(_)
            | Type::SkipList(_)
            | Type::SortedMap(_, _)
            | Type::Graph(_, _)
            | Type::FenwickTree
            | Type::BubbleSort
            | Type::SegmentTree
            | Type::SegmentTreeLazy
            | Type::SparseTable
            | Type::BitSet,
        ) = transparent_value_type(&object_ty)
        {
            self.error(
                    "E0609",
                    format!(
                        "no property `{field}` on a set; use `size`, or the `add`/`has`/`delete`/`values` methods"
                    ),
                    Some(field),
                );
            return ValueType::Error;
        }
        if matches!(transparent_value_type(&object_ty), Some(Type::Array(_)))
            && matches!(field, "length" | "capacity")
        {
            return ValueType::Known(Type::Usize);
        }
        if matches!(transparent_value_type(&object_ty), Some(Type::Tuple(_))) && field == "length" {
            return ValueType::Known(Type::Usize);
        }
        if matches!(transparent_value_type(&object_ty), Some(Type::Str)) && field == "length" {
            return ValueType::Known(Type::Usize);
        }
        if matches!(transparent_value_type(&object_ty), Some(Type::Named(name)) if name == "ChildProcess")
        {
            let field_ty = match field {
                "pid" => Type::U32,
                "stdin" => Type::Apply {
                    name: "WritableStream".to_string(),
                    args: vec![Type::Array(Box::new(Type::U8))],
                },
                "stdout" | "stderr" => Type::Apply {
                    name: "ReadableStream".to_string(),
                    args: vec![Type::Array(Box::new(Type::U8))],
                },
                _ => {
                    self.error(
                        "E0609",
                        format!("no field `{field}` on type `ChildProcess`"),
                        Some(field),
                    );
                    return ValueType::Error;
                }
            };
            return ValueType::Known(field_ty);
        }
        if matches!(transparent_value_type(&object_ty), Some(Type::Regex)) {
            let property_type = match field {
                "source" | "flags" => Type::Str,
                "global" | "ignoreCase" | "multiline" | "dotAll" | "sticky" | "unicode"
                | "hasIndices" => Type::Bool,
                "lastIndex" => Type::I32,
                _ => {
                    self.error(
                        "E0609",
                        format!("no property `{field}` on type `Regex`"),
                        Some(field),
                    );
                    return ValueType::Error;
                }
            };
            return ValueType::Known(property_type);
        }
        if let ValueType::Object(fields) = &object_ty {
            if let Some((_, field_ty)) = fields.iter().find(|(name, _)| name == field) {
                return field_ty.clone();
            }
            self.error(
                "E0609",
                format!("no field `{field}` on object type"),
                Some(field),
            );
            return ValueType::Error;
        }
        if let ValueType::Known(Type::Object(fields)) = &object_ty {
            if let Some(field_ty) = fields
                .iter()
                .find(|candidate| candidate.name == *field)
                .map(field_value_type)
            {
                return ValueType::Known(field_ty);
            }
            self.error(
                "E0609",
                format!("no field `{field}` on object type"),
                Some(field),
            );
            return ValueType::Error;
        }
        if let ValueType::Known(Type::Union(variants)) = &object_ty {
            let resolved_variants = self.union_variants_with_interface_fields(variants);
            if let Some(field_ty) = common_union_field_type(&resolved_variants, field) {
                return ValueType::Known(field_ty);
            }
            if let Some(underlying) = common_underlying_type(variants) {
                return self.check_field_on_value(ValueType::Known(underlying), field);
            }
            self.error(
                "E0609",
                format!("no common field `{field}` on every variant of union"),
                Some(field),
            );
            return ValueType::Error;
        }
        if crate::test::is_chain_modifier(field)
            && matches!(
                &object_ty,
                ValueType::Known(Type::Named(name)) if name == crate::test::ASSERTION_TYPE
            )
        {
            return ValueType::Known(Type::Named(crate::test::ASSERTION_TYPE.to_string()));
        }
        let named = match &object_ty {
            ValueType::Known(Type::Named(name)) => Some(name.clone()),
            ValueType::Known(Type::Ref(inner) | Type::MutRef(inner)) => match inner.as_ref() {
                Type::Named(name) => Some(name.clone()),
                _ => None,
            },
            _ => None,
        };
        if let Some(name) = named {
            if let Some(Type::Union(variants)) = self.type_aliases.get(&name).cloned() {
                let resolved_variants = self.union_variants_with_interface_fields(&variants);
                if let Some(field_ty) = common_union_field_type(&resolved_variants, field) {
                    return ValueType::Known(field_ty);
                }
                self.error(
                    "E0609",
                    format!("no common field `{field}` on every variant of union `{name}`"),
                    Some(field),
                );
                return ValueType::Error;
            }
            if let Some(field_ty) = self
                .interfaces
                .get(&name)
                .and_then(|fields| fields.iter().find(|candidate| candidate.name == *field))
                .map(field_value_type)
            {
                ValueType::Known(field_ty)
            } else {
                self.error(
                    "E0609",
                    format!("no field `{field}` on type `{name}`"),
                    Some(field),
                );
                ValueType::Error
            }
        } else if matches!(object_ty, ValueType::Error) {
            ValueType::Error
        } else {
            self.error(
                "E0609",
                format!("cannot access field `{field}` on `{}`", object_ty.display()),
                Some(field),
            );
            ValueType::Error
        }
    }

    fn object_fields_assignable(
        &self,
        expected_fields: &[crate::ast::Field],
        actual_fields: &[(String, ValueType)],
    ) -> bool {
        expected_fields.iter().all(|field| {
            actual_fields
                .iter()
                .find(|(candidate, _)| candidate == &field.name)
                .map(|(_, value)| self.is_assignable(&field_value_type(field), value))
                .unwrap_or(field.optional)
        }) && actual_fields.iter().all(|(actual_name, _)| {
            expected_fields
                .iter()
                .any(|field| field.name == *actual_name)
        })
    }

    fn object_fields_exactly_match_interface(
        &self,
        expected_fields: &[crate::ast::Field],
        actual: &ValueType,
    ) -> bool {
        let ValueType::Object(actual_fields) = actual else {
            return false;
        };
        expected_fields.len() == actual_fields.len()
            && expected_fields.iter().all(|field| {
                !field.optional
                    && !field.readonly
                    && actual_fields
                        .iter()
                        .find(|(candidate, _)| candidate == &field.name)
                        .map(|(_, value)| self.is_assignable(&field.ty, value))
                        .unwrap_or(false)
            })
            && actual_fields.iter().all(|(actual_name, _)| {
                expected_fields
                    .iter()
                    .any(|field| field.name == *actual_name)
            })
    }

    fn check_fetch_verb(&mut self, verb: &str, args: &[Expr]) -> ValueType {
        if args.is_empty() || args.len() > 2 {
            self.error(
                "E0061",
                format!(
                    "`fetch.{verb}` expects 1 to 2 argument(s), but {} were provided",
                    args.len()
                ),
                Some(verb),
            );
            for arg in args {
                self.check_expr(arg);
            }
            return ValueType::Error;
        }
        let url = self.check_expr_as(&args[0], &Type::Str);
        if !self.is_assignable(&Type::Str, &url) {
            self.error_at(
                "E0308",
                format!(
                    "`fetch.{verb}` expects a `string` URL, found `{}`",
                    url.display()
                ),
                args[0].span,
            );
        }
        if let Some(options) = args.get(1) {
            let expected = Type::Named("FetchOptions".to_string());
            let before = self.diagnostics.len();
            let actual = self.check_expr_as(options, &expected);
            let reported_specifically = self.diagnostics.len() > before;
            if !reported_specifically && !self.is_assignable(&expected, &actual) {
                self.error_at(
                    "E0308",
                    format!(
                        "`fetch.{verb}` options expect `FetchOptions`, found `{}`",
                        actual.display()
                    ),
                    options.span,
                );
            }
        }
        ValueType::Known(Type::Apply {
            name: "Promise".to_string(),
            args: vec![Type::Named("HttpClientResponse".to_string())],
        })
    }

    fn check_fetch_stream(&mut self, args: &[Expr]) -> ValueType {
        if args.is_empty() || args.len() > 2 {
            self.error(
                "E0061",
                format!(
                    "`fetch.stream` expects 1 to 2 argument(s), but {} were provided",
                    args.len()
                ),
                Some("stream"),
            );
            for arg in args {
                self.check_expr(arg);
            }
            return ValueType::Error;
        }
        let url = self.check_expr_as(&args[0], &Type::Str);
        if !self.is_assignable(&Type::Str, &url) {
            self.error_at(
                "E0308",
                format!(
                    "`fetch.stream` expects a `string` URL, found `{}`",
                    url.display()
                ),
                args[0].span,
            );
        }
        if let Some(options) = args.get(1) {
            let expected = Type::Named("FetchOptions".to_string());
            let before = self.diagnostics.len();
            let actual = self.check_expr_as(options, &expected);
            let reported_specifically = self.diagnostics.len() > before;
            if !reported_specifically && !self.is_assignable(&expected, &actual) {
                self.error_at(
                    "E0308",
                    format!(
                        "`fetch.stream` options expect `FetchOptions`, found `{}`",
                        actual.display()
                    ),
                    options.span,
                );
            }
        }
        ValueType::Known(Type::Apply {
            name: "Promise".to_string(),
            args: vec![Type::Named("HttpClientStreamResponse".to_string())],
        })
    }

    fn first_unresolved_type_name(&self, ty: &Type) -> Option<String> {
        match ty {
            Type::Named(name) => (!self.type_name_exists(name)).then(|| name.clone()),
            Type::Array(inner)
            | Type::Optional(inner)
            | Type::Ref(inner)
            | Type::MutRef(inner)
            | Type::Readonly(inner) => self.first_unresolved_type_name(inner),
            Type::Apply { args, .. } => args
                .iter()
                .find_map(|ty| self.first_unresolved_type_name(ty)),
            Type::Union(variants) | Type::Tuple(variants) => variants
                .iter()
                .find_map(|ty| self.first_unresolved_type_name(ty)),
            Type::Object(fields) => fields
                .iter()
                .find_map(|field| self.first_unresolved_type_name(&field.ty)),
            _ => None,
        }
    }

    fn takes_only_a_request(&self, handler: &Expr) -> bool {
        matches!(handler.kind(), ExprKind::ArrowFn { params, .. } if params.len() == 1)
            || matches!(
                handler,
                Expr { kind: ExprKind::Ident(name), .. }
                    if self
                        .fn_body_metadata
                        .get(name)
                        .is_some_and(|metadata| metadata.params.len() == 1)
            )
    }

    fn excess_object_field(&self, expected: &Type, actual: &ValueType) -> Option<String> {
        let ValueType::Object(actual_fields) = actual else {
            return None;
        };
        let expected_fields = match transparent_type(expected) {
            Type::Named(name) => self.interfaces.get(name)?,
            Type::Object(fields) => fields,
            _ => return None,
        };
        actual_fields
            .iter()
            .find(|(actual_name, _)| {
                !expected_fields
                    .iter()
                    .any(|field| field.name == *actual_name)
            })
            .map(|(name, _)| name.clone())
    }

    fn object_field_type_mismatch(
        &self,
        expected: &Type,
        actual: &ValueType,
    ) -> Option<(String, Type, ValueType)> {
        let ValueType::Object(actual_fields) = actual else {
            return None;
        };
        let expected_fields = match transparent_type(expected) {
            Type::Named(name) => self.interfaces.get(name)?,
            Type::Object(fields) => fields,
            _ => return None,
        };
        expected_fields.iter().find_map(|field| {
            let actual_ty = actual_fields
                .iter()
                .find(|(actual_name, _)| actual_name == &field.name)?
                .1
                .clone();
            let expected_ty = field_value_type(field);
            (!self.is_assignable(&expected_ty, &actual_ty))
                .then(|| (field.name.clone(), expected_ty, actual_ty))
        })
    }

    fn missing_object_field(
        &self,
        expected: &Type,
        actual: &ValueType,
    ) -> Option<(String, String)> {
        let ValueType::Object(actual_fields) = actual else {
            return None;
        };
        let (target, expected_fields) = match transparent_type(expected) {
            Type::Named(name) => (name.clone(), self.interfaces.get(name)?),
            Type::Object(fields) => ("object".to_string(), fields),
            _ => return None,
        };
        expected_fields
            .iter()
            .find(|field| {
                !field.optional
                    && !actual_fields
                        .iter()
                        .any(|(actual_name, _)| actual_name == &field.name)
            })
            .map(|field| (target, field.name.clone()))
    }

    fn declare(&mut self, name: &str, symbol: Symbol) {
        let scope = self.scopes.last_mut().unwrap();
        if scope.contains_key(name) {
            self.error_on_declaration(
                "E0428",
                format!("identifier `{name}` is declared more than once in this scope"),
                Some(name),
            );
        } else {
            scope.insert(name.to_string(), symbol);
            self.constant_integers
                .last_mut()
                .expect("the root constant scope is always present")
                .entry(name.to_string())
                .or_insert(None);
            if let Some(depths) = self.declaration_depths.last_mut() {
                depths.insert(name.to_string(), self.fn_depth);
            }
            self.validator_facts
                .last_mut()
                .unwrap()
                .insert(name.to_string(), ValidatorFacts::default());
        }
    }

    fn for_index_hint(&self, init: &Stmt, cond: &Expr) -> Option<Type> {
        let StmtKind::VarDecl {
            name,
            ty: Type::Infer,
            value: Some(value),
            ..
        } = init.kind()
        else {
            return None;
        };
        if !matches!(value.kind(), ExprKind::IntLit(start) if *start >= 0) {
            return None;
        }
        let ExprKind::BinOp { op, left, right } = cond.kind() else {
            return None;
        };
        if !matches!(
            op,
            BinOp::Lt | BinOp::Lte | BinOp::Gt | BinOp::Gte | BinOp::Eq | BinOp::Neq
        ) {
            return None;
        }
        let bound = match (left.kind(), right.kind()) {
            (ExprKind::Ident(counter), _) if counter == name => right.as_ref(),
            (_, ExprKind::Ident(counter)) if counter == name => left.as_ref(),
            _ => return None,
        };
        let bound_ty = self.for_bound_type(bound)?;
        is_integer_type(&bound_ty).then_some(bound_ty)
    }

    fn for_bound_type(&self, expr: &Expr) -> Option<Type> {
        match expr.kind() {
            ExprKind::Ident(name) => match &self.lookup(name)?.ty {
                ValueType::Known(ty) => Some(transparent_type(ty).clone()),
                _ => None,
            },
            ExprKind::FieldAccess { object, field }
                if matches!(field.as_str(), "length" | "capacity") =>
            {
                let object_ty = self.for_bound_type(object)?;
                matches!(object_ty, Type::Array(_) | Type::Tuple(_) | Type::Str)
                    .then_some(Type::Usize)
            }
            ExprKind::BinOp {
                op: BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod,
                left,
                right,
            } => self
                .for_bound_type(left)
                .or_else(|| self.for_bound_type(right)),
            _ => None,
        }
    }

    fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }

    fn report_unused_imports(&mut self) {
        let unused = self
            .scopes
            .first()
            .into_iter()
            .flat_map(|scope| scope.iter())
            .filter(|(_, symbol)| {
                if symbol.kind != SymbolKind::Import || symbol.used {
                    return false;
                }
                let is_type_import = matches!(&symbol.ty, ValueType::Known(Type::Named(n))
                    if self.interfaces.contains_key(n.as_str())
                        || self.type_aliases.contains_key(n.as_str()));
                !is_type_import
            })
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        let mut unused = unused;
        unused.sort();
        for name in unused {
            self.warning("W0001", format!("unused import `{name}`"), Some(&name));
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::default());
        self.constant_integers.push(HashMap::default());
        self.narrowed_paths.push(HashSet::default());
        self.declaration_depths.push(HashMap::default());
        self.validator_facts.push(HashMap::default());
        self.joined_handle_names.push(HashMap::default());
        self.awaited_promise_names.push(HashMap::default());
        self.mutex_guard_names.push(HashSet::default());
        self.finalizer_scopes.push(Vec::new());
    }

    fn path_is_narrowed(&self, path: &str) -> bool {
        self.narrowed_paths.iter().any(|scope| scope.contains(path))
    }

    fn narrow_field_path(&mut self, path: &str) {
        let Some((root, _)) = path.split_once('.') else {
            return;
        };
        let Some(symbol) = self.lookup(root) else {
            return;
        };
        if symbol.mutable {
            return;
        }
        if let Some(scope) = self.narrowed_paths.last_mut() {
            scope.insert(path.to_string());
        }
    }

    fn pop_scope(&mut self) {
        self.narrowed_paths.pop();
        debug_assert_eq!(self.finalizer_scopes.len(), self.scopes.len());
        debug_assert_eq!(self.constant_integers.len(), self.scopes.len());
        let finalizers = self
            .finalizer_scopes
            .pop()
            .expect("the root lexical finalizer scope is always present");
        debug_assert!(finalizers.iter().all(|registration| {
            registration.keyword_span.start <= registration.keyword_span.end
                && matches!(registration.kind, DeferKind::Always | DeferKind::OnError)
                && registration
                    .captures
                    .iter()
                    .all(|capture| !capture.is_empty())
        }));
        self.scopes.pop();
        self.constant_integers.pop();
        self.declaration_depths.pop();
        self.validator_facts.pop();
        self.joined_handle_names.pop();
        self.awaited_promise_names.pop();
        self.mutex_guard_names.pop();
    }

    fn register_finalizer(
        &mut self,
        kind: DeferKind,
        keyword_span: Span,
        captures: HashSet<String>,
    ) {
        self.finalizer_scopes
            .last_mut()
            .expect("the root lexical finalizer scope is always present")
            .push(FinalizerRegistration {
                kind,
                keyword_span,
                captures,
            });
    }

    fn in_finalizer(&self) -> bool {
        self.finalizer_contexts.last().is_some_and(|context| {
            debug_assert!(
                context.keyword_span.start <= context.keyword_span.end
                    && context.captures.iter().all(|capture| !capture.is_empty())
            );
            true
        })
    }

    fn active_finalizer_name(&self) -> Option<&'static str> {
        self.finalizer_contexts
            .last()
            .map(|context| match context.kind {
                DeferKind::Always => "defer",
                DeferKind::OnError => "errdefer",
            })
    }

    fn is_join_handle_consumed(&self, name: &str, span: Span) -> bool {
        self.joined_handle_names
            .iter()
            .rev()
            .any(|scope| scope.get(name).is_some_and(|consumed| *consumed != span))
    }

    fn mark_join_handle_consumed(&mut self, name: &str, span: Span) {
        if let Some(scope) = self.joined_handle_names.last_mut() {
            scope.insert(name.to_string(), span);
        }
    }

    fn mark_mutex_guard_local(&mut self, name: &str) {
        if let Some(scope) = self.mutex_guard_names.last_mut() {
            scope.insert(name.to_string());
        }
    }

    fn live_mutex_guard_name(&self) -> Option<&str> {
        self.mutex_guard_names
            .iter()
            .rev()
            .find_map(|scope| scope.iter().next())
            .map(String::as_str)
    }

    fn declaration_fn_depth(&self, name: &str) -> Option<usize> {
        self.declaration_depths
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .copied()
    }

    fn declare_match_bindings(
        &mut self,
        pattern: &MatchPattern,
        subject_ty: &ValueType,
        arm_index: usize,
    ) {
        let resolved_ty = match subject_ty {
            ValueType::Known(Type::Optional(inner)) => ValueType::Known(inner.as_ref().clone()),
            ValueType::Known(Type::Union(variants)) => ValueType::Union(
                variants
                    .iter()
                    .map(|t| match self.resolve_object_type(t) {
                        Some(fields) => ValueType::Object(fields),
                        None => ValueType::Known(t.clone()),
                    })
                    .collect(),
            ),
            ValueType::Known(Type::Result { success, error }) => {
                let variant = match pattern {
                    MatchPattern::ResultVariant { success: true, .. } => success,
                    MatchPattern::ResultVariant { success: false, .. } => error,
                    _ if arm_index == 0 => success,
                    _ => error,
                };
                if crate::native_handles::is_handle_type(variant) {
                    ValueType::Known((**variant).clone())
                } else {
                    match self.resolve_object_type(variant) {
                        Some(fields) => ValueType::Object(fields),
                        None => ValueType::Known((**variant).clone()),
                    }
                }
            }
            ValueType::Known(Type::Named(name)) => match self.type_aliases.get(name) {
                Some(Type::Union(variants)) => ValueType::Union(
                    variants
                        .iter()
                        .map(|t| match self.resolve_object_type(t) {
                            Some(fields) => ValueType::Object(fields),
                            None => ValueType::Known(t.clone()),
                        })
                        .collect(),
                ),
                _ => subject_ty.clone(),
            },
            _ => subject_ty.clone(),
        };
        match pattern {
            MatchPattern::Binding(name) => {
                self.declare(
                    name,
                    Symbol {
                        ty: resolved_ty.clone(),
                        mutable: false,
                        kind: SymbolKind::Variable,
                        used: false,
                        array_len: None,
                    },
                );
            }
            MatchPattern::ResultVariant {
                binding: Some(name),
                ..
            } => {
                self.declare(
                    name,
                    Symbol {
                        ty: resolved_ty.clone(),
                        mutable: false,
                        kind: SymbolKind::Variable,
                        used: false,
                        array_len: None,
                    },
                );
            }
            MatchPattern::Object(fields) => match &resolved_ty {
                ValueType::Object(obj_fields) => {
                    for field in fields {
                        if let Some(binding) = &field.binding
                            && let Some((_, field_ty)) =
                                obj_fields.iter().find(|(k, _)| k == &field.key)
                        {
                            self.declare(
                                binding,
                                Symbol {
                                    ty: field_ty.clone(),
                                    mutable: false,
                                    kind: SymbolKind::Variable,
                                    used: false,
                                    array_len: None,
                                },
                            );
                        }
                    }
                }
                ValueType::Union(variants) => {
                    let matched_variant = variants.iter().find(|variant| {
                        if let ValueType::Object(variant_fields) = variant {
                            fields.iter().all(|field| {
                                if let Some(lit) = &field.literal {
                                    variant_fields.iter().any(|(k, v)| {
                                        k == &field.key
                                            && match (v, lit) {
                                                (
                                                    ValueType::Known(Type::Str),
                                                    Expr {
                                                        kind: ExprKind::StrLit(_),
                                                        ..
                                                    },
                                                ) => true,
                                                _ => true,
                                            }
                                    })
                                } else {
                                    variant_fields.iter().any(|(k, _)| k == &field.key)
                                }
                            })
                        } else {
                            false
                        }
                    });
                    if let Some(ValueType::Object(variant_fields)) = matched_variant {
                        for field in fields {
                            if let Some(binding) = &field.binding
                                && let Some((_, field_ty)) =
                                    variant_fields.iter().find(|(k, _)| k == &field.key)
                            {
                                self.declare(
                                    binding,
                                    Symbol {
                                        ty: field_ty.clone(),
                                        mutable: false,
                                        kind: SymbolKind::Variable,
                                        used: false,
                                        array_len: None,
                                    },
                                );
                            }
                        }
                    } else {
                        for field in fields {
                            if let Some(binding) = &field.binding {
                                for variant in variants {
                                    if let ValueType::Object(vf) = variant
                                        && let Some((_, field_ty)) =
                                            vf.iter().find(|(k, _)| k == &field.key)
                                    {
                                        self.declare(
                                            binding,
                                            Symbol {
                                                ty: field_ty.clone(),
                                                mutable: false,
                                                kind: SymbolKind::Variable,
                                                used: false,
                                                array_len: None,
                                            },
                                        );
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            },
            MatchPattern::EnumVariant {
                enum_name,
                variant,
                binding: Some(binding),
            } => {
                let payload_ty = self
                    .enums
                    .get(enum_name)
                    .and_then(|enum_info| {
                        enum_info
                            .members
                            .iter()
                            .position(|(name, _)| name == variant)
                            .map(|index| match &enum_info.member_kinds[index] {
                                EnumPayload::Unit => ValueType::Known(Type::Void),
                                EnumPayload::Tuple(types) if types.len() == 1 => {
                                    ValueType::Known(types[0].clone())
                                }
                                EnumPayload::Tuple(types) => {
                                    ValueType::Known(Type::Tuple(types.clone()))
                                }
                                EnumPayload::Struct(fields) => ValueType::Object(
                                    fields
                                        .iter()
                                        .map(|field| {
                                            (
                                                field.name.clone(),
                                                ValueType::Known(field_value_type(field)),
                                            )
                                        })
                                        .collect(),
                                ),
                            })
                    })
                    .unwrap_or_else(|| resolved_ty.clone());
                self.declare_match_bindings(binding, &payload_ty, arm_index);
            }
            MatchPattern::Tuple(elements) => match &resolved_ty {
                ValueType::Known(Type::Tuple(element_types))
                    if element_types.len() == elements.len() =>
                {
                    for (element, ty) in elements.iter().zip(element_types.iter()) {
                        self.declare_match_bindings(
                            element,
                            &ValueType::Known(ty.clone()),
                            arm_index,
                        );
                    }
                }
                _ => {
                    for element in elements {
                        self.declare_match_bindings(element, &resolved_ty, arm_index);
                    }
                }
            },
            MatchPattern::Array { elements, .. } => {
                let ValueType::Known(subject) = &resolved_ty else {
                    return;
                };
                let Type::Array(element) = transparent_type(subject) else {
                    return;
                };
                let element_ty = element.as_ref().clone();
                for element in elements {
                    let binding_ty = if element.is_rest() {
                        ValueType::Known(Type::Array(Box::new(element_ty.clone())))
                    } else {
                        ValueType::Known(element_ty.clone())
                    };
                    self.declare_match_bindings(&element.pattern, &binding_ty, arm_index);
                }
            }
            _ => {}
        }
    }

    fn resolve_object_type(&self, ty: &Type) -> Option<Vec<(String, ValueType)>> {
        match ty {
            Type::Object(fields) => Some(
                fields
                    .iter()
                    .map(|f| (f.name.clone(), ValueType::Known(field_value_type(f))))
                    .collect(),
            ),
            Type::Named(name) => self.interfaces.get(name).map(|fields| {
                fields
                    .iter()
                    .map(|f| (f.name.clone(), ValueType::Known(field_value_type(f))))
                    .collect()
            }),
            _ => None,
        }
    }

    fn check_match_pattern_support(
        &mut self,
        pattern: &MatchPattern,
        subject_ty: &ValueType,
        pattern_span: Span,
    ) {
        match pattern {
            MatchPattern::Array { elements, span } => {
                let ValueType::Known(subject) = subject_ty else {
                    return;
                };
                let Type::Array(element_ty) = transparent_type(subject) else {
                    self.error_at(
                        "E0529",
                        format!(
                            "array match patterns require an array subject, found `{}`",
                            subject_ty.display()
                        ),
                        *span,
                    );
                    return;
                };
                for element in elements {
                    if element.is_rest() {
                        if !matches!(
                            element.pattern,
                            MatchPattern::Binding(_) | MatchPattern::Wildcard
                        ) {
                            self.error_at(
                                "E0793",
                                "an array rest pattern must bind an identifier or use `_`"
                                    .to_string(),
                                element.span,
                            );
                        }
                        continue;
                    }
                    match &element.pattern {
                        MatchPattern::Binding(_) | MatchPattern::Wildcard => {}
                        MatchPattern::Lit(literal) => {
                            let actual = self.check_expr(literal);
                            if !self.is_assignable(element_ty, &actual) {
                                self.error_at(
                                    "E0308",
                                    format!(
                                        "array pattern element must be `{element_ty}`, found `{}`",
                                        actual.display()
                                    ),
                                    literal.span,
                                );
                            }
                        }
                        _ => self.error_at(
                            "E0658",
                            "nested array match patterns are not supported yet".to_string(),
                            element.span,
                        ),
                    }
                }
            }
            MatchPattern::Or(_) => self.error(
                "E0658",
                "match OR patterns are not supported yet".to_string(),
                None,
            ),
            MatchPattern::Object(fields) => {
                for field in fields {
                    if let Some(literal) = &field.literal
                        && let ExprKind::ArrayLit(_) = literal.kind()
                    {
                        self.error(
                            "E0658",
                            "match array patterns are not supported yet".to_string(),
                            None,
                        );
                    }
                }
            }
            MatchPattern::Range {
                start,
                end,
                inclusive,
            } => {
                let subject = subject_ty.clone();
                let bound_ty = |this: &mut Self, bound: &Expr| -> ValueType {
                    let actual = this.check_expr(bound);
                    if !is_numeric(&actual) {
                        this.error_at(
                            "E0308",
                            format!(
                                "range pattern bound must be numeric, found `{}`",
                                actual.display()
                            ),
                            bound.span,
                        );
                    } else if let ValueType::Known(expected) = &subject
                        && !this.is_assignable(expected, &actual)
                    {
                        this.error_at(
                            "E0308",
                            format!(
                                "range pattern bound must be `{expected}`, found `{}`",
                                actual.display()
                            ),
                            bound.span,
                        );
                    }
                    actual
                };
                let start_ty = start.as_ref().map(|bound| bound_ty(self, bound));
                let end_ty = end.as_ref().map(|bound| bound_ty(self, bound));
                if let (Some(start), Some(end)) = (start, end)
                    && let (Some(start_value), Some(end_value)) =
                        (numeric_literal_value(start), numeric_literal_value(end))
                {
                    let empty = if *inclusive {
                        start_value > end_value
                    } else {
                        start_value >= end_value
                    };
                    if empty {
                        self.warning_at(
                            "W0003",
                            "range pattern is empty or inverted: the start is not before the end"
                                .to_string(),
                            start.span,
                        );
                    }
                }
                let _ = (start_ty, end_ty);
            }
            MatchPattern::ResultVariant { .. } => {
                if !matches!(subject_ty, ValueType::Known(Type::Result { .. })) {
                    self.error(
                        "E3073",
                        "`Ok(...)` and `Err(...)` patterns require a `Result` subject".to_string(),
                        None,
                    );
                }
            }
            MatchPattern::Lit(_) | MatchPattern::Wildcard | MatchPattern::Binding(_) => {}
            MatchPattern::Tuple(elements) => {
                for element in elements {
                    self.check_match_pattern_support(element, subject_ty, pattern_span);
                }
            }
            MatchPattern::EnumVariant {
                enum_name,
                variant,
                binding,
            } => {
                if let Some(enum_info) = self.enums.get(enum_name)
                    && let Some(member_idx) = enum_info
                        .members
                        .iter()
                        .position(|(name, _)| name == variant)
                    && let Some(binding) = binding
                {
                    match (&enum_info.member_kinds[member_idx], binding.as_ref()) {
                        (EnumPayload::Tuple(payload_types), MatchPattern::Tuple(elements)) => {
                            if payload_types.len() != elements.len() {
                                self.error_at_with_help(
                                    "E3292",
                                    format!(
                                        "variant `{}.{}` expects {} field(s), but {} were given",
                                        enum_name,
                                        variant,
                                        payload_types.len(),
                                        elements.len()
                                    ),
                                    pattern_span,
                                    vec![format!(
                                        "bind every field, as in `{}.{}({})`",
                                        enum_name,
                                        variant,
                                        enum_payload_binding_list(payload_types.len())
                                    )],
                                );
                            }
                        }
                        (EnumPayload::Tuple(payload_types), MatchPattern::Binding(_))
                            if payload_types.len() > 1 =>
                        {
                            self.error_at_with_help(
                                "E3292",
                                format!(
                                    "variant `{}.{}` expects {} field(s), but 1 was given",
                                    enum_name,
                                    variant,
                                    payload_types.len()
                                ),
                                pattern_span,
                                vec![format!(
                                    "bind every field, as in `{}.{}({})`",
                                    enum_name,
                                    variant,
                                    enum_payload_binding_list(payload_types.len())
                                )],
                            );
                        }
                        (EnumPayload::Tuple(payload_types), _) if !payload_types.is_empty() => {}
                        (EnumPayload::Unit, _) => {
                            self.error_at_with_help(
                                        "E3292",
                                        format!(
                                            "variant `{enum_name}.{variant}` has no payload, but a binding was given"
                                        ),
                                        pattern_span,
                                        vec![format!(
                                            "drop the binding and write `{}.{}`",
                                            enum_name, variant
                                        )],
                                    );
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn resolve_value_object_fields(&self, value: &ValueType) -> Option<Vec<(String, ValueType)>> {
        match value {
            ValueType::Object(fields) => Some(fields.clone()),
            ValueType::Known(ty) => self.resolve_object_type(ty),
            _ => None,
        }
    }

    fn check_unique_element_is_comparable(&mut self, decorator: &ValidatorDecorator, ty: &Type) {
        if decorator.name != "unique" {
            return;
        }
        let unwrapped = match ty {
            Type::Optional(inner) => inner.as_ref(),
            other => other,
        };
        let Type::Array(element) = transparent_type(unwrapped) else {
            return;
        };
        if self.supports_value_equality(element) {
            return;
        }
        self.error_at(
            "E3161",
            format!(
                "`@unique` cannot compare elements of type `{element}`, because they are not compared by value"
            ),
            decorator.span,
        );
    }

    fn supports_value_equality(&self, ty: &Type) -> bool {
        let unwrapped = match ty {
            Type::Optional(inner) => inner.as_ref(),
            other => other,
        };
        match transparent_type(unwrapped) {
            Type::Array(_) => false,
            Type::Tuple(elements) => elements
                .iter()
                .all(|element| self.supports_value_equality(element)),
            Type::Object(fields) => fields
                .iter()
                .all(|field| self.supports_value_equality(&field.ty)),
            Type::Named(name) => match self.interfaces.get(name.as_str()) {
                Some(fields) => fields
                    .iter()
                    .all(|field| self.supports_value_equality(&field.ty)),
                None => true,
            },
            _ => true,
        }
    }

    fn error_on_declaration_with_help(
        &mut self,
        code: &'static str,
        message: String,
        subject: Option<&str>,
        help: Vec<String>,
    ) {
        self.error_on_declaration(code, message, subject);
        if let Some(last) = self.diagnostics.last_mut() {
            last.help = help;
        }
    }

    fn error_with_help(
        &mut self,
        code: &'static str,
        message: String,
        subject: Option<&str>,
        help: Vec<String>,
    ) {
        self.error(code, message, subject);
        if let Some(last) = self.diagnostics.last_mut() {
            last.help = help;
        }
    }

    fn error_at_member(
        &mut self,
        code: &'static str,
        message: String,
        member_span: Option<Span>,
        member: &str,
    ) {
        self.error_at_member_with_help(code, message, member_span, member, Vec::new());
    }

    fn error_at_member_with_help(
        &mut self,
        code: &'static str,
        message: String,
        member_span: Option<Span>,
        member: &str,
        help: Vec<String>,
    ) {
        match member_span.filter(|span| !span.is_dummy()) {
            Some(span) => self.error_at_with_help(code, message, span, help),
            None => {
                self.error(code, message, Some(member));
                if !help.is_empty()
                    && let Some(last) = self.diagnostics.last_mut()
                {
                    last.help = help;
                }
            }
        }
    }

    fn error_at_with_help(
        &mut self,
        code: &'static str,
        message: String,
        span: Span,
        help: Vec<String>,
    ) {
        self.error_at(code, message, span);
        if let Some(last) = self.diagnostics.last_mut() {
            last.help = help;
        }
    }

    fn error_at(&mut self, code: &'static str, message: String, span: Span) {
        self.diagnostics.push(CheckDiagnostic {
            code,
            severity: CheckSeverity::Error,
            message,
            span: (!span.is_dummy()).then_some(span),
            subject: None,
            help: Vec::new(),
            prefer_declaration: false,
        });
    }

    fn error_at_with_subject(
        &mut self,
        code: &'static str,
        message: String,
        span: Span,
        subject: String,
    ) {
        self.diagnostics.push(CheckDiagnostic {
            code,
            severity: CheckSeverity::Error,
            message,
            span: (!span.is_dummy()).then_some(span),
            subject: Some(subject),
            help: Vec::new(),
            prefer_declaration: false,
        });
    }

    fn error(&mut self, code: &'static str, message: String, subject: Option<&str>) {
        let span = self.current_span();
        self.diagnostics.push(CheckDiagnostic {
            code,
            severity: CheckSeverity::Error,
            message,
            span,
            subject: subject.map(str::to_string),
            help: Vec::new(),
            prefer_declaration: false,
        });
    }

    fn current_span(&self) -> Option<Span> {
        self.span_stack
            .last()
            .copied()
            .or(self.item_span)
            .filter(|span| !span.is_dummy())
    }

    fn error_on_declaration(&mut self, code: &'static str, message: String, subject: Option<&str>) {
        let span = self.current_span();
        self.diagnostics.push(CheckDiagnostic {
            code,
            severity: CheckSeverity::Error,
            message,
            span,
            subject: subject.map(str::to_string),
            help: Vec::new(),
            prefer_declaration: true,
        });
    }

    fn warning(&mut self, code: &'static str, message: String, subject: Option<&str>) {
        let span = self.current_span();
        self.diagnostics.push(CheckDiagnostic {
            code,
            severity: CheckSeverity::Warning,
            message,
            span,
            subject: subject.map(str::to_string),
            help: Vec::new(),
            prefer_declaration: false,
        });
    }

    fn warning_at(&mut self, code: &'static str, message: String, span: Span) {
        self.diagnostics.push(CheckDiagnostic {
            code,
            severity: CheckSeverity::Warning,
            message,
            span: (!span.is_dummy()).then_some(span),
            subject: None,
            help: Vec::new(),
            prefer_declaration: false,
        });
    }

    fn warning_at_with_help(
        &mut self,
        code: &'static str,
        message: String,
        span: Span,
        help: Vec<String>,
    ) {
        self.diagnostics.push(CheckDiagnostic {
            code,
            severity: CheckSeverity::Warning,
            message,
            span: (!span.is_dummy()).then_some(span),
            subject: None,
            help,
            prefer_declaration: false,
        });
    }

    fn check_positive_capacity(
        &mut self,
        argument: Option<&Expr>,
        callee: &str,
        requirement: &str,
        reason: &str,
        help: &str,
    ) {
        let Some(argument) = argument else {
            return;
        };
        if !matches!(argument.kind(), ExprKind::IntLit(0) | ExprKind::UIntLit(0)) {
            return;
        }
        self.error_at_with_help(
            "E3035",
            format!("`{callee}` requires {requirement}: {reason}"),
            argument.span,
            vec![help.to_string()],
        );
    }

    fn check_busy_wait_loop(&mut self, cond: &Expr, body: &[Stmt], span: Span) {
        if self.async_function_stack.last() != Some(&true) {
            return;
        }
        if !matches!(cond.kind(), ExprKind::BoolLit(true)) {
            return;
        }
        if first_await_in_block(body).is_some() || block_can_end(body) {
            return;
        }
        self.warning_at_with_help(
            "E3032",
            "this loop never yields to the executor: it cannot end and its body never awaits, so it starves every other task sharing its worker thread".to_string(),
            span,
            vec![
                "add an `await` inside the loop, `await sleepAsync(...)` is enough to hand control back, or give it a `break`, `return` or `throw` that can end it".to_string(),
            ],
        );
    }
}

#[cfg(test)]
#[path = "../../tests/unit/type_checker.rs"]
mod tests;
