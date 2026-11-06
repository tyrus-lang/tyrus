use crate::ast::{Expr, ExprKind, Item, Program, Stmt, StmtKind, TemplatePart, Type};
use crate::collections::HashMap;
use crate::module_loader::ExportInfo;

pub const MODULE: &str = "tyrus:test";

pub fn is_test_file(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".test.tyr"))
}

pub const NOT_MODIFIER: &str = "not";
pub const REJECTS_MODIFIER: &str = "rejects";
pub const RESOLVES_MODIFIER: &str = "resolves";

pub fn is_chain_modifier(field: &str) -> bool {
    matches!(field, NOT_MODIFIER | REJECTS_MODIFIER | RESOLVES_MODIFIER)
}

pub const ASSERTION_TYPE: &str = "TyrusTestAssertion";

pub const DESCRIBE_TARGET: &str = "__tyrus_test_describe";
pub const TEST_TARGET: &str = "__tyrus_test_test";
pub const IT_TARGET: &str = "__tyrus_test_it";
pub const BEFORE_ALL_TARGET: &str = "__tyrus_test_beforeAll";
pub const AFTER_ALL_TARGET: &str = "__tyrus_test_afterAll";
pub const BEFORE_EACH_TARGET: &str = "__tyrus_test_beforeEach";
pub const AFTER_EACH_TARGET: &str = "__tyrus_test_afterEach";
pub const EXPECT_TARGET: &str = "__tyrus_test_expect";

fn fn_void(params: Vec<Type>) -> Type {
    Type::Function {
        params,
        ret: Box::new(Type::Void),
    }
}

pub fn describe_type() -> Type {
    fn_void(vec![Type::Str, Type::Unknown])
}

pub fn test_type() -> Type {
    fn_void(vec![Type::Str, Type::Unknown])
}

pub fn hook_type() -> Type {
    fn_void(vec![Type::Unknown])
}

pub fn expect_type() -> Type {
    Type::Function {
        params: vec![Type::Unknown],
        ret: Box::new(Type::Named(ASSERTION_TYPE.to_string())),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatcherArg {
    None,
    Any,
    Numeric,
    Length,
    Text,
    Contained,
    Throws,
}

#[derive(Debug, Clone, Copy)]
pub struct MatcherDef {
    pub name: &'static str,
    pub arg: MatcherArg,
}

pub const MATCHERS: &[MatcherDef] = &[
    MatcherDef {
        name: "toBe",
        arg: MatcherArg::Any,
    },
    MatcherDef {
        name: "toEqual",
        arg: MatcherArg::Any,
    },
    MatcherDef {
        name: "toStrictEqual",
        arg: MatcherArg::Any,
    },
    MatcherDef {
        name: "toBeTruthy",
        arg: MatcherArg::None,
    },
    MatcherDef {
        name: "toBeFalsy",
        arg: MatcherArg::None,
    },
    MatcherDef {
        name: "toBeNull",
        arg: MatcherArg::None,
    },
    MatcherDef {
        name: "toBeDefined",
        arg: MatcherArg::None,
    },
    MatcherDef {
        name: "toBeUndefined",
        arg: MatcherArg::None,
    },
    MatcherDef {
        name: "toBeGreaterThan",
        arg: MatcherArg::Numeric,
    },
    MatcherDef {
        name: "toBeGreaterThanOrEqual",
        arg: MatcherArg::Numeric,
    },
    MatcherDef {
        name: "toBeLessThan",
        arg: MatcherArg::Numeric,
    },
    MatcherDef {
        name: "toBeLessThanOrEqual",
        arg: MatcherArg::Numeric,
    },
    MatcherDef {
        name: "toBeCloseTo",
        arg: MatcherArg::Numeric,
    },
    MatcherDef {
        name: "toHaveLength",
        arg: MatcherArg::Length,
    },
    MatcherDef {
        name: "toContain",
        arg: MatcherArg::Contained,
    },
    MatcherDef {
        name: "toMatch",
        arg: MatcherArg::Text,
    },
    MatcherDef {
        name: "toThrow",
        arg: MatcherArg::Throws,
    },
];

pub fn matcher_signature(def: &MatcherDef) -> String {
    let params = match def.arg {
        MatcherArg::None => "",
        MatcherArg::Any | MatcherArg::Numeric => "expected",
        MatcherArg::Length => "length",
        MatcherArg::Text => "text",
        MatcherArg::Contained => "value",
        MatcherArg::Throws => "message?",
    };
    format!("{}({params})", def.name)
}

pub fn matcher_summary(name: &str) -> &'static str {
    match name {
        "toBe" => "Passes when the scalar value equals `expected`.",
        "toEqual" => "Passes when the scalar value equals `expected`.",
        "toStrictEqual" => "Passes when the scalar value equals `expected`.",
        "toBeTruthy" => "Passes when the value is truthy.",
        "toBeFalsy" => "Passes when the value is falsy.",
        "toBeNull" => "Passes when the optional value is absent.",
        "toBeUndefined" => "Passes when the optional value is absent.",
        "toBeDefined" => "Passes when the optional value is present.",
        "toBeGreaterThan" => "Passes when the number is greater than `expected`.",
        "toBeGreaterThanOrEqual" => {
            "Passes when the number is greater than or equal to `expected`."
        }
        "toBeLessThan" => "Passes when the number is less than `expected`.",
        "toBeLessThanOrEqual" => "Passes when the number is less than or equal to `expected`.",
        "toBeCloseTo" => "Passes when the float is within `0.005` of `expected`.",
        "toHaveLength" => "Passes when the string or array has `length` elements.",
        "toContain" => "Passes when the string contains `text`.",
        "toMatch" => "Passes when the string contains `text`.",
        "toThrow" => {
            "Calls the function, which must declare `throws` and take no arguments, and passes when it throws; with `message`, the thrown `string` must equal it."
        }
        _ => "",
    }
}

pub fn modifier_help(owner: &str, modifier: TestModifier) -> (String, &'static str) {
    match modifier {
        TestModifier::Skip => (
            format!("{owner}.skip(name, callback)"),
            "Lists the test as skipped without running it; the callback still has to compile.",
        ),
        TestModifier::Only => (
            format!("{owner}.only(name, callback)"),
            "Focuses the file: when anything is focused, only focused tests run.",
        ),
        TestModifier::Todo => (
            format!("{owner}.todo(name)"),
            "Lists a test that has not been written yet.",
        ),
        TestModifier::Each => (
            format!("{owner}.each(name, cases, callback)"),
            "Registers one test per case, named `name [0]`, `name [1]`, and so on. The cases must be an array literal.",
        ),
    }
}

pub fn matcher_def(name: &str) -> Option<&'static MatcherDef> {
    MATCHERS.iter().find(|candidate| candidate.name == name)
}

pub const INAPPLICABLE_MATCHERS: &[(&str, &str, &str)] = &[
    (
        "toHaveProperty",
        "a field is resolved at compile time, so this can only ever be true",
        "assert the field itself, as in `expect(user.email).toBeDefined()`",
    ),
    (
        "toBeInstanceOf",
        "tyrus has no class instances to test for",
        "narrow the value instead, with `match` or an optional check, and assert what the narrowed value holds",
    ),
];

pub fn inapplicable_matcher(name: &str) -> Option<(&'static str, &'static str)> {
    INAPPLICABLE_MATCHERS
        .iter()
        .find(|(candidate, ..)| *candidate == name)
        .map(|(_, reason, help)| (*reason, *help))
}

pub fn is_test_target(target: &str) -> bool {
    matches!(
        target,
        DESCRIBE_TARGET
            | TEST_TARGET
            | IT_TARGET
            | BEFORE_ALL_TARGET
            | AFTER_ALL_TARGET
            | BEFORE_EACH_TARGET
            | AFTER_EACH_TARGET
            | EXPECT_TARGET
    )
}

pub fn is_registrar_target(name: &str) -> bool {
    matches!(name, DESCRIBE_TARGET | TEST_TARGET | IT_TARGET)
}

pub fn registrar_display(target: &str) -> &'static str {
    match target {
        DESCRIBE_TARGET => "describe",
        IT_TARGET => "it",
        _ => "test",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestModifier {
    Skip,
    Only,
    Todo,
    Each,
}

pub fn test_modifier(target: &str, method: &str) -> Option<TestModifier> {
    let modifier = match method {
        "skip" => TestModifier::Skip,
        "only" => TestModifier::Only,
        "todo" => TestModifier::Todo,
        "each" => TestModifier::Each,
        _ => return None,
    };
    match target {
        TEST_TARGET | IT_TARGET => Some(modifier),
        DESCRIBE_TARGET if !matches!(modifier, TestModifier::Todo | TestModifier::Each) => {
            Some(modifier)
        }
        _ => None,
    }
}

pub const UNIMPLEMENTED_MODIFIERS: &[&str] =
    &["concurrent", "fails", "sequential", "skipIf", "runIf"];

pub fn expectation_actual(
    expr: &Expr,
    is_expect: impl Fn(&str) -> bool,
) -> Option<(&Expr, Expectation)> {
    let mut receiver = expr;
    let mut expectation = Expectation::default();
    loop {
        match receiver.kind() {
            ExprKind::MethodCall {
                object: inner_object,
                method: inner_method,
                ..
            }
            | ExprKind::FieldAccess {
                object: inner_object,
                field: inner_method,
            } if is_chain_modifier(inner_method) => {
                match inner_method.as_str() {
                    REJECTS_MODIFIER => expectation.rejects = true,
                    RESOLVES_MODIFIER => expectation.resolves = true,
                    _ => {}
                }
                receiver = inner_object;
            }
            _ => break,
        }
    }
    match receiver.kind() {
        ExprKind::FnCall { callee, args, .. } if is_expect(callee) && args.len() == 1 => {
            Some((&args[0], expectation))
        }
        _ => None,
    }
}

pub fn chain_modifier_summary(modifier: &str) -> Option<&'static str> {
    match modifier {
        NOT_MODIFIER => Some("Inverts the matcher that follows."),
        REJECTS_MODIFIER => Some(
            "Settles the promise and asserts that it failed. Followed by `toThrow()`, or `toThrow(message)` to require the message as well.",
        ),
        RESOLVES_MODIFIER => Some(
            "Settles the promise and hands the matcher that follows the value it resolved to. A promise that failed instead fails the assertion.",
        ),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Expectation {
    pub rejects: bool,
    pub resolves: bool,
}

pub fn program_uses_test_framework(program: &Program) -> bool {
    program.items.iter().any(item_uses_framework)
}

fn item_uses_framework(item: &Item) -> bool {
    match item {
        Item::FnDecl { body, .. } => stmts_use_framework(body),
        Item::Stmt(stmt) => stmt_uses_framework(stmt),
        _ => false,
    }
}

fn stmts_use_framework(stmts: &[Stmt]) -> bool {
    stmts.iter().any(stmt_uses_framework)
}

fn stmt_uses_framework(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::VarDecl { value, onfail, .. } => {
            value.as_ref().is_some_and(expr_uses_framework)
                || onfail.as_ref().is_some_and(|onfail| match &onfail.handler {
                    crate::ast::OnFailHandler::Expr(expr) => expr_uses_framework(expr),
                    crate::ast::OnFailHandler::Block(stmts) => stmts_use_framework(stmts),
                })
        }
        StmtKind::ArrayDestructure { value, .. } | StmtKind::ObjectDestructure { value, .. } => {
            expr_uses_framework(value)
        }
        StmtKind::Assign { value, .. } | StmtKind::Throw(value) => expr_uses_framework(value),
        StmtKind::IndexAssign { index, value, .. } => {
            expr_uses_framework(index) || expr_uses_framework(value)
        }
        StmtKind::FieldAssign { object, value, .. } => {
            expr_uses_framework(object) || expr_uses_framework(value)
        }
        StmtKind::Return(Some(value)) | StmtKind::Break(Some(value)) => expr_uses_framework(value),
        StmtKind::Return(None) | StmtKind::Break(None) | StmtKind::Continue => false,
        StmtKind::Defer { body, .. } => stmts_use_framework(body),
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            stmts_use_framework(try_body)
                || stmts_use_framework(catch_body)
                || finally_body.as_deref().is_some_and(stmts_use_framework)
        }
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            expr_uses_framework(cond) || stmts_use_framework(body)
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            stmt_uses_framework(init)
                || expr_uses_framework(cond)
                || stmt_uses_framework(update)
                || stmts_use_framework(body)
        }
        StmtKind::ForOf { iterable, body, .. } => {
            expr_uses_framework(iterable) || stmts_use_framework(body)
        }
        StmtKind::ForIn { object, body, .. } => {
            expr_uses_framework(object) || stmts_use_framework(body)
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            expr_uses_framework(expr)
                || cases
                    .iter()
                    .any(|case| expr_uses_framework(&case.value) || stmts_use_framework(&case.body))
                || default.as_deref().is_some_and(stmts_use_framework)
        }
        StmtKind::Match {
            expr,
            arms,
            default,
        } => {
            expr_uses_framework(expr)
                || arms.iter().any(|arm| {
                    arm.guard.as_ref().is_some_and(expr_uses_framework)
                        || stmts_use_framework(&arm.body)
                })
                || default.as_deref().is_some_and(stmts_use_framework)
        }
        StmtKind::Expr(expr) => expr_uses_framework(expr),
    }
}

fn expr_uses_framework(expr: &Expr) -> bool {
    match &expr.kind {
        ExprKind::StrLit(_)
        | ExprKind::RegexLit { .. }
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::DecimalLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::None
        | ExprKind::Topic
        | ExprKind::Borrow { .. } => false,
        ExprKind::Ident(name) => is_test_target(name),
        ExprKind::FnCall { callee, args, .. } => {
            is_test_target(callee) || args.iter().any(expr_uses_framework)
        }
        ExprKind::MethodCall { object, args, .. }
        | ExprKind::OptionalMethodCall { object, args, .. } => {
            expr_uses_framework(object) || args.iter().any(expr_uses_framework)
        }
        ExprKind::CallValue { callee, args, .. } => {
            expr_uses_framework(callee) || args.iter().any(expr_uses_framework)
        }
        ExprKind::FieldAccess { object, .. }
        | ExprKind::Typeof(object)
        | ExprKind::Await(object) => expr_uses_framework(object),
        ExprKind::IndexAccess { object, index }
        | ExprKind::BinOp {
            left: object,
            right: index,
            ..
        }
        | ExprKind::In {
            key: object,
            object: index,
        }
        | ExprKind::Range {
            start: object,
            end: index,
            ..
        } => expr_uses_framework(object) || expr_uses_framework(index),
        ExprKind::ChainedComparison { terms, .. } => terms.iter().any(expr_uses_framework),
        ExprKind::Pipeline { left, right } => {
            expr_uses_framework(left) || expr_uses_framework(right)
        }
        ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::BitNot(inner)
        | ExprKind::PrefixIncr(inner)
        | ExprKind::PrefixDecr(inner)
        | ExprKind::PostfixIncr(inner)
        | ExprKind::PostfixDecr(inner)
        | ExprKind::Unwrap(inner)
        | ExprKind::Try(inner)
        | ExprKind::Spread(inner)
        | ExprKind::ConstAssert(inner)
        | ExprKind::Comptime(inner) => expr_uses_framework(inner),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            expr_uses_framework(cond)
                || expr_uses_framework(then_expr)
                || expr_uses_framework(else_expr)
        }
        ExprKind::TypeAssert { value, .. } | ExprKind::Is { expr: value, .. } => {
            expr_uses_framework(value)
        }
        ExprKind::ObjectLit(fields) => fields.iter().any(|(_, value)| expr_uses_framework(value)),
        ExprKind::ObjectUpdate { base, fields } => {
            expr_uses_framework(base) || fields.iter().any(|(_, value)| expr_uses_framework(value))
        }
        ExprKind::ArrayLit(values) => values.iter().any(expr_uses_framework),
        ExprKind::ArrayRepeat { value, .. } => expr_uses_framework(value),
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => initial
            .as_ref()
            .is_some_and(|initial| expr_uses_framework(initial)),
        ExprKind::TemplateLit(parts) => parts.iter().any(|part| match part {
            TemplatePart::Text(_) => false,
            TemplatePart::Expr(expr, _) => expr_uses_framework(expr),
        }),
        ExprKind::OptionalFieldAccess { object, .. } => expr_uses_framework(object),
        ExprKind::ArrowFn { body, .. } => stmts_use_framework(body),
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            expr_uses_framework(cond)
                || stmts_use_framework(then_block)
                || else_block.as_deref().is_some_and(stmts_use_framework)
        }
        ExprKind::Loop { body } => stmts_use_framework(body),
    }
}

fn export(target: &str, ty: Type) -> ExportInfo {
    ExportInfo {
        target: target.to_string(),
        ty,
        interface_fields: None,
        behavior_info: None,
        type_alias_ty: None,
        enum_info: None,
        throws: Vec::new(),
        db_model: None,
    }
}

pub(crate) fn test_builtins() -> HashMap<String, ExportInfo> {
    let mut builtins = HashMap::default();
    builtins.insert(
        "describe".to_string(),
        export(DESCRIBE_TARGET, describe_type()),
    );
    builtins.insert("test".to_string(), export(TEST_TARGET, test_type()));
    builtins.insert("it".to_string(), export(IT_TARGET, test_type()));
    builtins.insert(
        "beforeAll".to_string(),
        export(BEFORE_ALL_TARGET, hook_type()),
    );
    builtins.insert(
        "afterAll".to_string(),
        export(AFTER_ALL_TARGET, hook_type()),
    );
    builtins.insert(
        "beforeEach".to_string(),
        export(BEFORE_EACH_TARGET, hook_type()),
    );
    builtins.insert(
        "afterEach".to_string(),
        export(AFTER_EACH_TARGET, hook_type()),
    );
    builtins.insert("expect".to_string(), export(EXPECT_TARGET, expect_type()));
    builtins
}
