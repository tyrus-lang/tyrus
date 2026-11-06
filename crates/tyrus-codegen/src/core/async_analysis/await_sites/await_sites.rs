use super::*;

impl Codegen<'_> {
    pub(super) fn async_expr_statement_await_sites(
        &mut self,
        outer_index: usize,
        stmt: &Stmt,
        expr: &Expr,
    ) -> Vec<AsyncAwaitSite> {
        match expr.kind() {
            ExprKind::Await(dependency) if first_await_expression(dependency).is_none() => {
                vec![AsyncAwaitSite {
                    outer_index,
                    dependency: dependency.as_ref().clone(),
                    kind: AsyncAwaitKind::Direct { returns: false },
                }]
            }
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } if first_await_expression(cond).is_some()
                && first_await_in_block(then_block).is_none()
                && else_block
                    .as_deref()
                    .is_none_or(|body| first_await_in_block(body).is_none()) =>
            {
                let condition = match cond.kind() {
                    ExprKind::Await(dependency) if first_await_expression(dependency).is_none() => {
                        Some((dependency.as_ref().clone(), None))
                    }
                    _ => self.async_nested_condition(cond, outer_index).and_then(
                        |(dependency, condition)| {
                            condition
                                .guard
                                .is_none()
                                .then_some((dependency, Some(condition)))
                        },
                    ),
                };
                condition
                    .map(|(dependency, condition)| {
                        vec![AsyncAwaitSite {
                            outer_index,
                            dependency,
                            kind: AsyncAwaitKind::IfCondition {
                                then_block: then_block.clone(),
                                else_block: else_block.clone().unwrap_or_default(),
                                condition,
                            },
                        }]
                    })
                    .unwrap_or_default()
            }
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } if first_await_expression(cond).is_none() => {
                if let Some(else_block) = else_block.as_deref()
                    && let Some(sites) = self.async_if_both_branches_sites(
                        outer_index,
                        cond,
                        then_block,
                        else_block,
                        None,
                    )
                {
                    return sites;
                }
                let direct = self.async_if_branch_await_site(
                    stmt,
                    outer_index,
                    cond,
                    then_block,
                    else_block.as_deref(),
                    None,
                    None,
                );
                if !direct.is_empty() {
                    return direct;
                }
                let other = else_block.clone().unwrap_or_default();
                let nested =
                    self.async_switch_in_context_block(outer_index, then_block, |before, after| {
                        AsyncSwitchContext::If {
                            cond: cond.as_ref().clone(),
                            before,
                            after,
                            other: other.clone(),
                            switch_in_then: true,
                        }
                    });
                if !nested.is_empty() {
                    return nested;
                }
                let Some(else_block) = else_block.as_deref() else {
                    return self.async_recursive_control_await_sites(outer_index, stmt);
                };
                let nested =
                    self.async_switch_in_context_block(outer_index, else_block, |before, after| {
                        AsyncSwitchContext::If {
                            cond: cond.as_ref().clone(),
                            before,
                            after,
                            other: then_block.clone(),
                            switch_in_then: false,
                        }
                    });
                if nested.is_empty() {
                    self.async_recursive_control_await_sites(outer_index, stmt)
                } else {
                    nested
                }
            }
            _ => self.async_expression_await_sites(stmt, outer_index),
        }
    }

    pub(super) fn async_while_await_sites(
        &mut self,
        outer_index: usize,
        stmt: &Stmt,
        cond: &Expr,
        body: &[Stmt],
    ) -> Vec<AsyncAwaitSite> {
        match cond.kind() {
            ExprKind::Await(dependency)
                if body.iter().all(|statement| {
                    first_await_in_simple_async_statement(statement).is_none()
                }) =>
            {
                vec![AsyncAwaitSite {
                    outer_index,
                    dependency: dependency.as_ref().clone(),
                    kind: AsyncAwaitKind::WhileCondition {
                        body: body.to_vec(),
                        condition: None,
                    },
                }]
            }
            _ if first_await_expression(cond).is_some()
                && body.iter().all(|statement| {
                    first_await_in_simple_async_statement(statement).is_none()
                }) =>
            {
                self.async_nested_condition(cond, outer_index)
                    .map(|(dependency, condition)| {
                        vec![AsyncAwaitSite {
                            outer_index,
                            dependency,
                            kind: AsyncAwaitKind::WhileCondition {
                                body: body.to_vec(),
                                condition: Some(condition),
                            },
                        }]
                    })
                    .unwrap_or_default()
            }
            _ if first_await_expression(cond).is_none() => {
                let direct = direct_awaits_in_block(body)
                    .map(|sites| {
                        sites
                            .iter()
                            .enumerate()
                            .map(|(site_index, (await_index, dependency, action))| {
                                let start = site_index
                                    .checked_sub(1)
                                    .map_or(0, |previous| sites[previous].0 + 1);
                                let final_in_body = site_index + 1 == sites.len();
                                AsyncAwaitSite {
                                    outer_index,
                                    dependency: dependency.clone(),
                                    kind: AsyncAwaitKind::WhileBody {
                                        cond: cond.clone(),
                                        before: body[start..*await_index].to_vec(),
                                        after: final_in_body
                                            .then(|| body[await_index + 1..].to_vec())
                                            .unwrap_or_default(),
                                        action: action.clone(),
                                        first_in_body: site_index == 0,
                                        final_in_body,
                                    },
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if !direct.is_empty() {
                    direct
                } else {
                    let nested_switch =
                        self.async_switch_in_context_block(outer_index, body, |before, after| {
                            AsyncSwitchContext::While {
                                cond: cond.clone(),
                                before,
                                after,
                            }
                        });
                    if nested_switch.is_empty() {
                        let nested_if = self.async_if_in_control_body_await_sites(
                            outer_index,
                            body,
                            AsyncControlContext::While {
                                condition: cond.clone(),
                                before: Vec::new(),
                                after: Vec::new(),
                            },
                        );
                        if nested_if.is_empty() {
                            self.async_recursive_control_await_sites(outer_index, stmt)
                        } else {
                            nested_if
                        }
                    } else {
                        nested_switch
                    }
                }
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn async_do_while_await_sites(
        &mut self,
        outer_index: usize,
        stmt: &Stmt,
        body: &[Stmt],
        cond: &Expr,
    ) -> Vec<AsyncAwaitSite> {
        match cond.kind() {
            ExprKind::Await(dependency)
                if body.iter().all(|statement| {
                    first_await_in_simple_async_statement(statement).is_none()
                }) =>
            {
                vec![AsyncAwaitSite {
                    outer_index,
                    dependency: dependency.as_ref().clone(),
                    kind: AsyncAwaitKind::DoWhileCondition {
                        body: body.to_vec(),
                        condition: None,
                    },
                }]
            }
            _ if first_await_expression(cond).is_some()
                && body.iter().all(|statement| {
                    first_await_in_simple_async_statement(statement).is_none()
                }) =>
            {
                self.async_nested_condition(cond, outer_index)
                    .map(|(dependency, condition)| {
                        vec![AsyncAwaitSite {
                            outer_index,
                            dependency,
                            kind: AsyncAwaitKind::DoWhileCondition {
                                body: body.to_vec(),
                                condition: Some(condition),
                            },
                        }]
                    })
                    .unwrap_or_default()
            }
            _ if first_await_expression(cond).is_none() => {
                let direct = direct_awaits_in_block(body)
                    .map(|sites| {
                        sites
                            .iter()
                            .enumerate()
                            .map(|(site_index, (await_index, dependency, action))| {
                                let start = site_index
                                    .checked_sub(1)
                                    .map_or(0, |previous| sites[previous].0 + 1);
                                let final_in_body = site_index + 1 == sites.len();
                                AsyncAwaitSite {
                                    outer_index,
                                    dependency: dependency.clone(),
                                    kind: AsyncAwaitKind::DoWhileBody {
                                        cond: cond.clone(),
                                        before: body[start..*await_index].to_vec(),
                                        after: final_in_body
                                            .then(|| body[await_index + 1..].to_vec())
                                            .unwrap_or_default(),
                                        action: action.clone(),
                                        first_in_body: site_index == 0,
                                        final_in_body,
                                    },
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if !direct.is_empty() {
                    direct
                } else {
                    let nested_switch =
                        self.async_switch_in_context_block(outer_index, body, |before, after| {
                            AsyncSwitchContext::DoWhile {
                                cond: cond.clone(),
                                before,
                                after,
                            }
                        });
                    if nested_switch.is_empty() {
                        let nested_if = self.async_if_in_control_body_await_sites(
                            outer_index,
                            body,
                            AsyncControlContext::DoWhile {
                                condition: cond.clone(),
                                before: Vec::new(),
                                after: Vec::new(),
                            },
                        );
                        if nested_if.is_empty() {
                            self.async_recursive_control_await_sites(outer_index, stmt)
                        } else {
                            nested_if
                        }
                    } else {
                        nested_switch
                    }
                }
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn async_for_await_sites(
        &mut self,
        outer_index: usize,
        stmt: &Stmt,
        init: &Stmt,
        cond: &Expr,
        update: &Stmt,
        body: &[Stmt],
    ) -> Vec<AsyncAwaitSite> {
        match cond.kind() {
            ExprKind::Await(dependency)
                if first_await_in_simple_async_statement(init).is_none()
                    && first_await_in_simple_async_statement(update).is_none()
                    && body.iter().all(|statement| {
                        first_await_in_simple_async_statement(statement).is_none()
                    }) =>
            {
                vec![AsyncAwaitSite {
                    outer_index,
                    dependency: dependency.as_ref().clone(),
                    kind: AsyncAwaitKind::ForCondition {
                        init: Box::new(init.clone()),
                        update: Box::new(update.clone()),
                        body: body.to_vec(),
                        condition: None,
                    },
                }]
            }
            _ if first_await_expression(cond).is_some()
                && first_await_in_simple_async_statement(init).is_none()
                && first_await_in_simple_async_statement(update).is_none()
                && body.iter().all(|statement| {
                    first_await_in_simple_async_statement(statement).is_none()
                }) =>
            {
                self.async_nested_condition(cond, outer_index)
                    .map(|(dependency, condition)| {
                        vec![AsyncAwaitSite {
                            outer_index,
                            dependency,
                            kind: AsyncAwaitKind::ForCondition {
                                init: Box::new(init.clone()),
                                update: Box::new(update.clone()),
                                body: body.to_vec(),
                                condition: Some(condition),
                            },
                        }]
                    })
                    .unwrap_or_default()
            }
            _ if first_await_in_simple_async_statement(init).is_none()
                && first_await_expression(cond).is_none()
                && first_await_in_simple_async_statement(update).is_none() =>
            {
                let direct = direct_awaits_in_block(body)
                    .map(|sites| {
                        sites
                            .iter()
                            .enumerate()
                            .map(|(site_index, (await_index, dependency, action))| {
                                let start = site_index
                                    .checked_sub(1)
                                    .map_or(0, |previous| sites[previous].0 + 1);
                                let final_in_body = site_index + 1 == sites.len();
                                AsyncAwaitSite {
                                    outer_index,
                                    dependency: dependency.clone(),
                                    kind: AsyncAwaitKind::ForBody {
                                        init: Box::new(init.clone()),
                                        cond: cond.clone(),
                                        update: Box::new(update.clone()),
                                        before: body[start..*await_index].to_vec(),
                                        after: final_in_body
                                            .then(|| body[await_index + 1..].to_vec())
                                            .unwrap_or_default(),
                                        action: action.clone(),
                                        first_in_body: site_index == 0,
                                        final_in_body,
                                    },
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if !direct.is_empty() {
                    direct
                } else {
                    let nested_switch =
                        self.async_switch_in_context_block(outer_index, body, |before, after| {
                            AsyncSwitchContext::For {
                                init: Box::new(init.clone()),
                                cond: cond.clone(),
                                update: Box::new(update.clone()),
                                before,
                                after,
                            }
                        });
                    if nested_switch.is_empty() {
                        let nested_if = self.async_if_in_control_body_await_sites(
                            outer_index,
                            body,
                            AsyncControlContext::For {
                                init: Box::new(init.clone()),
                                condition: cond.clone(),
                                update: Box::new(update.clone()),
                                before: Vec::new(),
                                after: Vec::new(),
                            },
                        );
                        if nested_if.is_empty() {
                            self.async_recursive_control_await_sites(outer_index, stmt)
                        } else {
                            nested_if
                        }
                    } else {
                        nested_switch
                    }
                }
            }
            _ => Vec::new(),
        }
    }

    pub(super) fn async_for_of_await_sites(
        &mut self,
        outer_index: usize,
        stmt: &Stmt,
        body: &[Stmt],
        name: &str,
        iterable: &Expr,
        loop_body: &[Stmt],
    ) -> Vec<AsyncAwaitSite> {
        let iterable_type = self.expr_type_hint(iterable).or_else(|| {
            let ExprKind::Ident(iterable_name) = iterable.kind() else {
                return None;
            };
            body[..outer_index].iter().find_map(|previous| {
                let StmtKind::VarDecl {
                    name, ty, value, ..
                } = previous.kind()
                else {
                    return None;
                };
                (name == iterable_name)
                    .then(|| self.resolve_binding_type(ty, value.as_ref()).ok())
                    .flatten()
            })
        });
        let element_type = iterable_type.and_then(|ty| match transparent_codegen_type(&ty) {
            Type::Array(element) => Some(self.resolve_type(element)),
            _ => None,
        });
        element_type
            .map(|element_type| {
                let direct = direct_awaits_in_block(loop_body)
                    .map(|sites| {
                        sites
                            .iter()
                            .enumerate()
                            .map(|(site_index, (await_index, dependency, action))| {
                                let start = site_index
                                    .checked_sub(1)
                                    .map_or(0, |previous| sites[previous].0 + 1);
                                let final_in_body = site_index + 1 == sites.len();
                                AsyncAwaitSite {
                                    outer_index,
                                    dependency: dependency.clone(),
                                    kind: AsyncAwaitKind::ForOfBody {
                                        name: name.to_string(),
                                        iterable: iterable.clone(),
                                        element_type: element_type.clone(),
                                        before: loop_body[start..*await_index].to_vec(),
                                        after: final_in_body
                                            .then(|| loop_body[await_index + 1..].to_vec())
                                            .unwrap_or_default(),
                                        action: action.clone(),
                                        first_in_body: site_index == 0,
                                        final_in_body,
                                    },
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if !direct.is_empty() {
                    direct
                } else {
                    let nested_switch = self.async_switch_in_context_block(
                        outer_index,
                        loop_body,
                        |before, after| AsyncSwitchContext::ForOf {
                            name: name.to_string(),
                            iterable: iterable.clone(),
                            element_type: element_type.clone(),
                            before,
                            after,
                        },
                    );
                    if nested_switch.is_empty() {
                        let nested_if = self.async_if_in_control_body_await_sites(
                            outer_index,
                            loop_body,
                            AsyncControlContext::ForOf {
                                name: name.to_string(),
                                iterable: iterable.clone(),
                                element_type: element_type.clone(),
                                before: Vec::new(),
                                after: Vec::new(),
                            },
                        );
                        if nested_if.is_empty() {
                            self.async_recursive_control_await_sites(outer_index, stmt)
                        } else {
                            nested_if
                        }
                    } else {
                        nested_switch
                    }
                }
            })
            .unwrap_or_default()
    }

    pub(super) fn async_try_await_sites(
        &mut self,
        outer_index: usize,
        stmt: &Stmt,
    ) -> Vec<AsyncAwaitSite> {
        let StmtKind::TryCatch {
            try_body,
            catch_binding,
            catch_type,
            catch_body,
            finally_body,
            ..
        } = stmt.kind()
        else {
            return Vec::new();
        };
        if first_await_in_block(try_body).is_none()
            && first_await_in_block(catch_body).is_none()
            && let Some(finally_sites) = finally_body
                .as_deref()
                .and_then(direct_awaits_in_block)
                .filter(|sites| !sites.is_empty())
        {
            let return_storage = self
                .return_types
                .last()
                .filter(|ty| !matches!(transparent_codegen_type(ty), Type::Void))
                .map(|_| format!("__tyrus_async_finally_return_{outer_index}"));
            return finally_sites
                .iter()
                .enumerate()
                .map(|(site_index, (await_index, dependency, action))| {
                    let start = site_index
                        .checked_sub(1)
                        .map_or(0, |previous| finally_sites[previous].0 + 1);
                    let final_in_finally = site_index + 1 == finally_sites.len();
                    AsyncAwaitSite {
                        outer_index,
                        dependency: dependency.clone(),
                        kind: AsyncAwaitKind::FinallyCompletion {
                            contexts: Vec::new(),
                            enter_depth: 0,
                            exit_depth: 0,
                            try_body: try_body.clone(),
                            catch_binding: catch_binding.clone(),
                            catch_type: catch_type.clone(),
                            catch_body: catch_body.clone(),
                            before: finally_body.as_ref().unwrap()[start..*await_index].to_vec(),
                            after: final_in_finally
                                .then(|| finally_body.as_ref().unwrap()[await_index + 1..].to_vec())
                                .unwrap_or_default(),
                            action: action.clone(),
                            mode_storage: format!("__tyrus_async_finally_mode_{outer_index}"),
                            throw_storage: format!("__tyrus_async_finally_throw_{outer_index}"),
                            return_storage: return_storage.clone(),
                            first_in_finally: site_index == 0,
                            final_in_finally,
                        },
                    }
                })
                .collect();
        }
        let lowerable_handlers = (first_await_in_block(catch_body).is_none()
            || direct_awaits_in_block(catch_body).is_some())
            && finally_body.as_deref().is_none_or(|body| {
                first_await_in_block(body).is_none() || direct_awaits_in_block(body).is_some()
            });
        if lowerable_handlers {
            let nested =
                self.async_switch_in_context_block(outer_index, try_body, |before, after| {
                    AsyncSwitchContext::Try {
                        before,
                        after,
                        catch_binding: catch_binding.clone(),
                        catch_type: catch_type.clone(),
                        catch_body: catch_body.clone(),
                        finally_body: finally_body.clone().unwrap_or_default(),
                    }
                });
            if !nested.is_empty() {
                return nested;
            }
        }
        let sites = direct_awaits_in_block(try_body).filter(|sites| !sites.is_empty());
        let Some(sites) = sites else {
            if let [only] = &try_body[..]
                && let StmtKind::Expr(expression) = only.kind()
                && let ExprKind::If {
                    cond,
                    then_block,
                    else_block,
                } = expression.kind()
                && first_await_expression(cond).is_none()
            {
                let finally = finally_body
                    .as_ref()
                    .filter(|body| first_await_in_block(body).is_none())
                    .cloned()
                    .unwrap_or_default();
                let context = AsyncTryContext {
                    catch_binding: catch_binding.clone(),
                    catch_type: catch_type.clone(),
                    catch_body: catch_body.clone(),
                    finally_body: finally,
                };
                let context = Some(context);
                if let Some(else_block) = else_block.as_deref()
                    && let Some(sites) = self.async_if_both_branches_sites(
                        outer_index,
                        cond,
                        then_block,
                        else_block,
                        context.clone(),
                    )
                {
                    return sites;
                }
                return self.async_if_branch_await_site(
                    only,
                    outer_index,
                    cond,
                    then_block,
                    else_block.as_deref(),
                    context,
                    None,
                );
            }
            if lowerable_handlers
                && let Some(sites) =
                    self.async_descend_control_statement(outer_index, stmt, &mut Vec::new())
            {
                return sites;
            }
            return Vec::new();
        };
        sites
            .iter()
            .enumerate()
            .map(|(site_index, (await_index, dependency, action))| {
                let start = site_index
                    .checked_sub(1)
                    .map_or(0, |previous| sites[previous].0 + 1);
                let final_in_try = site_index + 1 == sites.len();
                AsyncAwaitSite {
                    outer_index,
                    dependency: dependency.clone(),
                    kind: AsyncAwaitKind::Try {
                        before: try_body[start..*await_index].to_vec(),
                        after: final_in_try
                            .then(|| try_body[await_index + 1..].to_vec())
                            .unwrap_or_default(),
                        catch_binding: catch_binding.clone(),
                        catch_type: catch_type.clone(),
                        catch_body: catch_body.clone(),
                        finally_body: finally_body
                            .as_ref()
                            .filter(|body| !block_contains_simple_async_await(body))
                            .cloned()
                            .unwrap_or_default(),
                        action: action.clone(),
                        final_in_try,
                    },
                }
            })
            .collect()
    }
}
