use super::*;

impl BorrowChecker {
    fn declare_match_bindings(&mut self, pattern: &MatchPattern) {
        match pattern {
            MatchPattern::Binding(name) => {
                self.declare(name, false, Type::Infer);
            }
            MatchPattern::ResultVariant {
                binding: Some(name),
                ..
            } => {
                self.declare(name, false, Type::Infer);
            }
            MatchPattern::Object(fields) => {
                for field in fields {
                    if let Some(binding) = &field.binding {
                        self.declare(binding, false, Type::Infer);
                    }
                }
            }
            MatchPattern::EnumVariant {
                binding: Some(binding),
                ..
            } => self.declare_match_bindings(binding),
            MatchPattern::EnumVariant { binding: None, .. } => {}
            MatchPattern::Tuple(elements) => {
                for element in elements {
                    self.declare_match_bindings(element);
                }
            }
            MatchPattern::Array { elements, .. } => {
                for element in elements {
                    self.declare_match_bindings(&element.pattern);
                }
            }
            _ => {}
        }
    }

    pub(super) fn check_stmt(&mut self, stmt: &Stmt) {
        self.check_stmt_inner(stmt);
        self.refresh_persistent_borrows();
    }

    pub(super) fn check_stmt_sequence(&mut self, stmts: &[Stmt]) {
        let last_mentions = last_mentions(stmts);
        for (index, stmt) in stmts.iter().enumerate() {
            self.check_stmt(stmt);
            self.expire_dead_borrows(&last_mentions, index, stmt.span);
        }
    }

    fn expire_dead_borrows(
        &mut self,
        last_mentions: &HashMap<String, usize>,
        index: usize,
        stmt_span: Span,
    ) {
        if let Some(liveness) = &self.current_liveness
            && let Some(live_after) = liveness.get(&stmt_span)
        {
            let mut expired_or_revived = false;
            for scope in &mut self.scopes {
                for (name, var) in scope.iter_mut() {
                    if var.active_borrow.is_none() && !is_reference_type(&var.ty) {
                        continue;
                    }
                    let is_live = live_after.contains(name);
                    if !is_live && !var.dead {
                        var.dead = true;
                        expired_or_revived = true;
                    } else if is_live && var.dead {
                        var.dead = false;
                        expired_or_revived = true;
                    }
                }
            }
            if expired_or_revived {
                self.refresh_persistent_borrows();
            }
            return;
        }

        let scope_level = self.current_scope_level();
        let mut expired_any = false;
        if let Some(scope) = self.scopes.last_mut() {
            for (name, var) in scope.iter_mut() {
                if var.dead || var.scope_level != scope_level {
                    continue;
                }
                if var.active_borrow.is_none() && !is_reference_type(&var.ty) {
                    continue;
                }
                if last_mentions.get(name).is_some_and(|last| *last > index) {
                    continue;
                }
                var.dead = true;
                expired_any = true;
            }
        }
        if expired_any {
            self.refresh_persistent_borrows();
        }
    }

    #[allow(clippy::too_many_lines)]
    fn check_stmt_inner(&mut self, stmt: &Stmt) {
        match stmt.kind() {
            StmtKind::VarDecl {
                mutable,
                name,
                ty,
                value,
                ..
            } => {
                let Some(value) = value else {
                    self.declare_with_active_borrow(name, *mutable, ty.clone(), None, false);
                    return;
                };
                self.check_expr_read(value);
                self.validate_reference_assignment(self.current_scope_level(), value);
                let active_borrow = self.active_borrow_for_expr(value);
                let inferred_type = if let ExprKind::ArrowFn {
                    params, ret_type, ..
                } = value.kind()
                {
                    let contextual = match ty {
                        Type::Function {
                            params: expected_params,
                            ret: expected_return,
                        } => Some((expected_params.as_slice(), expected_return.as_ref())),
                        _ => None,
                    };
                    let actual = Type::Function {
                        params: params
                            .iter()
                            .enumerate()
                            .map(|(index, param)| {
                                if matches!(param.ty, Type::Infer) {
                                    contextual
                                        .and_then(|(params, _)| params.get(index))
                                        .cloned()
                                        .unwrap_or(Type::Infer)
                                } else {
                                    param.ty.clone()
                                }
                            })
                            .collect(),
                        ret: Box::new(if matches!(ret_type, Type::Infer) {
                            contextual
                                .map(|(_, ret)| ret.clone())
                                .unwrap_or(Type::Infer)
                        } else {
                            ret_type.clone()
                        }),
                    };
                    if !matches!(ty, Type::Infer) && ty != &actual {
                        self.error(format!(
                            "error: arrow function has type `{actual}`, but variable `{name}` is declared as `{ty}`"
                        ));
                    }
                    Some(actual)
                } else if matches!(ty, Type::Infer) {
                    match value {
                        Expr {
                            kind: ExprKind::Ident(src),
                            ..
                        } => self.lookup(src).map(|var| var.ty.clone()),
                        Expr {
                            kind:
                                ExprKind::FnCall {
                                    callee, type_args, ..
                                },
                            ..
                        } if callee == "Mutex" && type_args.len() == 1 => Some(Type::Apply {
                            name: "Mutex".to_string(),
                            args: type_args.clone(),
                        }),
                        Expr {
                            kind:
                                ExprKind::FnCall {
                                    callee, type_args, ..
                                },
                            ..
                        } if callee == "RwLock" && type_args.len() == 1 => Some(Type::Apply {
                            name: "RwLock".to_string(),
                            args: type_args.clone(),
                        }),
                        Expr {
                            kind: ExprKind::FnCall { callee, .. },
                            ..
                        } if callee == "Semaphore" => Some(Type::Named("Semaphore".to_string())),
                        Expr {
                            kind: ExprKind::Await(inner),
                            ..
                        } => self
                            .mutex_guard_return_type(inner)
                            .or_else(|| self.rwlock_guard_return_type(inner))
                            .or_else(|| self.semaphore_permit_return_type(inner)),
                        Expr {
                            kind: ExprKind::MethodCall { .. },
                            ..
                        } => self
                            .mutex_guard_return_type(value)
                            .or_else(|| self.rwlock_guard_return_type(value))
                            .or_else(|| self.semaphore_permit_return_type(value)),
                        Expr {
                            kind: ExprKind::FnCall { callee, .. },
                            ..
                        } => self
                            .lookup(callee)
                            .and_then(|var| match &var.ty {
                                Type::Function { ret, .. } => Some(ret.as_ref().clone()),
                                _ => None,
                            })
                            .filter(|ty| matches!(ty, Type::Function { .. })),
                        Expr {
                            kind: ExprKind::IntLit(_),
                            ..
                        } => Some(Type::I32),
                        Expr {
                            kind: ExprKind::UIntLit(_),
                            ..
                        } => Some(Type::Usize),
                        Expr {
                            kind: ExprKind::FloatLit(_),
                            ..
                        } => Some(Type::F64),
                        Expr {
                            kind: ExprKind::BoolLit(_),
                            ..
                        } => Some(Type::Bool),
                        _ => None,
                    }
                } else {
                    None
                };
                if let ExprKind::Ident(src) = value.kind() {
                    self.maybe_move(src, ty);
                }
                self.move_out_of_field(value, ty);
                let declare_ty = inferred_type.as_ref().unwrap_or(ty);
                self.declare_with_active_borrow(
                    name,
                    *mutable,
                    declare_ty.clone(),
                    active_borrow,
                    false,
                );
            }

            StmtKind::Assign { name, op: _, value } => {
                match self.lookup(name) {
                    None => self.error(format!(
                        "error: cannot assign to undeclared variable `{name}`"
                    )),
                    Some(v) if !v.mutable => {
                        self.error(format!("error: cannot assign to immutable variable `{name}` (declared with `const`)"));
                    }
                    Some(v) if v.borrow != BorrowState::None => {
                        self.error(format!(
                            "error: cannot assign to `{name}` while it is borrowed"
                        ));
                    }
                    _ => {}
                }
                self.check_expr_read(value);
                let target_scope_level = self
                    .lookup(name)
                    .map(|var| var.scope_level)
                    .unwrap_or_else(|| self.current_scope_level());
                self.validate_reference_assignment(target_scope_level, value);
                if let ExprKind::Ident(src) = value.kind() {
                    let target_ty = self
                        .lookup(name)
                        .map(|var| var.ty.clone())
                        .unwrap_or(Type::Infer);
                    self.maybe_move(src, &target_ty);
                }
                let active_borrow = self.active_borrow_for_expr(value);
                if let Some(var) = self.lookup_mut(name) {
                    var.known_non_null = false;
                    var.active_borrow = active_borrow;
                    var.dead = false;
                    var.moved_fields.clear();
                }
                self.refresh_persistent_borrows();
            }

            StmtKind::IndexAssign { name, index, value } => {
                match self.lookup(name) {
                    None => self.error(format!(
                        "error: cannot assign to undeclared variable `{name}`"
                    )),
                    Some(v) if !v.mutable => self.error(format!(
                        "error: cannot assign through immutable `{name}` (declared with `const`)"
                    )),
                    Some(v) if v.borrow != BorrowState::None => self.error(format!(
                        "error: cannot assign through `{name}` while it is borrowed"
                    )),
                    _ => {}
                }
                self.check_expr_read(index);
                self.check_expr_read(value);
            }

            StmtKind::FieldAssign {
                object,
                value,
                field,
                op: _,
            } => {
                if let Some(name) = crate::ast::place_root_ident(object) {
                    let is_mutex_guard_value = field == "value"
                        && self.lookup(name).is_some_and(|v| {
                            crate::type_checker::is_mutex_guard_typed(&v.ty)
                                || crate::type_checker::is_rwlock_write_guard_typed(&v.ty)
                        });
                    if !is_mutex_guard_value {
                        match self.lookup(name) {
                            None => self.error(format!(
                                "error: cannot assign to undeclared variable `{name}`"
                            )),
                            Some(v) if !v.mutable => self.error(format!(
                                "error: cannot assign through immutable `{name}` (declared with `const`)"
                            )),
                            Some(v) if v.borrow != BorrowState::None => self.error(format!(
                                "error: cannot assign through `{name}` while it is borrowed"
                            )),
                            _ => {}
                        }
                    }
                }
                self.check_expr_read(value);
                let assigned = Expr::synthetic(ExprKind::FieldAccess {
                    object: Box::new(object.clone()),
                    field: field.clone(),
                });
                match field_move_path(&assigned) {
                    Some((root, path)) if self.lookup(&root).is_some() => {
                        let moved_ancestor = self.lookup(&root).and_then(|var| {
                            var.moved_fields
                                .iter()
                                .find(|moved| path.starts_with(&format!("{moved}.")))
                                .cloned()
                        });
                        if let Some(ancestor) = moved_ancestor {
                            self.error(format!("error: use of moved value: `{root}.{ancestor}`"));
                        } else {
                            self.check_binding_state(&root);
                            if let Some(var) = self.lookup_mut(&root) {
                                var.moved_fields.retain(|moved| {
                                    moved != &path && !moved.starts_with(&format!("{path}."))
                                });
                            }
                        }
                    }
                    _ => self.check_expr_read(object),
                }
            }

            StmtKind::Return(expr) => {
                if let Some(e) = expr {
                    self.check_expr_read(e);
                    self.validate_return_references(e);
                    if let ExprKind::Ident(name) = e.kind() {
                        let is_result = self.lookup(name).is_some_and(|variable| {
                            matches!(variable.ty, Type::Result { .. })
                                || matches!(
                                    variable.ty,
                                    Type::Apply { ref name, .. } if name == "Result"
                                )
                        });
                        if is_result {
                            self.maybe_move(name, &Type::Infer);
                        }
                    }
                }
            }

            StmtKind::Break(None) | StmtKind::Continue => {}
            StmtKind::Break(Some(value)) => self.check_expr_read(value),

            StmtKind::While { cond, body } => {
                self.check_expr_read(cond);
                let before = self.outer_move_state();
                self.push_scope();
                self.check_stmt_sequence(body);
                self.pop_scope();
                self.report_moves_repeated_by_loop(&before, body);
            }

            StmtKind::DoWhile { body, cond } => {
                let before = self.outer_move_state();
                self.push_scope();
                self.check_stmt_sequence(body);
                self.pop_scope();
                self.report_moves_repeated_by_loop(&before, body);
                self.check_expr_read(cond);
            }

            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                self.push_scope();
                self.check_stmt(init);
                self.check_expr_read(cond);
                let before = self.outer_move_state();
                self.push_scope();
                self.check_stmt_sequence(body);
                self.pop_scope();
                self.check_stmt(update);
                self.report_moves_repeated_by_loop(&before, body);
                self.pop_scope();
            }
            StmtKind::ForOf {
                mutable,
                name,
                iterable,
                body,
            } => {
                self.check_expr_read(iterable);
                let before = self.outer_move_state();
                self.push_scope();
                self.declare(name, *mutable, Type::Infer);
                self.check_stmt_sequence(body);
                self.pop_scope();
                self.report_moves_repeated_by_loop(&before, body);
            }

            StmtKind::ForIn { name, object, body } => {
                self.check_expr_read(object);
                self.push_scope();
                self.declare(name, false, Type::Str);
                self.check_stmt_sequence(body);
                self.pop_scope();
            }

            StmtKind::ArrayDestructure {
                mutable,
                bindings,
                value,
            } => {
                self.check_expr_read(value);
                for binding in bindings {
                    self.declare(&binding.name, *mutable, binding.ty.clone());
                }
            }
            StmtKind::ObjectDestructure {
                mutable,
                bindings,
                value,
            } => {
                self.check_expr_read(value);
                for binding in bindings {
                    self.declare(&binding.name, *mutable, binding.ty.clone());
                }
            }
            StmtKind::Expr(expr) => {
                self.check_expr_read(expr);
            }
            StmtKind::Switch {
                expr,
                cases,
                default,
                default_index,
            } => {
                self.check_expr_read(expr);
                for case in cases {
                    self.check_expr_read(&case.value);
                }
                let scopes_before_branches = self.scopes.clone();
                let mut branch_scopes = Vec::new();
                let mut bodies = cases
                    .iter()
                    .map(|case| case.body.as_slice())
                    .collect::<Vec<_>>();
                if let Some(default_body) = default {
                    bodies.insert(
                        default_index.expect("switch default must have an index"),
                        default_body,
                    );
                }
                for entry in 0..bodies.len() {
                    self.scopes.clone_from(&scopes_before_branches);
                    let mut reaches_next_statement = true;
                    for body in &bodies[entry..] {
                        self.push_scope();
                        self.check_stmt_sequence(body);
                        self.pop_scope();
                        reaches_next_statement &= Self::block_reaches_next_statement(body);
                        if !switch_clause_falls_through(body) {
                            break;
                        }
                    }
                    branch_scopes.push((self.scopes.clone(), reaches_next_statement));
                }
                if default.is_none() {
                    branch_scopes.push((scopes_before_branches.clone(), true));
                }
                self.scopes =
                    Self::merge_reachable_branch_scopes(&scopes_before_branches, &branch_scopes);
            }
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                self.check_expr_read(expr);
                let scopes_before_branches = self.scopes.clone();
                let mut branch_scopes = Vec::new();
                for arm in arms {
                    self.scopes.clone_from(&scopes_before_branches);
                    self.push_scope();
                    self.declare_match_bindings(&arm.pattern);
                    if let Some(guard) = &arm.guard {
                        self.check_expr_read(guard);
                    }
                    self.check_stmt_sequence(&arm.body);
                    self.pop_scope();
                    branch_scopes.push((
                        self.scopes.clone(),
                        Self::block_reaches_next_statement(&arm.body),
                    ));
                }
                if let Some(default_body) = default {
                    self.scopes.clone_from(&scopes_before_branches);
                    self.push_scope();
                    self.check_stmt_sequence(default_body);
                    self.pop_scope();
                    branch_scopes.push((
                        self.scopes.clone(),
                        Self::block_reaches_next_statement(default_body),
                    ));
                } else {
                    branch_scopes.push((scopes_before_branches.clone(), true));
                }
                self.scopes =
                    Self::merge_reachable_branch_scopes(&scopes_before_branches, &branch_scopes);
                if let ExprKind::Ident(name) = expr.kind() {
                    let is_result = self.lookup(name).is_some_and(|variable| {
                        matches!(variable.ty, Type::Result { .. })
                            || matches!(
                                variable.ty,
                                Type::Apply { ref name, .. } if name == "Result"
                            )
                    });
                    if is_result {
                        self.maybe_move(name, &Type::Infer);
                    }
                }
            }
            StmtKind::Throw(expr) => {
                self.check_expr_read(expr);
            }
            StmtKind::TryCatch {
                try_body,
                catch_binding,
                catch_type,
                catch_type_span: _,
                catch_body,
                finally_body,
            } => {
                let scopes_before_branches = self.scopes.clone();

                self.push_scope();
                self.check_stmt_sequence(try_body);
                self.pop_scope();
                let try_scopes = self.scopes.clone();

                self.scopes.clone_from(&scopes_before_branches);
                self.push_scope();
                self.declare(catch_binding, false, catch_type.clone());
                self.check_stmt_sequence(catch_body);
                self.pop_scope();
                let catch_scopes = self.scopes.clone();

                let try_reaches_next_statement = Self::block_reaches_next_statement(try_body);
                let catch_reaches_next_statement = Self::block_reaches_next_statement(catch_body);
                self.scopes = Self::merge_reachable_branch_scopes(
                    &scopes_before_branches,
                    &[
                        (try_scopes, try_reaches_next_statement),
                        (catch_scopes, catch_reaches_next_statement),
                    ],
                );

                if let Some(finally_stmts) = finally_body {
                    self.push_scope();
                    self.check_stmt_sequence(finally_stmts);
                    self.pop_scope();
                }
            }
            StmtKind::Defer { body, .. } => {
                let scopes_before_finalizer = self.scopes.clone();
                let finalizer_scopes_before = self.finalizer_scopes.clone();
                self.push_scope();
                for statement in body {
                    self.check_stmt(statement);
                }
                self.scopes = scopes_before_finalizer;
                self.finalizer_scopes = finalizer_scopes_before;
                self.register_finalizer(body);
                self.refresh_persistent_borrows();
            }
        }
    }
}
