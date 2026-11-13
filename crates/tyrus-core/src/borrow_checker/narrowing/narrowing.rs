use super::*;

impl BorrowChecker {
    pub(super) fn null_narrowing(expr: &Expr) -> Option<(&str, bool)> {
        let Expr {
            kind: ExprKind::BinOp { op, left, right },
            ..
        } = expr
        else {
            return None;
        };

        let name = match (left.as_ref(), right.as_ref()) {
            (
                Expr {
                    kind: ExprKind::Ident(name),
                    ..
                },
                Expr {
                    kind: ExprKind::Null | ExprKind::None,
                    ..
                },
            )
            | (
                Expr {
                    kind: ExprKind::Null | ExprKind::None,
                    ..
                },
                Expr {
                    kind: ExprKind::Ident(name),
                    ..
                },
            ) => name.as_str(),
            _ => return None,
        };

        match op {
            BinOp::Neq => Some((name, true)),
            BinOp::Eq => Some((name, false)),
            _ => None,
        }
    }

    pub(super) fn apply_null_narrowing(
        &mut self,
        narrowing: Option<(&str, bool)>,
        branch_true: bool,
    ) {
        if let Some((name, non_null_when_true)) = narrowing
            && non_null_when_true == branch_true
            && let Some(var) = self.lookup_mut(name)
        {
            var.known_non_null = true;
        }
    }

    pub(super) fn typeof_narrowing(expr: &Expr) -> Option<(String, String, bool)> {
        let Expr {
            kind: ExprKind::BinOp { op, left, right },
            ..
        } = expr
        else {
            return None;
        };
        let (typeof_expr, type_name) = match (left.as_ref(), right.as_ref()) {
            (
                Expr {
                    kind: ExprKind::Typeof(inner),
                    ..
                },
                Expr {
                    kind: ExprKind::StrLit(name),
                    ..
                },
            ) => (inner, name.clone()),
            (
                Expr {
                    kind: ExprKind::StrLit(name),
                    ..
                },
                Expr {
                    kind: ExprKind::Typeof(inner),
                    ..
                },
            ) => (inner, name.clone()),
            _ => return None,
        };
        let var_name = match typeof_expr.as_ref().kind() {
            ExprKind::Ident(name) => name.clone(),
            _ => return None,
        };
        match op {
            BinOp::Eq => Some((var_name, type_name, true)),
            BinOp::Neq => Some((var_name, type_name, false)),
            _ => None,
        }
    }

    pub(super) fn apply_typeof_narrowing(
        &mut self,
        narrowing: Option<&(String, String, bool)>,
        branch_true: bool,
    ) {
        if let Some((name, _type_name, is_type_in_then)) = narrowing
            && *is_type_in_then == branch_true
            && let Some(var) = self.lookup_mut(name)
        {
            var.known_non_null = true;
        }
    }

    pub(super) fn merge_branch_scopes(base: &[Scope], branches: &[Vec<Scope>]) -> Vec<Scope> {
        let mut merged = base.to_vec();
        for (scope_index, scope) in merged.iter_mut().enumerate() {
            for (name, var) in scope {
                var.availability = if branches.iter().all(|branch| {
                    branch
                        .get(scope_index)
                        .and_then(|scope| scope.get(name))
                        .is_some_and(|branch_var| {
                            branch_var.availability == Availability::Available
                        })
                }) {
                    Availability::Available
                } else if branches.iter().all(|branch| {
                    branch
                        .get(scope_index)
                        .and_then(|scope| scope.get(name))
                        .is_some_and(|branch_var| branch_var.availability == Availability::Dropped)
                }) {
                    Availability::Dropped
                } else {
                    Availability::Moved
                };
                var.known_non_null = branches.iter().all(|branch| {
                    branch
                        .get(scope_index)
                        .and_then(|scope| scope.get(name))
                        .is_some_and(|branch_var| branch_var.known_non_null)
                });
                let mut moved_fields = Vec::new();
                for branch_var in branches
                    .iter()
                    .filter_map(|branch| branch.get(scope_index).and_then(|scope| scope.get(name)))
                {
                    for path in &branch_var.moved_fields {
                        if !moved_fields.contains(path) {
                            moved_fields.push(path.clone());
                        }
                    }
                }
                var.moved_fields = moved_fields;
            }
        }
        merged
    }

    pub(super) fn outer_move_state(&self) -> Vec<(usize, String, Availability, Vec<String>)> {
        self.scopes
            .iter()
            .enumerate()
            .flat_map(|(index, scope)| {
                scope.iter().map(move |(name, var)| {
                    (
                        index,
                        name.clone(),
                        var.availability,
                        var.moved_fields.clone(),
                    )
                })
            })
            .collect()
    }

    pub(super) fn report_moves_repeated_by_loop(
        &mut self,
        before: &[(usize, String, Availability, Vec<String>)],
        body: &[Stmt],
    ) {
        let leaves_every_pass = body.last().is_some_and(|stmt| {
            matches!(
                stmt.kind(),
                StmtKind::Return(_) | StmtKind::Throw(_) | StmtKind::Break(_)
            )
        });
        if leaves_every_pass {
            return;
        }
        let mut repeated = Vec::new();
        for (index, name, availability, moved_fields) in before {
            let Some(var) = self.scopes.get(*index).and_then(|scope| scope.get(name)) else {
                continue;
            };
            if *availability == Availability::Available && var.availability == Availability::Moved {
                repeated.push(format!(
                    "error: use of moved value: `{name}` (moved in the previous iteration of the loop)"
                ));
            }
            for path in &var.moved_fields {
                if !moved_fields.contains(path) {
                    repeated.push(format!(
                        "error: use of moved value: `{name}.{path}` (moved in the previous iteration of the loop)"
                    ));
                }
            }
        }
        for message in repeated {
            self.error(message);
        }
    }

    pub(super) fn check_binding_state(&mut self, name: &str) {
        match self.lookup(name) {
            None if !self.is_external(name) => {
                self.error(format!("error: use of undeclared variable `{name}`"));
            }
            Some(v) if v.availability == Availability::Moved => {
                self.error(format!("error: use of moved value: `{name}`"));
            }
            Some(v) if v.availability == Availability::Dropped => {
                self.error(format!("error: use of dropped value: `{name}`"));
            }
            Some(v) if v.borrow == BorrowState::Mutable => self.error(format!(
                "error: cannot use `{name}` while it is mutably borrowed"
            )),
            _ => {}
        }
    }

    pub(super) fn merge_reachable_branch_scopes(
        base: &[Scope],
        branches: &[(Vec<Scope>, bool)],
    ) -> Vec<Scope> {
        let reachable = branches
            .iter()
            .filter(|(_, reaches_next_statement)| *reaches_next_statement)
            .map(|(scopes, _)| scopes.clone())
            .collect::<Vec<_>>();
        if reachable.is_empty() {
            base.to_vec()
        } else {
            Self::merge_branch_scopes(base, &reachable)
        }
    }

    pub(super) fn block_reaches_next_statement(stmts: &[Stmt]) -> bool {
        stmts.iter().all(Self::stmt_reaches_next_statement)
    }

    pub(super) fn stmt_reaches_next_statement(stmt: &Stmt) -> bool {
        match stmt.kind() {
            StmtKind::Return(_) | StmtKind::Throw(_) => false,
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        then_block,
                        else_block: Some(else_block),
                        ..
                    },
                ..
            }) => {
                Self::block_reaches_next_statement(then_block)
                    || Self::block_reaches_next_statement(else_block)
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                finally_body
                    .as_ref()
                    .is_none_or(|body| Self::block_reaches_next_statement(body))
                    && (Self::block_reaches_next_statement(try_body)
                        || Self::block_reaches_next_statement(catch_body))
            }
            _ => true,
        }
    }
}
