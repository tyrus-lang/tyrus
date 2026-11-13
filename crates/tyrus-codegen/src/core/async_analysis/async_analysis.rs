use super::*;

#[path = "await_sites/await_sites.rs"]
mod await_sites;

impl Codegen<'_> {
    pub(super) fn async_expression_await_sites(
        &mut self,
        statement: &Stmt,
        outer_index: usize,
    ) -> Vec<AsyncAwaitSite> {
        let mut continuation_statement = statement.clone();
        {
            let Some(expression) =
                simple_async_statement_expression_mut(&mut continuation_statement)
            else {
                return Vec::new();
            };
            let mut expression_id = 0usize;
            assign_async_expression_ids(expression, outer_index, &mut expression_id);
        }
        let continuation_template = continuation_statement.clone();
        let expression = simple_async_statement_expression(&continuation_statement).unwrap();
        let mut awaited = Vec::new();
        collect_await_expressions(expression, &mut awaited);
        let awaited_count = awaited.len();
        awaited
            .into_iter()
            .enumerate()
            .filter_map(|(index, awaited)| {
                let ExprKind::Await(dependency) = awaited.kind() else {
                    return None;
                };
                let promise_type = self.expr_type_hint(dependency)?;
                let payload = promise_payload_type(transparent_codegen_type(&promise_type))?;
                let payload = self.resolve_type(payload);
                let mut prefix_expressions = Vec::new();
                collect_prefixes_before_await(expression, awaited.span, &mut prefix_expressions);
                let prefixes = prefix_expressions
                    .into_iter()
                    .filter(|prefix| async_prefix_requires_storage(prefix))
                    .map(|prefix| {
                        let ty = self
                            .expr_type_hint(prefix)
                            .map(|ty| self.resolve_type(&ty))?;
                        ty.is_copy().then(|| (prefix.clone(), ty))
                    })
                    .collect::<Option<Vec<_>>>()?;
                let guard = (awaited_count == 1)
                    .then(|| conditional_guard_for_await(expression, awaited.span))
                    .flatten();
                (payload.is_copy() || self.async_owned_value_supported(&payload)).then(|| {
                    AsyncAwaitSite {
                        outer_index,
                        dependency: dependency.as_ref().clone(),
                        kind: AsyncAwaitKind::Expression {
                            statement: continuation_template.clone(),
                            span: awaited.span,
                            ty: payload,
                            prefixes,
                            guard,
                            final_in_statement: index + 1 == awaited_count,
                        },
                    }
                })
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn async_if_branch_await_site(
        &self,
        _statement: &Stmt,
        outer_index: usize,
        cond: &Expr,
        then_block: &[Stmt],
        else_block: Option<&[Stmt]>,
        try_context: Option<AsyncTryContext>,
        control_context: Option<AsyncControlContext>,
    ) -> Vec<AsyncAwaitSite> {
        if first_await_expression(cond).is_some() {
            return Vec::new();
        }
        let Some(then_awaits) = direct_awaits_in_block(then_block) else {
            return Vec::new();
        };
        let else_body = else_block.unwrap_or_default();
        let Some(else_awaits) = direct_awaits_in_block(else_body) else {
            return Vec::new();
        };
        if then_awaits.is_empty() == else_awaits.is_empty() {
            return Vec::new();
        }
        let (await_in_then, sites, selected, other) = if then_awaits.is_empty() {
            (false, else_awaits, else_body, then_block)
        } else {
            (true, then_awaits, then_block, else_body)
        };
        sites
            .iter()
            .enumerate()
            .map(|(site_index, (await_index, dependency, action))| {
                let start = site_index
                    .checked_sub(1)
                    .map_or(0, |previous| sites[previous].0 + 1);
                let final_in_branch = site_index + 1 == sites.len();
                AsyncAwaitSite {
                    outer_index,
                    dependency: dependency.clone(),
                    kind: AsyncAwaitKind::IfBranch {
                        cond: cond.clone(),
                        before: selected[start..*await_index].to_vec(),
                        after: final_in_branch
                            .then(|| selected[await_index + 1..].to_vec())
                            .unwrap_or_default(),
                        other: other.to_vec(),
                        await_in_then,
                        action: action.clone(),
                        first_in_branch: site_index == 0,
                        final_in_branch,
                        try_context: try_context.clone(),
                        control_context: control_context.clone(),
                    },
                }
            })
            .collect()
    }

    pub(super) fn async_if_in_control_body_await_sites(
        &self,
        outer_index: usize,
        body: &[Stmt],
        context: AsyncControlContext,
    ) -> Vec<AsyncAwaitSite> {
        let mut selected = None;
        for (index, statement) in body.iter().enumerate() {
            if first_await_in_block(std::slice::from_ref(statement)).is_none() {
                continue;
            }
            if selected.is_some() {
                return Vec::new();
            }
            let StmtKind::Expr(expression) = statement.kind() else {
                return Vec::new();
            };
            let ExprKind::If {
                cond,
                then_block,
                else_block,
            } = expression.kind()
            else {
                return Vec::new();
            };
            if first_await_expression(cond).is_some() {
                return Vec::new();
            }
            selected = Some((index, statement, cond, then_block, else_block.as_deref()));
        }
        let Some((index, statement, cond, then_block, else_block)) = selected else {
            return Vec::new();
        };
        self.async_if_branch_await_site(
            statement,
            outer_index,
            cond,
            then_block,
            else_block,
            None,
            Some(context.with_surrounding(body[..index].to_vec(), body[index + 1..].to_vec())),
        )
    }

    pub(super) fn async_recursive_control_await_sites(
        &mut self,
        outer_index: usize,
        statement: &Stmt,
    ) -> Vec<AsyncAwaitSite> {
        let mut contexts = Vec::new();
        self.async_descend_control_statement(outer_index, statement, &mut contexts)
            .unwrap_or_default()
    }

    pub(super) fn async_switch_await_sites(
        &mut self,
        outer_index: usize,
        expr: &Expr,
        cases: &[SwitchCase],
        default: &Option<Vec<Stmt>>,
        default_index: Option<usize>,
        context: Option<AsyncSwitchContext>,
    ) -> Vec<AsyncAwaitSite> {
        let switch_id = expr.span;
        let subject_type = self
            .expr_type_hint(expr)
            .map(|subject_type| self.resolve_type(&subject_type));
        let subject = match expr.kind() {
            ExprKind::Await(dependency) if first_await_expression(dependency).is_none() => {
                subject_type
                    .clone()
                    .map(|subject_type| (dependency.as_ref().clone(), subject_type))
            }
            _ => None,
        };
        let mut continuation_cases = cases.to_vec();
        let mut case_awaits = Vec::new();
        for (case_index, case) in continuation_cases.iter_mut().enumerate() {
            let mut expression_id = 0usize;
            assign_async_expression_ids(
                &mut case.value,
                outer_index.wrapping_mul(1024).wrapping_add(case_index),
                &mut expression_id,
            );
            let expression = case.value.clone();
            let mut awaited = Vec::new();
            collect_await_expressions(&expression, &mut awaited);
            let awaited_count = awaited.len();
            for (await_index, awaited) in awaited.into_iter().enumerate() {
                let ExprKind::Await(dependency) = awaited.kind() else {
                    continue;
                };
                let Some(promise_type) = self.expr_type_hint(dependency) else {
                    continue;
                };
                let Some(payload) =
                    promise_payload_type(transparent_codegen_type(&promise_type)).cloned()
                else {
                    continue;
                };
                let payload = self.resolve_type(&payload);
                if !payload.is_copy() && !self.async_owned_value_supported(&payload) {
                    continue;
                }
                let mut prefix_expressions = Vec::new();
                collect_prefixes_before_await(&expression, awaited.span, &mut prefix_expressions);
                let Some(prefixes) = prefix_expressions
                    .into_iter()
                    .filter(|prefix| async_prefix_requires_storage(prefix))
                    .map(|prefix| {
                        let ty = self
                            .expr_type_hint(prefix)
                            .map(|ty| self.resolve_type(&ty))?;
                        ty.is_copy().then(|| (prefix.clone(), ty))
                    })
                    .collect::<Option<Vec<_>>>()
                else {
                    continue;
                };
                let guard = (awaited_count == 1)
                    .then(|| conditional_guard_for_await(&expression, awaited.span))
                    .flatten();
                case_awaits.push((
                    case_index,
                    dependency.as_ref().clone(),
                    awaited.span,
                    payload,
                    prefixes,
                    guard,
                    await_index + 1 == awaited_count,
                ));
            }
        }
        let subject_storage = (!case_awaits.is_empty()).then(|| {
            format!(
                "__tyrus_async_switch_subject_{outer_index}_{}_{}",
                switch_id.start, switch_id.end
            )
        });
        let clause_sites =
            direct_awaits_in_switch(cases, default, default_index).unwrap_or_default();
        let clause_site_count = clause_sites.len();
        let mut sites = Vec::with_capacity(
            clause_site_count + case_awaits.len() + usize::from(subject.is_some()),
        );
        if let Some((dependency, subject_type)) = subject {
            sites.push(AsyncAwaitSite {
                outer_index,
                dependency,
                kind: AsyncAwaitKind::SwitchSubject {
                    switch_id,
                    cases: continuation_cases.clone(),
                    default: default.clone(),
                    default_index,
                    subject_type,
                    subject_storage: subject_storage.clone(),
                    context: context.clone(),
                },
            });
        }
        let subject_suspends = !sites.is_empty();
        if let (Some(subject_type), Some(subject_storage)) =
            (subject_type.clone(), subject_storage.clone())
        {
            sites.extend(case_awaits.into_iter().enumerate().map(
                |(
                    await_index,
                    (
                        case_index,
                        dependency,
                        span,
                        payload_type,
                        prefixes,
                        guard,
                        final_in_case_expression,
                    ),
                )| AsyncAwaitSite {
                    outer_index,
                    dependency,
                    kind: AsyncAwaitKind::SwitchCaseExpression {
                        switch_id,
                        expr: expr.clone(),
                        cases: continuation_cases.clone(),
                        default: default.clone(),
                        default_index,
                        subject_type: subject_type.clone(),
                        subject_storage: subject_storage.clone(),
                        case_index,
                        span,
                        payload_type,
                        prefixes,
                        guard,
                        final_in_case_expression,
                        first_in_switch: !subject_suspends && await_index == 0,
                        context: context.clone(),
                    },
                },
            ));
        }
        let dispatch_suspends = !sites.is_empty();
        sites.extend(
            clause_sites
                .into_iter()
                .enumerate()
                .map(|(site_index, site)| AsyncAwaitSite {
                    outer_index,
                    dependency: site.dependency,
                    kind: AsyncAwaitKind::Switch {
                        switch_id,
                        expr: expr.clone(),
                        cases: continuation_cases.clone(),
                        default: default.clone(),
                        default_index,
                        await_clause: site.await_clause,
                        previous: site.previous,
                        before: site.before,
                        after: site.after,
                        action: site.action,
                        first_in_clause: site.first_in_clause,
                        final_in_clause: site.final_in_clause,
                        first_in_switch: !dispatch_suspends && site_index == 0,
                        final_in_switch: site_index + 1 == clause_site_count,
                        entry_guard: site.entry_guard,
                        context: context.clone(),
                    },
                }),
        );
        sites
    }

    pub(super) fn collect_async_await_sites(&mut self, body: &[Stmt]) -> Vec<AsyncAwaitSite> {
        body.iter()
            .enumerate()
            .flat_map(|(outer_index, stmt)| match stmt.kind() {
                StmtKind::Expr(expr) => {
                    self.async_expr_statement_await_sites(outer_index, stmt, expr)
                }
                StmtKind::VarDecl {
                    name,
                    ty,
                    value: Some(value),
                    onfail,
                    ..
                } => match value.kind() {
                    ExprKind::Await(dependency) if first_await_expression(dependency).is_none() => {
                        self.resolve_onfail_binding_type(ty, value, onfail.is_some())
                            .ok()
                            .map(|binding_ty| {
                                vec![AsyncAwaitSite {
                                    outer_index,
                                    dependency: dependency.as_ref().clone(),
                                    kind: AsyncAwaitKind::Binding {
                                        name: name.clone(),
                                        ty: binding_ty,
                                        onfail: onfail.clone(),
                                        declared_ty: onfail.as_ref().map(|_| ty.clone()),
                                        value: onfail.as_ref().map(|_| value.clone()),
                                    },
                                }]
                            })
                            .unwrap_or_default()
                    }
                    _ => self.async_expression_await_sites(stmt, outer_index),
                },
                StmtKind::Assign {
                    name,
                    op: AssignOp::Assign,
                    value,
                } => match value.kind() {
                    ExprKind::Await(dependency) if first_await_expression(dependency).is_none() => {
                        vec![AsyncAwaitSite {
                            outer_index,
                            dependency: dependency.as_ref().clone(),
                            kind: AsyncAwaitKind::Assignment { name: name.clone() },
                        }]
                    }
                    _ => self.async_expression_await_sites(stmt, outer_index),
                },
                StmtKind::Return(Some(expr)) => match expr.kind() {
                    ExprKind::Await(dependency) if first_await_expression(dependency).is_none() => {
                        vec![AsyncAwaitSite {
                            outer_index,
                            dependency: dependency.as_ref().clone(),
                            kind: AsyncAwaitKind::Direct { returns: true },
                        }]
                    }
                    _ => self.async_expression_await_sites(stmt, outer_index),
                },
                StmtKind::While { cond, body } => {
                    self.async_while_await_sites(outer_index, stmt, cond, body)
                }
                StmtKind::DoWhile { body, cond } => {
                    self.async_do_while_await_sites(outer_index, stmt, body, cond)
                }
                StmtKind::For {
                    init,
                    cond,
                    update,
                    body,
                } => self.async_for_await_sites(outer_index, stmt, init, cond, update, body),
                StmtKind::ForOf {
                    name,
                    iterable,
                    body: loop_body,
                    ..
                } if first_await_expression(iterable).is_none()
                    && matches!(iterable.kind(), ExprKind::Ident(_)) =>
                {
                    self.async_for_of_await_sites(
                        outer_index,
                        stmt,
                        body,
                        name,
                        iterable,
                        loop_body,
                    )
                }
                StmtKind::Switch {
                    expr,
                    cases,
                    default,
                    default_index,
                } => self.async_switch_await_sites(
                    outer_index,
                    expr,
                    cases,
                    default,
                    *default_index,
                    None,
                ),
                StmtKind::TryCatch { .. } => self.async_try_await_sites(outer_index, stmt),
                _ => self.async_expression_await_sites(stmt, outer_index),
            })
            .collect()
    }

    pub(super) fn collect_async_catch_await_groups(
        &self,
        body: &[Stmt],
        await_sites: &[AsyncAwaitSite],
    ) -> Vec<AsyncCatchAwaitGroup> {
        let collect_sites = |catch_body: &[Stmt]| {
            let sites = direct_awaits_in_block(catch_body)?;
            (!sites.is_empty()).then(|| {
                sites
                    .iter()
                    .enumerate()
                    .map(|(site_index, (await_index, dependency, action))| {
                        let start = site_index
                            .checked_sub(1)
                            .map_or(0, |previous| sites[previous].0 + 1);
                        let final_in_catch = site_index + 1 == sites.len();
                        AsyncCatchAwaitSite {
                            dependency: dependency.clone(),
                            before: catch_body[start..*await_index].to_vec(),
                            after: final_in_catch
                                .then(|| catch_body[await_index + 1..].to_vec())
                                .unwrap_or_default(),
                            action: action.clone(),
                            final_in_catch,
                        }
                    })
                    .collect::<Vec<_>>()
            })
        };
        let mut groups = body
            .iter()
            .enumerate()
            .filter_map(|(outer_index, statement)| {
                let StmtKind::TryCatch {
                    try_body,
                    catch_binding,
                    catch_type,
                    catch_type_span: _,
                    catch_body,
                    finally_body,
                    ..
                } = statement.kind()
                else {
                    return None;
                };
                let suspending_try = await_sites.iter().any(|site| {
                    site.outer_index == outer_index
                        && matches!(site.kind, AsyncAwaitKind::Try { .. })
                });
                let suspending_if = await_sites.iter().any(|site| {
                    site.outer_index == outer_index
                        && matches!(
                            site.kind,
                            AsyncAwaitKind::IfBranch {
                                try_context: Some(_),
                                ..
                            } | AsyncAwaitKind::IfBothBranches {
                                try_context: Some(_),
                                ..
                            }
                        )
                });
                let suspending_switch = await_sites.iter().any(|site| {
                    site.outer_index == outer_index
                        && matches!(
                            site.kind,
                            AsyncAwaitKind::SwitchSubject {
                                context: Some(AsyncSwitchContext::Try { .. }),
                                ..
                            } | AsyncAwaitKind::SwitchCaseExpression {
                                context: Some(AsyncSwitchContext::Try { .. }),
                                ..
                            } | AsyncAwaitKind::Switch {
                                context: Some(AsyncSwitchContext::Try { .. }),
                                ..
                            }
                        )
                });
                let synchronous_try = first_await_in_block(try_body).is_none();
                if !suspending_try && !suspending_if && !suspending_switch && !synchronous_try {
                    return None;
                }
                let sites = collect_sites(catch_body)?;
                Some(AsyncCatchAwaitGroup {
                    outer_index,
                    recursive_try_depth: None,
                    recursive_try_id: None,
                    catch_binding: catch_binding.clone(),
                    catch_type: catch_type.clone(),
                    storage_name: format!("__tyrus_async_catch_error_{outer_index}"),
                    finally_body: finally_body
                        .as_ref()
                        .filter(|body| !block_contains_simple_async_await(body))
                        .cloned()
                        .unwrap_or_default(),
                    finally_at_recovered: suspending_if,
                    sites,
                })
            })
            .collect::<Vec<_>>();
        let mut recursive_groups = HashSet::default();
        for site in await_sites {
            let Some(contexts) = control_flow_contexts(site) else {
                continue;
            };
            for (depth, context) in contexts.iter().enumerate() {
                let AsyncControlContext::Try {
                    try_id,
                    catch_binding,
                    catch_type,
                    catch_body,
                    finally_body,
                    ..
                } = context
                else {
                    continue;
                };
                let key = *try_id;
                if !recursive_groups.insert(key) {
                    continue;
                }
                let Some(sites) = collect_sites(catch_body) else {
                    continue;
                };
                groups.push(AsyncCatchAwaitGroup {
                    outer_index: site.outer_index,
                    recursive_try_depth: Some(depth),
                    recursive_try_id: Some(*try_id),
                    catch_binding: catch_binding.clone(),
                    catch_type: catch_type.clone(),
                    storage_name: format!(
                        "__tyrus_async_recursive_catch_error_{}_{}",
                        site.outer_index, try_id.start
                    ),
                    finally_body: (!block_contains_simple_async_await(finally_body))
                        .then(|| finally_body.clone())
                        .unwrap_or_default(),
                    finally_at_recovered: false,
                    sites,
                });
            }
        }
        groups
    }

    pub(super) fn collect_async_finally_await_groups(
        &self,
        body: &[Stmt],
        await_sites: &[AsyncAwaitSite],
    ) -> Vec<AsyncFinallyAwaitGroup> {
        let mut groups = body
            .iter()
            .enumerate()
            .filter_map(|(outer_index, statement)| {
                let StmtKind::TryCatch {
                    finally_body: Some(finally_body),
                    ..
                } = statement.kind()
                else {
                    return None;
                };
                if !await_sites.iter().any(|site| {
                    site.outer_index == outer_index
                        && matches!(
                            site.kind,
                            AsyncAwaitKind::Try { .. }
                                | AsyncAwaitKind::IfBranch {
                                    try_context: Some(_),
                                    ..
                                }
                                | AsyncAwaitKind::IfBothBranches {
                                    try_context: Some(_),
                                    ..
                                }
                                | AsyncAwaitKind::SwitchSubject {
                                    context: Some(AsyncSwitchContext::Try { .. }),
                                    ..
                                }
                                | AsyncAwaitKind::SwitchCaseExpression {
                                    context: Some(AsyncSwitchContext::Try { .. }),
                                    ..
                                }
                                | AsyncAwaitKind::Switch {
                                    context: Some(AsyncSwitchContext::Try { .. }),
                                    ..
                                }
                        )
                }) {
                    return None;
                }
                let sites = direct_awaits_in_block(finally_body)?;
                if sites.is_empty() {
                    return None;
                }
                let sites = sites
                    .iter()
                    .enumerate()
                    .map(|(site_index, (await_index, dependency, action))| {
                        let start = site_index
                            .checked_sub(1)
                            .map_or(0, |previous| sites[previous].0 + 1);
                        let final_in_finally = site_index + 1 == sites.len();
                        AsyncFinallyAwaitSite {
                            dependency: dependency.clone(),
                            before: finally_body[start..*await_index].to_vec(),
                            after: final_in_finally
                                .then(|| finally_body[await_index + 1..].to_vec())
                                .unwrap_or_default(),
                            action: action.clone(),
                            final_in_finally,
                        }
                    })
                    .collect();
                Some(AsyncFinallyAwaitGroup {
                    outer_index,
                    recursive_try_id: None,
                    mode_storage: format!("__tyrus_async_finally_mode_{outer_index}"),
                    failure_storage: format!("__tyrus_async_finally_failure_{outer_index}"),
                    completion: await_sites
                        .iter()
                        .any(|site| {
                            site.outer_index == outer_index
                                && matches!(
                                    site.kind,
                                    AsyncAwaitKind::SwitchSubject {
                                        context: Some(AsyncSwitchContext::Try { .. }),
                                        ..
                                    } | AsyncAwaitKind::SwitchCaseExpression {
                                        context: Some(AsyncSwitchContext::Try { .. }),
                                        ..
                                    } | AsyncAwaitKind::Switch {
                                        context: Some(AsyncSwitchContext::Try { .. }),
                                        ..
                                    }
                                )
                        })
                        .then(|| AsyncFinallyCompletionStorage {
                            mode_storage: format!(
                                "__tyrus_async_switch_finally_completion_mode_{outer_index}"
                            ),
                            throw_storage: format!(
                                "__tyrus_async_switch_finally_completion_throw_{outer_index}"
                            ),
                            return_storage: format!(
                                "__tyrus_async_switch_finally_completion_return_{outer_index}"
                            ),
                        }),
                    sites,
                })
            })
            .collect::<Vec<_>>();
        let mut recursive_groups = HashSet::default();
        for site in await_sites {
            let Some(contexts) = control_flow_contexts(site) else {
                continue;
            };
            for context in contexts {
                let AsyncControlContext::Try {
                    try_id,
                    finally_body,
                    ..
                } = context
                else {
                    continue;
                };
                if !recursive_groups.insert(*try_id) {
                    continue;
                }
                let Some(finally_sites) = direct_awaits_in_block(finally_body) else {
                    continue;
                };
                if finally_sites.is_empty() {
                    continue;
                }
                let sites = finally_sites
                    .iter()
                    .enumerate()
                    .map(|(site_index, (await_index, dependency, action))| {
                        let start = site_index
                            .checked_sub(1)
                            .map_or(0, |previous| finally_sites[previous].0 + 1);
                        let final_in_finally = site_index + 1 == finally_sites.len();
                        AsyncFinallyAwaitSite {
                            dependency: dependency.clone(),
                            before: finally_body[start..*await_index].to_vec(),
                            after: final_in_finally
                                .then(|| finally_body[await_index + 1..].to_vec())
                                .unwrap_or_default(),
                            action: action.clone(),
                            final_in_finally,
                        }
                    })
                    .collect();
                let suffix = format!("{}_{}", site.outer_index, try_id.start);
                groups.push(AsyncFinallyAwaitGroup {
                    outer_index: site.outer_index,
                    recursive_try_id: Some(*try_id),
                    mode_storage: format!("__tyrus_async_recursive_finally_mode_{suffix}"),
                    failure_storage: format!("__tyrus_async_recursive_finally_failure_{suffix}"),
                    completion: Some(AsyncFinallyCompletionStorage {
                        mode_storage: format!(
                            "__tyrus_async_recursive_finally_completion_mode_{suffix}"
                        ),
                        throw_storage: format!(
                            "__tyrus_async_recursive_finally_completion_throw_{suffix}"
                        ),
                        return_storage: format!(
                            "__tyrus_async_recursive_finally_completion_return_{suffix}"
                        ),
                    }),
                    sites,
                });
            }
        }
        groups
    }
}
