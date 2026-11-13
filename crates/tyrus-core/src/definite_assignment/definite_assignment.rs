use crate::ast::{Expr, ExprKind, Item, Program, Stmt, StmtKind, Type};
use crate::collections::HashSet;
use crate::type_checker::{CheckDiagnostic, CheckSeverity};

pub const USE_BEFORE_ASSIGN: &str = "E0381";

pub const NEVER_ASSIGNED: &str = "E0382";

fn is_optional(ty: &Type) -> bool {
    match ty {
        Type::Optional(_) => true,
        Type::Readonly(inner) => is_optional(inner),
        _ => false,
    }
}

fn is_empty_constructible_collection(ty: &Type) -> bool {
    match ty {
        Type::Map(_, _) | Type::Set(_) => true,
        Type::Readonly(inner) => is_empty_constructible_collection(inner),
        _ => false,
    }
}

pub fn check(program: &Program) -> Vec<CheckDiagnostic> {
    let mut out = Vec::new();
    let top_level: Vec<&Stmt> = program
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Stmt(statement) => Some(statement),
            _ => None,
        })
        .collect();
    analyze_body_and_nested(&top_level, &mut out);

    for item in &program.items {
        if let Item::FnDecl { body, .. } = item {
            analyze_body_and_nested(&body.iter().collect::<Vec<_>>(), &mut out);
        }
    }
    out
}

fn analyze_body_and_nested(body: &[&Stmt], out: &mut Vec<CheckDiagnostic>) {
    Analyzer::new().run(body, out);
    for statement in body {
        for_each_closure_body(statement, &mut |closure_body| {
            analyze_body_and_nested(&closure_body.iter().collect::<Vec<_>>(), out);
        });
    }
}

struct Analyzer {
    deferred: Vec<String>,
    deferred_set: HashSet<String>,
    excluded: HashSet<String>,
    ever_assigned: HashSet<String>,
    reported: HashSet<String>,
}

impl Analyzer {
    fn new() -> Self {
        Self {
            deferred: Vec::new(),
            deferred_set: HashSet::default(),
            excluded: HashSet::default(),
            ever_assigned: HashSet::default(),
            reported: HashSet::default(),
        }
    }

    fn run(&mut self, body: &[&Stmt], out: &mut Vec<CheckDiagnostic>) {
        for statement in body {
            for_each_closure_body(statement, &mut |closure_body| {
                collect_assigned_names(closure_body, &mut self.excluded);
            });
        }
        let mut assigned = HashSet::default();
        self.block(body, &mut assigned, out);

        for name in &self.deferred {
            if !self.ever_assigned.contains(name)
                && !self.excluded.contains(name)
                && !self.reported.contains(name)
            {
                out.push(CheckDiagnostic {
                    code: NEVER_ASSIGNED,
                    severity: CheckSeverity::Warning,
                    message: format!("`{name}` is declared but never assigned a value"),
                    subject: Some(name.clone()),
                    help: Vec::new(),
                    prefer_declaration: true,
                    span: None,
                });
            }
        }
    }

    fn block(
        &mut self,
        stmts: &[&Stmt],
        assigned: &mut HashSet<String>,
        out: &mut Vec<CheckDiagnostic>,
    ) -> bool {
        for statement in stmts {
            if self.stmt(statement, assigned, out) {
                return true;
            }
        }
        false
    }

    fn stmt(
        &mut self,
        statement: &Stmt,
        assigned: &mut HashSet<String>,
        out: &mut Vec<CheckDiagnostic>,
    ) -> bool {
        match statement.kind() {
            StmtKind::VarDecl {
                name, ty, value, ..
            } => {
                match value {
                    Some(value) => {
                        self.reads(value, assigned, out);
                        assigned.insert(name.clone());
                        self.ever_assigned.insert(name.clone());
                    }
                    None if is_optional(ty) || is_empty_constructible_collection(ty) => {
                        assigned.insert(name.clone());
                        self.ever_assigned.insert(name.clone());
                    }
                    None => {
                        if self.deferred_set.insert(name.clone()) {
                            self.deferred.push(name.clone());
                        }
                    }
                }
                false
            }
            StmtKind::Assign { name, op, value } => {
                self.reads(value, assigned, out);
                if !matches!(op, crate::ast::AssignOp::Assign) {
                    self.read_name(name, assigned, out);
                }
                assigned.insert(name.clone());
                self.ever_assigned.insert(name.clone());
                false
            }
            StmtKind::IndexAssign { name, index, value } => {
                self.read_name(name, assigned, out);
                self.reads(index, assigned, out);
                self.reads(value, assigned, out);
                false
            }
            StmtKind::FieldAssign { object, value, .. } => {
                self.reads(object, assigned, out);
                self.reads(value, assigned, out);
                false
            }
            StmtKind::ArrayDestructure { value, .. }
            | StmtKind::ObjectDestructure { value, .. } => {
                self.reads(value, assigned, out);
                false
            }
            StmtKind::Return(Some(value)) | StmtKind::Throw(value) => {
                self.reads(value, assigned, out);
                true
            }
            StmtKind::Return(None) | StmtKind::Continue => true,
            StmtKind::Break(value) => {
                if let Some(value) = value {
                    self.reads(value, assigned, out);
                }
                true
            }
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        cond,
                        then_block,
                        else_block,
                    },
                ..
            }) => self.branch_if(cond, then_block, else_block.as_deref(), assigned, out),
            StmtKind::Expr(Expr {
                kind: ExprKind::Loop { body },
                ..
            }) => {
                let mut body_state = assigned.clone();
                let diverged = self.block(&refs(body), &mut body_state, out);
                diverged && !loop_has_break(body)
            }
            StmtKind::Expr(value) => {
                self.reads(value, assigned, out);
                false
            }
            StmtKind::While { cond, body } => {
                self.reads(cond, assigned, out);
                let mut body_state = assigned.clone();
                self.block(&refs(body), &mut body_state, out);
                false
            }
            StmtKind::DoWhile { body, cond } => {
                let diverged = self.block(&refs(body), assigned, out);
                self.reads(cond, assigned, out);
                diverged
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                self.stmt(init, assigned, out);
                self.reads(cond, assigned, out);
                let mut body_state = assigned.clone();
                self.stmt(update, &mut body_state, out);
                self.block(&refs(body), &mut body_state, out);
                false
            }
            StmtKind::ForOf {
                name,
                iterable,
                body,
                ..
            } => {
                self.reads(iterable, assigned, out);
                let mut body_state = assigned.clone();
                body_state.insert(name.clone());
                self.block(&refs(body), &mut body_state, out);
                false
            }
            StmtKind::ForIn { name, object, body } => {
                self.reads(object, assigned, out);
                let mut body_state = assigned.clone();
                body_state.insert(name.clone());
                self.block(&refs(body), &mut body_state, out);
                false
            }
            StmtKind::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                self.reads(expr, assigned, out);
                let mut paths: Vec<(HashSet<String>, bool)> = Vec::new();
                for case in cases {
                    let mut state = assigned.clone();
                    let diverged = self.block(&refs(&case.body), &mut state, out);
                    if diverged && switch_clause_exit(&case.body) == Some(true) {
                        paths.push((state, false));
                    } else {
                        paths.push((state, diverged));
                    }
                }
                match default {
                    Some(default_body) => {
                        let mut state = assigned.clone();
                        let diverged = self.block(&refs(default_body), &mut state, out);
                        if diverged && switch_clause_exit(default_body) == Some(true) {
                            paths.push((state, false));
                        } else {
                            paths.push((state, diverged));
                        }
                    }
                    None => paths.push((assigned.clone(), false)),
                }
                let (merged, diverged) = merge_paths(paths, assigned);
                *assigned = merged;
                diverged
            }
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                self.reads(expr, assigned, out);
                let mut paths: Vec<(HashSet<String>, bool)> = Vec::new();
                for arm in arms {
                    let mut state = assigned.clone();
                    if let Some(guard) = &arm.guard {
                        self.reads(guard, &state, out);
                    }
                    let diverged = self.block(&refs(&arm.body), &mut state, out);
                    paths.push((state, diverged));
                }
                match default {
                    Some(default_body) => {
                        let mut state = assigned.clone();
                        let diverged = self.block(&refs(default_body), &mut state, out);
                        paths.push((state, diverged));
                    }
                    None => paths.push((assigned.clone(), false)),
                }
                let (merged, diverged) = merge_paths(paths, assigned);
                *assigned = merged;
                diverged
            }
            StmtKind::TryCatch {
                try_body,
                catch_binding,
                catch_body,
                finally_body,
                ..
            } => {
                let mut try_state = assigned.clone();
                let try_diverged = self.block(&refs(try_body), &mut try_state, out);

                let mut catch_state = assigned.clone();
                catch_state.insert(catch_binding.clone());
                let catch_diverged = self.block(&refs(catch_body), &mut catch_state, out);

                let (merged, merged_diverged) = merge_paths(
                    vec![(try_state, try_diverged), (catch_state, catch_diverged)],
                    assigned,
                );
                *assigned = merged;

                if let Some(finally_body) = finally_body {
                    let finally_diverged = self.block(&refs(finally_body), assigned, out);
                    finally_diverged || merged_diverged
                } else {
                    merged_diverged
                }
            }
            StmtKind::Defer { body, .. } => {
                let mut finalizer_state = assigned.clone();
                self.block(&refs(body), &mut finalizer_state, out);
                false
            }
        }
    }

    fn branch_if(
        &mut self,
        cond: &Expr,
        then_block: &[Stmt],
        else_block: Option<&[Stmt]>,
        assigned: &mut HashSet<String>,
        out: &mut Vec<CheckDiagnostic>,
    ) -> bool {
        self.reads(cond, assigned, out);
        let mut then_state = assigned.clone();
        let then_diverged = self.block(&refs(then_block), &mut then_state, out);

        let else_path = match else_block {
            Some(else_block) => {
                let mut else_state = assigned.clone();
                let diverged = self.block(&refs(else_block), &mut else_state, out);
                (else_state, diverged)
            }
            None => (assigned.clone(), false),
        };

        let (merged, diverged) =
            merge_paths(vec![(then_state, then_diverged), else_path], assigned);
        *assigned = merged;
        diverged
    }

    fn read_name(
        &mut self,
        name: &str,
        assigned: &HashSet<String>,
        out: &mut Vec<CheckDiagnostic>,
    ) {
        if self.deferred_set.contains(name)
            && !assigned.contains(name)
            && !self.excluded.contains(name)
            && self.reported.insert(name.to_string())
        {
            out.push(CheckDiagnostic {
                code: USE_BEFORE_ASSIGN,
                severity: CheckSeverity::Warning,
                message: format!("`{name}` is used before being assigned a value"),
                subject: Some(name.to_string()),
                help: Vec::new(),
                prefer_declaration: true,
                span: None,
            });
        }
    }

    fn reads(&mut self, expr: &Expr, assigned: &HashSet<String>, out: &mut Vec<CheckDiagnostic>) {
        match expr.kind() {
            ExprKind::Ident(name) => self.read_name(name, assigned, out),
            ExprKind::Borrow { name, .. } => self.read_name(name, assigned, out),
            ExprKind::ArrowFn { .. } | ExprKind::If { .. } | ExprKind::Loop { .. } => {}
            ExprKind::Not(inner)
            | ExprKind::Neg(inner)
            | ExprKind::BitNot(inner)
            | ExprKind::Unwrap(inner)
            | ExprKind::Typeof(inner)
            | ExprKind::Await(inner)
            | ExprKind::Try(inner)
            | ExprKind::Spread(inner)
            | ExprKind::ConstAssert(inner)
            | ExprKind::Comptime(inner)
            | ExprKind::PrefixIncr(inner)
            | ExprKind::PrefixDecr(inner)
            | ExprKind::PostfixIncr(inner)
            | ExprKind::PostfixDecr(inner) => self.reads(inner, assigned, out),
            ExprKind::BinOp { left, right, .. } => {
                self.reads(left, assigned, out);
                self.reads(right, assigned, out);
            }
            ExprKind::ChainedComparison { terms, .. } => {
                for term in terms {
                    self.reads(term, assigned, out);
                }
            }
            ExprKind::Pipeline { left, right } => {
                self.reads(left, assigned, out);
                self.reads(right, assigned, out);
            }
            ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
                self.reads(object, assigned, out);
            }
            ExprKind::IndexAccess { object, index } => {
                self.reads(object, assigned, out);
                self.reads(index, assigned, out);
            }
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.reads(cond, assigned, out);
                self.reads(then_expr, assigned, out);
                self.reads(else_expr, assigned, out);
            }
            ExprKind::TypeAssert { value, .. } => self.reads(value, assigned, out),
            ExprKind::Is { expr, .. } => self.reads(expr, assigned, out),
            ExprKind::ArrayLit(values) => values
                .iter()
                .for_each(|value| self.reads(value, assigned, out)),
            ExprKind::ArrayRepeat { value, count } => {
                self.reads(value, assigned, out);
                self.reads(count, assigned, out);
            }
            ExprKind::ObjectLit(fields) => fields
                .iter()
                .for_each(|(_, value)| self.reads(value, assigned, out)),
            ExprKind::ObjectUpdate { base, fields } => {
                self.reads(base, assigned, out);
                fields
                    .iter()
                    .for_each(|(_, value)| self.reads(value, assigned, out));
            }
            ExprKind::FnCall { args, .. } => {
                args.iter().for_each(|arg| self.reads(arg, assigned, out));
            }
            ExprKind::CallValue { callee, args, .. } => {
                self.reads(callee, assigned, out);
                args.iter().for_each(|arg| self.reads(arg, assigned, out));
            }
            ExprKind::MethodCall { object, args, .. }
            | ExprKind::OptionalMethodCall { object, args, .. } => {
                self.reads(object, assigned, out);
                args.iter().for_each(|arg| self.reads(arg, assigned, out));
            }
            ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
                if let Some(initial) = initial {
                    self.reads(initial, assigned, out);
                }
            }
            ExprKind::In { key, object } => {
                self.reads(key, assigned, out);
                self.reads(object, assigned, out);
            }
            ExprKind::Range { start, end, .. } => {
                self.reads(start, assigned, out);
                self.reads(end, assigned, out);
            }
            ExprKind::TemplateLit(parts) => {
                for part in parts {
                    if let crate::ast::TemplatePart::Expr(expr, _) = part {
                        self.reads(expr, assigned, out);
                    }
                }
            }
            ExprKind::StrLit(_)
            | ExprKind::RegexLit { .. }
            | ExprKind::IntLit(_)
            | ExprKind::UIntLit(_)
            | ExprKind::FloatLit(_)
            | ExprKind::DecimalLit(_)
            | ExprKind::BoolLit(_)
            | ExprKind::Null
            | ExprKind::None
            | ExprKind::Topic => {}
        }
    }
}

fn merge_paths(
    paths: Vec<(HashSet<String>, bool)>,
    base: &HashSet<String>,
) -> (HashSet<String>, bool) {
    let mut surviving = paths.into_iter().filter(|(_, diverged)| !diverged);
    let Some((first, _)) = surviving.next() else {
        return (base.clone(), true);
    };
    let merged = surviving.fold(first, |acc, (set, _)| {
        acc.intersection(&set).cloned().collect()
    });
    (merged, false)
}

fn refs(body: &[Stmt]) -> Vec<&Stmt> {
    body.iter().collect()
}

fn switch_clause_exit(body: &[Stmt]) -> Option<bool> {
    for statement in body {
        let outcome = match statement.kind() {
            StmtKind::Break(_) => Some(true),
            StmtKind::Continue | StmtKind::Return(_) | StmtKind::Throw(_) => Some(false),
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        then_block,
                        else_block: Some(else_block),
                        ..
                    },
                ..
            }) => match (
                switch_clause_exit(then_block),
                switch_clause_exit(else_block),
            ) {
                (Some(then_break), Some(else_break)) => Some(then_break || else_break),
                _ => None,
            },
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                if let Some(finally_exit) = finally_body
                    .as_ref()
                    .and_then(|body| switch_clause_exit(body))
                {
                    Some(finally_exit)
                } else {
                    match (switch_clause_exit(try_body), switch_clause_exit(catch_body)) {
                        (Some(try_break), Some(catch_break)) => Some(try_break || catch_break),
                        _ => None,
                    }
                }
            }
            StmtKind::Defer { body, .. } => Some(loop_has_break(body)),
            _ => None,
        };
        if outcome.is_some() {
            return outcome;
        }
    }
    None
}

fn loop_has_break(body: &[Stmt]) -> bool {
    for statement in body {
        match statement.kind() {
            StmtKind::Break(_) => return true,
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        then_block,
                        else_block,
                        ..
                    },
                ..
            }) => {
                if loop_has_break(then_block)
                    || else_block
                        .as_ref()
                        .is_some_and(|block| loop_has_break(block))
                {
                    return true;
                }
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                if loop_has_break(try_body)
                    || loop_has_break(catch_body)
                    || finally_body
                        .as_ref()
                        .is_some_and(|block| loop_has_break(block))
                {
                    return true;
                }
            }
            StmtKind::While { .. }
            | StmtKind::DoWhile { .. }
            | StmtKind::For { .. }
            | StmtKind::ForOf { .. }
            | StmtKind::ForIn { .. }
            | StmtKind::Switch { .. }
            | StmtKind::Match { .. }
            | StmtKind::Expr(_)
            | StmtKind::Continue
            | StmtKind::Return(_)
            | StmtKind::Throw(_)
            | StmtKind::VarDecl { .. }
            | StmtKind::ArrayDestructure { .. }
            | StmtKind::ObjectDestructure { .. }
            | StmtKind::Assign { .. }
            | StmtKind::IndexAssign { .. }
            | StmtKind::FieldAssign { .. } => {}
            StmtKind::Defer { .. } => {}
        }
    }
    false
}

fn collect_assigned_names(body: &[Stmt], names: &mut HashSet<String>) {
    for statement in body {
        match statement.kind() {
            StmtKind::Assign { name, .. } | StmtKind::IndexAssign { name, .. } => {
                names.insert(name.clone());
            }
            StmtKind::VarDecl { name, .. } => {
                names.insert(name.clone());
            }
            _ => {}
        }
        for_each_child_block(statement, &mut |child| collect_assigned_names(child, names));
    }
}

fn for_each_closure_body(statement: &Stmt, visit: &mut dyn FnMut(&[Stmt])) {
    fn in_expr(expr: &Expr, visit: &mut dyn FnMut(&[Stmt])) {
        match expr.kind() {
            ExprKind::ArrowFn { body, .. } => visit(body),
            ExprKind::Not(inner)
            | ExprKind::Neg(inner)
            | ExprKind::BitNot(inner)
            | ExprKind::Unwrap(inner)
            | ExprKind::Typeof(inner)
            | ExprKind::Spread(inner)
            | ExprKind::ConstAssert(inner)
            | ExprKind::Comptime(inner)
            | ExprKind::PrefixIncr(inner)
            | ExprKind::PrefixDecr(inner)
            | ExprKind::PostfixIncr(inner)
            | ExprKind::PostfixDecr(inner)
            | ExprKind::FieldAccess { object: inner, .. }
            | ExprKind::OptionalFieldAccess { object: inner, .. }
            | ExprKind::TypeAssert { value: inner, .. } => in_expr(inner, visit),
            ExprKind::BinOp { left, right, .. } => {
                in_expr(left, visit);
                in_expr(right, visit);
            }
            ExprKind::IndexAccess { object, index } => {
                in_expr(object, visit);
                in_expr(index, visit);
            }
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                in_expr(cond, visit);
                in_expr(then_expr, visit);
                in_expr(else_expr, visit);
            }
            ExprKind::ArrayLit(values) => values.iter().for_each(|value| in_expr(value, visit)),
            ExprKind::ArrayRepeat { value, count } => {
                in_expr(value, visit);
                in_expr(count, visit);
            }
            ExprKind::ObjectLit(fields) | ExprKind::ObjectUpdate { fields, .. } => {
                fields.iter().for_each(|(_, value)| in_expr(value, visit));
            }
            ExprKind::FnCall { args, .. } => args.iter().for_each(|arg| in_expr(arg, visit)),
            ExprKind::MethodCall { object, args, .. }
            | ExprKind::OptionalMethodCall { object, args, .. } => {
                in_expr(object, visit);
                args.iter().for_each(|arg| in_expr(arg, visit));
            }
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                in_expr(cond, visit);
                then_block
                    .iter()
                    .for_each(|stmt| for_each_closure_body(stmt, visit));
                if let Some(else_block) = else_block {
                    else_block
                        .iter()
                        .for_each(|stmt| for_each_closure_body(stmt, visit));
                }
            }
            ExprKind::Loop { body } => {
                body.iter()
                    .for_each(|stmt| for_each_closure_body(stmt, visit));
            }
            ExprKind::In { key, object } => {
                in_expr(key, visit);
                in_expr(object, visit);
            }
            ExprKind::TemplateLit(parts) => {
                for part in parts {
                    if let crate::ast::TemplatePart::Expr(expr, _) = part {
                        in_expr(expr, visit);
                    }
                }
            }
            _ => {}
        }
    }

    for_each_expr(statement, &mut |expr| in_expr(expr, visit));
    for_each_child_block(statement, &mut |child| {
        child
            .iter()
            .for_each(|stmt| for_each_closure_body(stmt, visit));
    });
}

fn for_each_expr(statement: &Stmt, visit: &mut dyn FnMut(&Expr)) {
    match statement.kind() {
        StmtKind::VarDecl { value: Some(v), .. }
        | StmtKind::Assign { value: v, .. }
        | StmtKind::Return(Some(v))
        | StmtKind::Throw(v)
        | StmtKind::Expr(v)
        | StmtKind::Break(Some(v))
        | StmtKind::ArrayDestructure { value: v, .. }
        | StmtKind::ObjectDestructure { value: v, .. } => visit(v),
        StmtKind::IndexAssign { index, value, .. } => {
            visit(index);
            visit(value);
        }
        StmtKind::While { cond, .. } | StmtKind::DoWhile { cond, .. } => visit(cond),
        StmtKind::For { cond, .. } => visit(cond),
        StmtKind::ForOf { iterable, .. } => visit(iterable),
        StmtKind::ForIn { object, .. } => visit(object),
        StmtKind::Switch { expr, .. } | StmtKind::Match { expr, .. } => visit(expr),
        _ => {}
    }
}

fn for_each_child_block(statement: &Stmt, visit: &mut dyn FnMut(&[Stmt])) {
    match statement.kind() {
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::ForIn { body, .. } => visit(body),
        StmtKind::Expr(Expr {
            kind: ExprKind::Loop { body },
            ..
        }) => visit(body),
        StmtKind::For {
            init, update, body, ..
        } => {
            for_each_child_block(init, visit);
            for_each_child_block(update, visit);
            visit(body);
        }
        StmtKind::Switch { cases, default, .. } => {
            cases.iter().for_each(|case| visit(&case.body));
            if let Some(default) = default {
                visit(default);
            }
        }
        StmtKind::Match { arms, default, .. } => {
            arms.iter().for_each(|arm| visit(&arm.body));
            if let Some(default) = default {
                visit(default);
            }
        }
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            visit(try_body);
            visit(catch_body);
            if let Some(finally_body) = finally_body {
                visit(finally_body);
            }
        }
        _ => {}
    }
}
