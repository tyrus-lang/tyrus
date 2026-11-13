//! Which local arrays never leave the scope that created them.
//!
//! A `let numbers: i32[] = [1, 2, 3]` inside a loop allocates a heap buffer on
//! every iteration and frees it at the end of the same one. If nothing can
//! observe that buffer after the scope ends, the allocation is pure ceremony
//! and the bytes could live on the stack, measured at **3,27×** on a loop
//! dominated by exactly that shape.
//!
//! # Why this errs toward "it escapes"
//!
//! Getting this wrong in the safe direction costs an allocation nobody needed.
//! Getting it wrong in the other direction puts a value on a stack frame that
//! outlives it, and a read after that frame dies is a use-after-return: the
//! worst class of bug, and one a leak corpus cannot see, because memory spent
//! on the stack is never leaked memory.
//!
//! So the question asked here is deliberately narrow. A binding qualifies only
//! when *every* mention of its name in the whole function is one of two shapes:
//!
//! * indexing it, `numbers[i]`, including on the left of an assignment, which
//!   writes an element without moving the buffer; and
//! * reading its length, `numbers.length`.
//!
//! Anything else disqualifies it, including uses this analysis could in
//! principle prove safe. Returning it, assigning it to another name, passing it
//! to a function, putting it in an object, capturing it in a closure, calling
//! any method on it, all escape, and `push` escapes twice over, since growing
//! a buffer means replacing it and a stack slot cannot be replaced.
//!
//! # Why the walk has no wildcard arm
//!
//! The soundness of the whole analysis rests on seeing *every* mention. A match
//! arm that quietly ignores a node it does not recognize would not fail: it
//! would silently conclude that a binding is contained when it is not. So the
//! traversal below matches every variant of `ExprKind` and `StmtKind`
//! explicitly, with no `_ =>`. Adding a node to the AST therefore breaks this
//! file, which is the intended behaviour, somebody has to decide whether the
//! new node can carry an array out of its scope.

use crate::ast::{Expr, ExprKind, Item, Program, Stmt, StmtKind, Type};
use crate::collections::HashSet;
use crate::span::Span;

#[derive(Debug, Default, Clone)]
pub struct ContainedArrays {
    spans: HashSet<Span>,
    names: Vec<String>,
}

impl ContainedArrays {
    pub fn contains(&self, span: Span) -> bool {
        self.spans.contains(&span)
    }

    pub fn len(&self) -> usize {
        self.spans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Subject {
    Array,
    Counter,
    Closure,
}

#[derive(Debug, Default, Clone)]
pub struct ConfinedClosures {
    spans: HashSet<Span>,
    literals: HashSet<Span>,
    names: Vec<String>,
}

impl ConfinedClosures {
    pub fn contains(&self, span: Span) -> bool {
        self.spans.contains(&span)
    }

    pub fn contains_literal(&self, span: Span) -> bool {
        self.literals.contains(&span)
    }

    pub fn len(&self) -> usize {
        self.spans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }
}

pub fn analyze_confined_closures(program: &Program) -> ConfinedClosures {
    let mut confined = ConfinedClosures::default();
    for item in &program.items {
        match item {
            Item::FnDecl { body, .. } => confined_in_body(body, &mut confined),
            Item::Stmt(stmt) => confined_in_nested_bodies(stmt, &mut confined),
            Item::InterfaceDecl { .. }
            | Item::BehaviorInterfaceDecl { .. }
            | Item::TypeAlias { .. }
            | Item::EnumDecl { .. } => {}
        }
    }
    confined
}

fn confined_in_body(body: &[Stmt], confined: &mut ConfinedClosures) {
    for stmt in body {
        if let StmtKind::VarDecl { name, value, .. } = stmt.kind()
            && is_synchronous_closure_literal(value.as_ref())
            && !leaves_frame(name, body, Subject::Closure)
        {
            confined.spans.insert(stmt.span);
            if let Some(literal) = value.as_ref() {
                confined.literals.insert(literal.span);
            }
            confined.names.push(name.clone());
        }
    }
    for stmt in body {
        confined_in_nested_bodies(stmt, confined);
    }
}

fn confined_in_nested_bodies(stmt: &Stmt, confined: &mut ConfinedClosures) {
    for nested in nested_bodies(stmt) {
        confined_in_body(nested, confined);
    }
    for (nested, _) in expression_bodies(stmt) {
        confined_in_body(&nested, confined);
    }
}

pub fn closure_assigned_captures(
    params: &[crate::ast::Param],
    body: &[Stmt],
) -> Vec<(String, Span)> {
    let mut declared = HashSet::default();
    for param in params {
        for name in param.pattern.binding_names() {
            declared.insert(name.to_string());
        }
    }
    let mut assigned = Vec::new();
    collect_assignments(body, &mut declared, &mut assigned);
    assigned.sort_by(|left, right| left.0.cmp(&right.0));
    assigned.dedup_by(|left, right| left.0 == right.0);
    assigned
}

fn collect_assignments(
    body: &[Stmt],
    declared: &mut HashSet<String>,
    assigned: &mut Vec<(String, Span)>,
) {
    for stmt in body {
        match stmt.kind() {
            StmtKind::VarDecl { name, .. } => {
                declared.insert(name.clone());
            }
            StmtKind::Assign { name, .. } => {
                if !declared.contains(name) {
                    assigned.push((name.clone(), stmt.span));
                }
            }
            StmtKind::Expr(expr) => collect_assigning_expr(expr, declared, assigned, stmt.span),
            _ => {}
        }
        for nested in nested_bodies(stmt) {
            let mut inner = declared.clone();
            collect_assignments(nested, &mut inner, assigned);
        }
    }
}

fn collect_assigning_expr(
    expr: &Expr,
    declared: &HashSet<String>,
    assigned: &mut Vec<(String, Span)>,
    span: Span,
) {
    if let ExprKind::PrefixIncr(inner)
    | ExprKind::PrefixDecr(inner)
    | ExprKind::PostfixIncr(inner)
    | ExprKind::PostfixDecr(inner) = expr.kind()
        && let ExprKind::Ident(name) = inner.kind()
        && !declared.contains(name)
    {
        assigned.push((name.clone(), span));
    }
}

fn is_synchronous_closure_literal(value: Option<&Expr>) -> bool {
    matches!(
        value.map(Expr::kind),
        Some(ExprKind::ArrowFn {
            is_async: false,
            ..
        })
    )
}

#[derive(Debug, Default, Clone)]
pub struct ConstantGlobalArrays {
    spans: HashSet<Span>,
    names: Vec<String>,
}

impl ConstantGlobalArrays {
    pub fn contains(&self, span: Span) -> bool {
        self.spans.contains(&span)
    }

    pub fn len(&self) -> usize {
        self.spans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.spans.is_empty()
    }

    pub fn names(&self) -> &[String] {
        &self.names
    }
}

pub fn analyze_constant_global_arrays(program: &Program) -> ConstantGlobalArrays {
    let mut constant = ConstantGlobalArrays::default();
    let top_level: Vec<Stmt> = program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Stmt(stmt) => Some(stmt.clone()),
            _ => None,
        })
        .collect();
    for item in &program.items {
        let Item::Stmt(stmt) = item else {
            continue;
        };
        let StmtKind::VarDecl {
            mutable,
            name,
            ty,
            value: Some(value),
            ..
        } = stmt.kind()
        else {
            continue;
        };
        if *mutable || !is_constant_array_literal(ty, value) {
            continue;
        }
        let escapes_anywhere = leaves_frame(name, &top_level, Subject::Array)
            || program.items.iter().any(|other| match other {
                Item::FnDecl { body, .. } => leaves_frame(name, body, Subject::Array),
                _ => false,
            });
        if !escapes_anywhere {
            constant.spans.insert(stmt.span);
            constant.names.push(name.clone());
        }
    }
    constant
}

fn is_constant_array_literal(ty: &Type, value: &Expr) -> bool {
    let ty = match ty {
        Type::Readonly(inner) => inner.as_ref(),
        ty => ty,
    };
    if !matches!(ty, Type::Array(_)) {
        return false;
    }
    let ExprKind::ArrayLit(elements) = value.kind() else {
        return false;
    };
    !elements.is_empty() && elements.iter().all(is_constant_element)
}

fn is_constant_element(element: &Expr) -> bool {
    match element.kind() {
        ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::StrLit(_) => true,
        ExprKind::Neg(inner) => matches!(
            inner.kind(),
            ExprKind::IntLit(_) | ExprKind::UIntLit(_) | ExprKind::FloatLit(_)
        ),
        _ => false,
    }
}

pub fn analyze(program: &Program) -> ContainedArrays {
    let mut contained = ContainedArrays::default();
    for item in &program.items {
        match item {
            Item::FnDecl { body, is_async, .. } => analyze_body(body, &mut contained, *is_async),
            Item::Stmt(stmt) => analyze_nested_bodies(stmt, &mut contained, false),
            Item::InterfaceDecl { .. }
            | Item::BehaviorInterfaceDecl { .. }
            | Item::TypeAlias { .. }
            | Item::EnumDecl { .. } => {}
        }
    }
    contained
}

fn analyze_body(body: &[Stmt], contained: &mut ContainedArrays, in_async: bool) {
    if !in_async {
        for stmt in body {
            if let StmtKind::VarDecl {
                name, ty, value, ..
            } = stmt.kind()
                && is_fixed_size_array_literal(ty, value.as_ref())
                && !escapes(name, body)
            {
                contained.spans.insert(stmt.span);
                contained.names.push(name.clone());
            }
        }
    }
    for stmt in body {
        analyze_nested_bodies(stmt, contained, in_async);
    }
}

fn analyze_nested_bodies(stmt: &Stmt, contained: &mut ContainedArrays, in_async: bool) {
    for nested in nested_bodies(stmt) {
        analyze_body(nested, contained, in_async);
    }
    for (nested, is_async) in expression_bodies(stmt) {
        analyze_body(&nested, contained, in_async || is_async);
    }
}

fn is_fixed_size_array_literal(ty: &Type, value: Option<&Expr>) -> bool {
    if !matches!(ty, Type::Array(_)) {
        return false;
    }
    match value.map(Expr::kind) {
        Some(ExprKind::ArrayLit(elements)) => !elements
            .iter()
            .any(|element| matches!(element.kind(), ExprKind::Spread(_))),
        Some(ExprKind::ArrayRepeat { .. }) => true,
        _ => false,
    }
}

fn escapes(name: &str, body: &[Stmt]) -> bool {
    leaves_frame(name, body, Subject::Array)
}

pub(crate) fn leaves_frame(name: &str, body: &[Stmt], subject: Subject) -> bool {
    let mut walker = Walker {
        name,
        escaped: false,
        in_closure: false,
        subject,
    };
    walker.statements(body);
    walker.escaped
}

struct Walker<'a> {
    name: &'a str,
    escaped: bool,
    in_closure: bool,
    subject: Subject,
}

impl Walker<'_> {
    fn statements(&mut self, statements: &[Stmt]) {
        for statement in statements {
            self.statement(statement);
        }
    }

    fn mention(&mut self, expr: &Expr) {
        if !matches!(expr.kind(), ExprKind::Ident(ident) if ident == self.name) {
            return;
        }
        if self.subject == Subject::Counter && !self.in_closure {
            return;
        }
        self.escaped = true;
    }

    fn write(&mut self, target: &Expr) {
        if matches!(target.kind(), ExprKind::Ident(ident) if ident == self.name) {
            self.escaped = true;
        }
    }

    fn expr(&mut self, expr: &Expr) {
        if self.escaped {
            return;
        }
        match expr.kind() {
            ExprKind::IndexAccess { object, index } => {
                if self.subject != Subject::Array
                    || self.in_closure
                    || !matches!(object.kind(), ExprKind::Ident(ident) if ident == self.name)
                {
                    self.expr(object);
                }
                self.expr(index);
            }
            ExprKind::FieldAccess { object, field } => {
                if self.subject != Subject::Array
                    || self.in_closure
                    || field != "length"
                    || !matches!(object.kind(), ExprKind::Ident(ident) if ident == self.name)
                {
                    self.expr(object);
                }
            }

            ExprKind::Ident(_) => self.mention(expr),
            ExprKind::Borrow { name, .. } => {
                if name == self.name {
                    self.escaped = true;
                }
            }

            ExprKind::StrLit(_)
            | ExprKind::RegexLit { .. }
            | ExprKind::IntLit(_)
            | ExprKind::UIntLit(_)
            | ExprKind::FloatLit(_)
            | ExprKind::BoolLit(_)
            | ExprKind::DecimalLit(_)
            | ExprKind::Null
            | ExprKind::None
            | ExprKind::Topic => {}

            ExprKind::Try(inner) | ExprKind::Comptime(inner) => self.expr(inner),

            ExprKind::Loop { body } => self.statements(body),

            ExprKind::NewSet { initial, .. } => {
                if let Some(initial) = initial {
                    self.expr(initial);
                }
            }
            ExprKind::NewMap { initial, .. } => {
                if let Some(initial) = initial {
                    self.expr(initial);
                }
            }
            ExprKind::TemplateLit(parts) => {
                for part in parts {
                    if let crate::ast::TemplatePart::Expr(expr, _) = part {
                        self.expr(expr);
                    }
                }
            }
            ExprKind::ObjectLit(fields) => {
                for (_, value) in fields {
                    self.expr(value);
                }
            }
            ExprKind::ObjectUpdate { base, fields } => {
                self.expr(base);
                for (_, value) in fields {
                    self.expr(value);
                }
            }
            ExprKind::ArrayLit(elements) => {
                for element in elements {
                    self.expr(element);
                }
            }
            ExprKind::ArrayRepeat { value, count } => {
                self.expr(value);
                self.expr(count);
            }
            ExprKind::BinOp { left, right, .. } | ExprKind::Pipeline { left, right } => {
                self.expr(left);
                self.expr(right);
            }
            ExprKind::ChainedComparison { terms, .. } => {
                for term in terms {
                    self.expr(term);
                }
            }
            ExprKind::PrefixIncr(inner)
            | ExprKind::PrefixDecr(inner)
            | ExprKind::PostfixIncr(inner)
            | ExprKind::PostfixDecr(inner) => {
                self.write(inner);
                self.expr(inner);
            }
            ExprKind::Not(inner)
            | ExprKind::Neg(inner)
            | ExprKind::BitNot(inner)
            | ExprKind::Unwrap(inner)
            | ExprKind::Typeof(inner)
            | ExprKind::Await(inner)
            | ExprKind::Spread(inner)
            | ExprKind::ConstAssert(inner) => self.expr(inner),
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.expr(cond);
                self.expr(then_expr);
                self.expr(else_expr);
            }
            ExprKind::TypeAssert { value, .. } => self.expr(value),
            ExprKind::Is { expr, .. } => self.expr(expr),
            ExprKind::OptionalFieldAccess { object, .. } => self.expr(object),
            ExprKind::OptionalMethodCall { object, args, .. }
            | ExprKind::MethodCall { object, args, .. } => {
                self.expr(object);
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::FnCall { args, .. } => {
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::CallValue { callee, args, .. } => {
                self.expr(callee);
                for arg in args {
                    self.expr(arg);
                }
            }
            ExprKind::ArrowFn { body, .. } => {
                let outer = self.in_closure;
                self.in_closure = true;
                self.statements(body);
                self.in_closure = outer;
            }
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                self.expr(cond);
                self.statements(then_block);
                if let Some(else_block) = else_block {
                    self.statements(else_block);
                }
            }
            ExprKind::In { key, object } => {
                self.expr(key);
                self.expr(object);
            }
            ExprKind::Range { start, end, .. } => {
                self.expr(start);
                self.expr(end);
            }
        }
    }

    fn statement(&mut self, statement: &Stmt) {
        if self.escaped {
            return;
        }
        match statement.kind() {
            StmtKind::VarDecl { value, .. } => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            StmtKind::Assign { name, value, .. } => {
                if name == self.name {
                    self.escaped = true;
                }
                self.expr(value);
            }
            StmtKind::IndexAssign { name, index, value } => {
                if self.in_closure && name == self.name {
                    self.escaped = true;
                }
                let _ = name;
                self.expr(index);
                self.expr(value);
            }
            StmtKind::FieldAssign { object, value, .. } => {
                self.expr(object);
                self.expr(value);
            }
            StmtKind::Return(value) => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            StmtKind::Throw(value) | StmtKind::Expr(value) => self.expr(value),
            StmtKind::ArrayDestructure { value, .. }
            | StmtKind::ObjectDestructure { value, .. } => self.expr(value),
            StmtKind::While { cond, body } | StmtKind::DoWhile { cond, body } => {
                self.expr(cond);
                self.statements(body);
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                self.statement(init);
                self.expr(cond);
                self.statement(update);
                self.statements(body);
            }
            StmtKind::ForOf { iterable, body, .. } => {
                self.expr(iterable);
                self.statements(body);
            }
            StmtKind::ForIn { object, body, .. } => {
                self.expr(object);
                self.statements(body);
            }
            StmtKind::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                self.expr(expr);
                for case in cases {
                    self.expr(&case.value);
                    self.statements(&case.body);
                }
                if let Some(default) = default {
                    self.statements(default);
                }
            }
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                self.expr(expr);
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        self.expr(guard);
                    }
                    self.statements(&arm.body);
                }
                if let Some(default) = default {
                    self.statements(default);
                }
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                self.statements(try_body);
                self.statements(catch_body);
                if let Some(finally_body) = finally_body {
                    self.statements(finally_body);
                }
            }
            StmtKind::Defer { body, .. } => self.statements(body),
            StmtKind::Break(value) => {
                if let Some(value) = value {
                    self.expr(value);
                }
            }
            StmtKind::Continue => {}
        }
    }
}

pub(crate) fn expression_bodies(statement: &Stmt) -> Vec<(Vec<Stmt>, bool)> {
    let mut bodies = Vec::new();
    let mut collect = |expr: &Expr| collect_expression_bodies(expr, &mut bodies);
    match statement.kind() {
        StmtKind::VarDecl { value, .. } => {
            if let Some(value) = value {
                collect(value);
            }
        }
        StmtKind::Assign { value, .. }
        | StmtKind::Throw(value)
        | StmtKind::Expr(value)
        | StmtKind::ArrayDestructure { value, .. }
        | StmtKind::ObjectDestructure { value, .. } => collect(value),
        StmtKind::Return(value) => {
            if let Some(value) = value {
                collect(value);
            }
        }
        StmtKind::IndexAssign { index, value, .. } => {
            collect(index);
            collect(value);
        }
        StmtKind::FieldAssign { object, value, .. } => {
            collect(object);
            collect(value);
        }
        StmtKind::While { cond, .. } | StmtKind::DoWhile { cond, .. } => collect(cond),
        StmtKind::For { cond, .. } => collect(cond),
        StmtKind::ForOf { iterable, .. } => collect(iterable),
        StmtKind::ForIn { object, .. } => collect(object),
        StmtKind::Switch { expr, .. } | StmtKind::Match { expr, .. } => collect(expr),
        StmtKind::TryCatch { .. }
        | StmtKind::Defer { .. }
        | StmtKind::Break(_)
        | StmtKind::Continue => {}
    }
    bodies
}

fn collect_expression_bodies(expr: &Expr, bodies: &mut Vec<(Vec<Stmt>, bool)>) {
    match expr.kind() {
        ExprKind::ArrowFn { body, is_async, .. } => bodies.push((body.clone(), *is_async)),
        ExprKind::If {
            then_block,
            else_block,
            ..
        } => {
            bodies.push((then_block.clone(), false));
            if let Some(else_block) = else_block {
                bodies.push((else_block.clone(), false));
            }
        }
        _ => {}
    }
    for child in child_expressions(expr) {
        collect_expression_bodies(child, bodies);
    }
}

pub(crate) fn child_expressions(expr: &Expr) -> Vec<&Expr> {
    let mut children: Vec<&Expr> = Vec::new();
    match expr.kind() {
        ExprKind::DecimalLit(_) | ExprKind::None => {}
        ExprKind::Try(inner) | ExprKind::Comptime(inner) => children.push(inner),
        ExprKind::Loop { .. } => {}
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            if let Some(initial) = initial {
                children.push(initial);
            }
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let crate::ast::TemplatePart::Expr(expr, _) = part {
                    children.push(expr);
                }
            }
        }
        ExprKind::ObjectLit(fields) => children.extend(fields.iter().map(|(_, value)| value)),
        ExprKind::ObjectUpdate { base, fields } => {
            children.push(base);
            children.extend(fields.iter().map(|(_, value)| value));
        }
        ExprKind::ArrayLit(elements) => children.extend(elements.iter()),
        ExprKind::ArrayRepeat { value, count } => {
            children.push(value);
            children.push(count);
        }
        ExprKind::BinOp { left, right, .. } | ExprKind::Pipeline { left, right } => {
            children.push(left);
            children.push(right);
        }
        ExprKind::ChainedComparison { terms, .. } => children.extend(terms.iter()),
        ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::BitNot(inner)
        | ExprKind::PrefixIncr(inner)
        | ExprKind::PrefixDecr(inner)
        | ExprKind::PostfixIncr(inner)
        | ExprKind::PostfixDecr(inner)
        | ExprKind::Unwrap(inner)
        | ExprKind::Typeof(inner)
        | ExprKind::Await(inner)
        | ExprKind::Spread(inner)
        | ExprKind::ConstAssert(inner) => children.push(inner),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            children.push(cond);
            children.push(then_expr);
            children.push(else_expr);
        }
        ExprKind::TypeAssert { value, .. } => children.push(value),
        ExprKind::Is { expr, .. } => children.push(expr),
        ExprKind::IndexAccess { object, index } => {
            children.push(object);
            children.push(index);
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            children.push(object);
        }
        ExprKind::OptionalMethodCall { object, args, .. }
        | ExprKind::MethodCall { object, args, .. } => {
            children.push(object);
            children.extend(args.iter());
        }
        ExprKind::FnCall { args, .. } => children.extend(args.iter()),
        ExprKind::CallValue { callee, args, .. } => {
            children.push(callee);
            children.extend(args.iter());
        }
        ExprKind::In { key, object } => {
            children.push(key);
            children.push(object);
        }
        ExprKind::Range { start, end, .. } => {
            children.push(start);
            children.push(end);
        }
        ExprKind::StrLit(_)
        | ExprKind::RegexLit { .. }
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::Topic
        | ExprKind::Ident(_)
        | ExprKind::Borrow { .. }
        | ExprKind::ArrowFn { .. }
        | ExprKind::If { .. } => {}
    }
    children
}

pub(crate) fn nested_bodies(statement: &Stmt) -> Vec<&[Stmt]> {
    match statement.kind() {
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::ForIn { body, .. } => vec![body.as_slice()],
        StmtKind::Switch { cases, default, .. } => {
            let mut bodies: Vec<&[Stmt]> = cases.iter().map(|case| case.body.as_slice()).collect();
            if let Some(default) = default {
                bodies.push(default.as_slice());
            }
            bodies
        }
        StmtKind::Match { arms, default, .. } => {
            let mut bodies: Vec<&[Stmt]> = arms.iter().map(|arm| arm.body.as_slice()).collect();
            if let Some(default) = default {
                bodies.push(default.as_slice());
            }
            bodies
        }
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            let mut bodies = vec![try_body.as_slice(), catch_body.as_slice()];
            if let Some(finally_body) = finally_body {
                bodies.push(finally_body.as_slice());
            }
            bodies
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
#[path = "../../tests/unit/escape_analysis.rs"]
mod tests;
