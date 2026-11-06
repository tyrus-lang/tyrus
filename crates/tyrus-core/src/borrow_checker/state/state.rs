use super::BorrowChecker;
use super::liveness::closure_capture_names;
use crate::ast::*;
use crate::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub(crate) struct VarState {
    pub(super) mutable: bool,
    pub(super) ty: Type,
    pub(super) scope_level: usize,
    pub(super) is_parameter: bool,
    pub(super) availability: Availability,
    pub(super) borrow: BorrowState,
    pub(super) active_borrow: Option<ActiveBorrow>,
    pub(super) known_non_null: bool,
    pub(super) dead: bool,
    pub(super) moved_fields: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Availability {
    Available,
    Moved,
    Dropped,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum BorrowState {
    None,
    Immutable(usize),
    Mutable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveBorrow {
    pub(super) source: String,
    pub(super) mutable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ReferenceOrigin {
    pub(super) source: String,
    pub(super) source_scope_level: usize,
    pub(super) source_is_parameter: bool,
    pub(super) source_ty: Type,
}

pub(crate) type Scope = HashMap<String, VarState>;

#[derive(Debug, Clone)]
pub(crate) struct ScheduledFinalizer {
    pub(super) captures: HashSet<String>,
    pub(super) consumes: HashSet<String>,
}

impl BorrowChecker {
    pub(super) fn push_scope(&mut self) {
        self.scopes.push(HashMap::default());
        self.finalizer_scopes.push(Vec::new());
    }

    pub(super) fn pop_scope(&mut self) {
        debug_assert_eq!(self.finalizer_scopes.len(), self.scopes.len());
        self.scopes.pop();
        self.finalizer_scopes.pop();
        self.refresh_persistent_borrows();
    }

    pub(super) fn register_finalizer(&mut self, body: &[Stmt]) {
        let consumes = self.finalizer_consumptions(body);
        for name in &consumes {
            if self
                .finalizer_scopes
                .iter()
                .flat_map(|scope| scope.iter())
                .any(|finalizer| finalizer.consumes.contains(name))
            {
                self.error(format!(
                    "error: cannot schedule consumption of `{name}` because a pending finalizer already consumes it"
                ));
            }
        }
        self.finalizer_scopes
            .last_mut()
            .expect("the root lexical finalizer scope is always present")
            .push(ScheduledFinalizer {
                captures: closure_capture_names(&[], body),
                consumes,
            });
    }

    pub(super) fn finalizer_consumptions(&self, body: &[Stmt]) -> HashSet<String> {
        let mut consumed = HashSet::default();
        for statement in body {
            self.collect_finalizer_consumptions(statement, &mut consumed);
        }
        consumed
    }

    pub(super) fn collect_finalizer_consumptions(
        &self,
        statement: &Stmt,
        consumed: &mut HashSet<String>,
    ) {
        match statement.kind() {
            StmtKind::Expr(Expr {
                kind: ExprKind::FnCall { callee, args, .. },
                ..
            }) if self.consuming_imports.contains(callee) => {
                if let Some(Expr {
                    kind: ExprKind::Ident(name),
                    ..
                }) = args.first()
                {
                    consumed.insert(name.clone());
                }
            }
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::MethodCall {
                        object,
                        method,
                        args,
                        ..
                    },
                ..
            }) if self.replacing_imports.contains(method) => {
                if let ExprKind::Ident(name) = object.kind() {
                    consumed.insert(name.clone());
                }
                for arg in args {
                    if let ExprKind::Ident(name) = arg.kind() {
                        consumed.insert(name.clone());
                    }
                }
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                for inner in try_body {
                    self.collect_finalizer_consumptions(inner, consumed);
                }
                for inner in catch_body {
                    self.collect_finalizer_consumptions(inner, consumed);
                }
                if let Some(finally_body) = finally_body {
                    for inner in finally_body {
                        self.collect_finalizer_consumptions(inner, consumed);
                    }
                }
            }
            StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::ForIn { body, .. } => {
                for inner in body {
                    self.collect_finalizer_consumptions(inner, consumed);
                }
            }
            StmtKind::Switch { cases, default, .. } => {
                for case in cases {
                    for inner in &case.body {
                        self.collect_finalizer_consumptions(inner, consumed);
                    }
                }
                if let Some(default) = default {
                    for inner in default {
                        self.collect_finalizer_consumptions(inner, consumed);
                    }
                }
            }
            StmtKind::Match { arms, default, .. } => {
                for arm in arms {
                    for inner in &arm.body {
                        self.collect_finalizer_consumptions(inner, consumed);
                    }
                }
                if let Some(default) = default {
                    for inner in default {
                        self.collect_finalizer_consumptions(inner, consumed);
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn is_captured_by_pending_finalizer(&self, name: &str) -> bool {
        self.finalizer_scopes
            .iter()
            .flat_map(|scope| scope.iter())
            .any(|finalizer| finalizer.captures.contains(name))
    }

    pub(super) fn declare(&mut self, name: &str, mutable: bool, ty: Type) {
        self.declare_with_active_borrow(name, mutable, ty, None, false);
    }

    pub(super) fn declare_parameter(&mut self, name: &str, mutable: bool, ty: Type) {
        self.declare_with_active_borrow(name, mutable, ty, None, true);
    }

    pub(super) fn declare_with_active_borrow(
        &mut self,
        name: &str,
        mutable: bool,
        ty: Type,
        active_borrow: Option<ActiveBorrow>,
        is_parameter: bool,
    ) {
        let scope_level = self.current_scope_level();
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name.to_string(),
                VarState {
                    mutable,
                    ty,
                    scope_level,
                    is_parameter,
                    availability: Availability::Available,
                    borrow: BorrowState::None,
                    active_borrow,
                    known_non_null: false,
                    dead: false,
                    moved_fields: Vec::new(),
                },
            );
        }
        self.refresh_persistent_borrows();
    }

    pub(super) fn current_scope_level(&self) -> usize {
        self.scopes.len().saturating_sub(1)
    }

    pub(super) fn refresh_persistent_borrows(&mut self) {
        let mut persistent = HashMap::<String, BorrowState>::default();
        for scope in &self.scopes {
            for var in scope.values() {
                if var.dead {
                    continue;
                }
                let Some(active) = &var.active_borrow else {
                    continue;
                };
                let entry = persistent
                    .entry(active.source.clone())
                    .or_insert(BorrowState::None);
                if active.mutable {
                    *entry = BorrowState::Mutable;
                } else if !matches!(entry, BorrowState::Mutable) {
                    let count = match entry {
                        BorrowState::Immutable(count) => *count + 1,
                        BorrowState::None => 1,
                        BorrowState::Mutable => unreachable!(),
                    };
                    *entry = BorrowState::Immutable(count);
                }
            }
        }

        for scope in &mut self.scopes {
            for var in scope.values_mut() {
                var.borrow = BorrowState::None;
            }
        }

        for (name, borrow) in persistent {
            if let Some(var) = self.lookup_mut(&name) {
                var.borrow = borrow;
            }
        }
    }

    pub(super) fn lookup(&self, name: &str) -> Option<&VarState> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.get(name) {
                return Some(var);
            }
        }
        None
    }

    pub(super) fn lookup_mut(&mut self, name: &str) -> Option<&mut VarState> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(name) {
                return Some(var);
            }
        }
        None
    }

    pub(super) fn error(&mut self, msg: impl Into<String>) {
        let rendered = msg.into();
        if !self.errors.contains(&rendered) {
            self.errors.push(rendered);
        }
    }
}

pub(super) fn field_borrow_type(field: &crate::ast::Field) -> Type {
    if field.optional && !matches!(field.ty, Type::Optional(_)) {
        Type::Optional(Box::new(field.ty.clone()))
    } else {
        field.ty.clone()
    }
}

pub(super) fn is_reference_type(ty: &Type) -> bool {
    matches!(transparent_borrow_type(ty), Type::Ref(_) | Type::MutRef(_))
}

pub(super) fn transparent_borrow_type(ty: &Type) -> &Type {
    match ty {
        Type::Readonly(inner) => transparent_borrow_type(inner),
        other => other,
    }
}

pub(super) fn is_sync_array_method(method: &str) -> bool {
    matches!(
        method,
        "map"
            | "filter"
            | "partition"
            | "uniqueBy"
            | "groupBy"
            | "some"
            | "every"
            | "find"
            | "findLast"
            | "findIndex"
            | "findLastIndex"
            | "forEach"
            | "reduce"
            | "reduceRight"
            | "sort"
            | "toSorted"
            | "flat"
            | "flatMap"
    )
}

pub(super) fn switch_clause_falls_through(body: &[Stmt]) -> bool {
    for statement in body {
        let exits = match statement.kind() {
            StmtKind::Break(_) | StmtKind::Continue | StmtKind::Return(_) | StmtKind::Throw(_) => {
                true
            }
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        then_block,
                        else_block: Some(else_block),
                        ..
                    },
                ..
            }) => {
                !switch_clause_falls_through(then_block) && !switch_clause_falls_through(else_block)
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                finally_body
                    .as_ref()
                    .is_some_and(|body| !switch_clause_falls_through(body))
                    || (!switch_clause_falls_through(try_body)
                        && !switch_clause_falls_through(catch_body))
            }
            _ => false,
        };
        if exits {
            return false;
        }
    }
    true
}

pub(super) fn field_move_path(expr: &Expr) -> Option<(String, String)> {
    let mut segments = Vec::new();
    let mut current = expr;
    loop {
        match current.kind() {
            ExprKind::FieldAccess { object, field } => {
                segments.push(field.clone());
                current = object;
            }
            ExprKind::Ident(name) if !segments.is_empty() => {
                segments.reverse();
                return Some((name.clone(), segments.join(".")));
            }
            _ => return None,
        }
    }
}

pub(super) fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || left.starts_with(&format!("{right}."))
        || right.starts_with(&format!("{left}."))
}
