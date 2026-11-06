use super::*;

impl<'ctx> Codegen<'ctx> {
    fn awaited_result_type_hint(&self, expression: &Expr) -> Option<Type> {
        if let Some(awaited) = self.expr_type_hint(expression)
            && matches!(transparent_codegen_type(&awaited), Type::Result { .. })
        {
            return Some(awaited);
        }
        let ExprKind::Await(dependency) = expression.kind() else {
            return None;
        };
        let promise = self.expr_type_hint(dependency)?;
        let payload = promise_payload_type(transparent_codegen_type(&promise))?;
        matches!(transparent_codegen_type(payload), Type::Result { .. }).then(|| payload.clone())
    }

    pub(super) fn store_async_nested_await_action(
        &mut self,
        function: FunctionValue<'ctx>,
        action: &AsyncNestedAwaitAction,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<(), String> {
        let name = match action {
            AsyncNestedAwaitAction::Discard => return Ok(()),
            AsyncNestedAwaitAction::Binding { name, .. }
            | AsyncNestedAwaitAction::Assignment { name } => name,
        };
        let (slot, _) = self
            .async_frame_slot(name)
            .ok_or_else(|| format!("internal: nested async binding `{name}` has no frame slot"))?;
        self.clear_async_frame_local_before_store(function, name, slot, ty)?;
        self.builder.build_store(slot, value).unwrap();
        self.mark_async_frame_value_initialized(name, true);
        if matches!(action, AsyncNestedAwaitAction::Assignment { .. }) {
            if let Some(ownership) = self.async_frame_string_ownership(name) {
                self.builder
                    .build_store(ownership, self.context.bool_type().const_int(1, false))
                    .unwrap();
            }
            if let Some(ownership) = self.async_frame_string_array_ownership(name) {
                self.builder
                    .build_store(ownership, self.context.i8_type().const_int(2, false))
                    .unwrap();
            }
        }
        self.store_var(name, slot, ty.clone());
        Ok(())
    }

    pub(crate) fn async_owned_string_expression(&self, expression: &Expr) -> bool {
        if self.owned_user_function_call(expression) {
            return true;
        }
        if let ExprKind::FnCall { callee, .. } = expression.kind()
            && callee == "__tyrus_fs_readFileSync"
        {
            return true;
        }
        if let ExprKind::MethodCall { object, method, .. } = expression.kind()
            && method == "join"
            && self
                .expr_type_hint(object)
                .is_some_and(|ty| matches!(transparent_codegen_type(&ty), Type::Array(_)))
        {
            return true;
        }
        if matches!(expression.kind(), ExprKind::BinOp { op: BinOp::Add, .. })
            && self
                .expr_type_hint(expression)
                .is_some_and(|ty| matches!(transparent_codegen_type(&ty), Type::Str))
        {
            return true;
        }
        if matches!(expression.kind(), ExprKind::TemplateLit(_)) {
            return true;
        }
        if let ExprKind::BinOp {
            op: BinOp::NullCoalesce,
            left,
            ..
        } = expression.kind()
            && let Some(Type::Optional(inner)) =
                self.expr_type_hint(left).map(|ty| self.resolve_type(&ty))
        {
            return !inner.is_copy() && self.async_owned_value_supported(&inner);
        }
        let ExprKind::MethodCall {
            object,
            method,
            args,
            ..
        } = expression.kind()
        else {
            return false;
        };
        if owned_string_frame_method(method)
            && self
                .expr_type_hint(object)
                .is_some_and(|ty| matches!(transparent_codegen_type(&ty), Type::Str))
        {
            return true;
        }
        if matches!(
            tyrus_core::automatic_data_methods::AutomaticDataMethod::instance(method),
            Some(
                tyrus_core::automatic_data_methods::AutomaticDataMethod::JsonEncode
                    | tyrus_core::automatic_data_methods::AutomaticDataMethod::Debug
            )
        ) && self.automatic_data_method_for(object, method).is_some()
        {
            return true;
        }
        if tyrus_core::async_types::owned_string_handle_method(method)
            && self.expr_type_hint(object).is_some_and(|ty| {
                tyrus_core::native_handles::is_handle_type(transparent_codegen_type(&ty))
            })
        {
            return true;
        }
        if self.numeric_to_string_allocates(object, method, args) {
            return true;
        }
        self.crypto_call_target(object, method)
            .is_some_and(tyrus_core::crypto::method_returns_owned_string)
    }

    pub(crate) fn async_owned_copy_array_expression(&self, expression: &Expr) -> bool {
        if let ExprKind::MethodCall { object, method, .. } = expression.kind()
            && tyrus_core::async_types::owned_array_handle_method(method)
            && self.expr_type_hint(object).is_some_and(|ty| {
                tyrus_core::native_handles::is_handle_type(transparent_codegen_type(&ty))
            })
        {
            return true;
        }
        self.owned_user_function_call(expression)
            || matches!(expression.kind(), ExprKind::ArrayLit(_))
            || matches!(expression.kind(), ExprKind::ArrayRepeat { .. })
            || matches!(
                expression.kind(),
                ExprKind::MethodCall { method, .. } if owned_copy_array_frame_method(method)
            )
    }

    pub(super) fn async_owned_string_array_expression(&self, expression: &Expr) -> bool {
        self.owned_user_function_call(expression)
            || matches!(
                expression.kind(),
                ExprKind::MethodCall { object, method, .. }
                    if owned_string_array_frame_method(method)
                        && self.expr_type_hint(object).is_some_and(|ty| {
                            matches!(transparent_codegen_type(&ty), Type::Str | Type::Regex)
                                || matches!(transparent_codegen_type(&ty), Type::Array(element) if matches!(transparent_codegen_type(element), Type::Str))
                        })
            )
    }

    pub(crate) fn async_string_array_ownership_mode(&self, expression: &Expr) -> Option<u64> {
        if static_string_array_frame_literal(expression) {
            Some(1)
        } else if self.async_owned_string_array_expression(expression)
            || self.owned_string_array_literal_expression(expression)
        {
            Some(2)
        } else if matches!(expression.kind(), ExprKind::ArrayLit(_))
            || matches!(expression.kind(), ExprKind::ArrayRepeat { .. })
        {
            Some(1)
        } else {
            None
        }
    }

    pub(super) fn async_static_object_expression(&self, ty: &Type, expression: &Expr) -> bool {
        let ExprKind::ObjectLit(values) = expression.kind() else {
            return false;
        };
        self.async_object_fields(ty).is_some_and(|fields| {
            fields.iter().all(|field| {
                if field.optional {
                    return false;
                }
                let Some((_, value)) = values.iter().find(|(name, _)| name == &field.name) else {
                    return false;
                };
                let field_type = self.resolve_type(&field_storage_type(field));
                field_type.is_copy()
                    || matches!(transparent_codegen_type(&field_type), Type::Str)
                        && matches!(value.kind(), ExprKind::StrLit(_))
                    || async_string_array(&field_type) && static_string_array_frame_literal(value)
            })
        })
    }

    pub(super) fn async_copy_object_expression(&self, ty: &Type, expression: &Expr) -> bool {
        let ExprKind::ObjectLit(values) = expression.kind() else {
            return false;
        };
        self.async_object_fields(ty).is_some_and(|fields| {
            fields.iter().all(|field| {
                !field.optional
                    && self.resolve_type(&field_storage_type(field)).is_copy()
                    && values.iter().any(|(name, _)| name == &field.name)
            })
        })
    }

    pub(super) fn resolved_async_fn_ret_type(
        &self,
        name: &str,
        params: &[Param],
        ret_type: &Type,
        body: &[Stmt],
    ) -> Result<Type, String> {
        let inferred = self.resolved_fn_ret_type(name, params, ret_type, body)?;
        if promise_payload_type(&inferred).is_some() {
            Ok(inferred)
        } else {
            Ok(Type::Apply {
                name: "Promise".to_string(),
                args: vec![inferred],
            })
        }
    }

    pub(super) fn forward_declare_async_main(
        &mut self,
        name: &str,
        params: &[Param],
        ret_type: &Type,
    ) {
        let fn_type = self.context.i32_type().fn_type(&[], false);
        let fn_val = self.module.add_function(name, fn_type, None);
        self.fns.insert(name.to_string(), fn_val);
        self.fn_signatures
            .insert(name.to_string(), (Vec::new(), ret_type.clone()));
        self.fn_param_metadata
            .insert(name.to_string(), params.to_vec());
    }

    pub(super) fn async_nested_condition(
        &mut self,
        cond: &Expr,
        outer_index: usize,
    ) -> Option<(Expr, AsyncConditionExpr)> {
        let (continuation, awaited, guard) =
            if let Some((guard, continuation, awaited)) = guarded_nested_condition_await(cond) {
                (continuation, awaited, Some(guard.clone()))
            } else {
                (cond, nested_condition_await(cond)?, None)
            };
        let ExprKind::Await(dependency) = awaited.kind() else {
            return None;
        };
        let promise = self.expr_type_hint(dependency)?;
        let payload = promise_payload_type(transparent_codegen_type(&promise))?;
        let payload = self.resolve_type(payload);
        if !payload.is_copy() {
            return None;
        }
        let mut expression = continuation.clone();
        let mut id = 0usize;
        assign_async_expression_ids(&mut expression, outer_index, &mut id);
        let mut awaits = Vec::new();
        collect_await_expressions(&expression, &mut awaits);
        let span = awaits.first()?.span;
        Some((
            dependency.as_ref().clone(),
            AsyncConditionExpr {
                expression,
                span,
                payload,
                guard,
            },
        ))
    }

    pub(super) fn async_descend_control_block(
        &mut self,
        outer_index: usize,
        body: &[Stmt],
        contexts: &mut Vec<AsyncControlContext>,
    ) -> Option<Vec<AsyncAwaitSite>> {
        if let Some(sites) = direct_awaits_in_block(body).filter(|sites| !sites.is_empty()) {
            return Some(
                sites
                    .iter()
                    .enumerate()
                    .map(|(site_index, (await_index, dependency, action))| {
                        let start = site_index
                            .checked_sub(1)
                            .map_or(0, |previous| sites[previous].0 + 1);
                        let final_in_path = site_index + 1 == sites.len();
                        AsyncAwaitSite {
                            outer_index,
                            dependency: dependency.clone(),
                            kind: AsyncAwaitKind::ControlFlow {
                                contexts: contexts.clone(),
                                before: body[start..*await_index].to_vec(),
                                after: final_in_path
                                    .then(|| body[await_index + 1..].to_vec())
                                    .unwrap_or_default(),
                                action: action.clone(),
                                enter_depth: if site_index == 0 { 0 } else { contexts.len() },
                                exit_depth: if final_in_path { 0 } else { contexts.len() },
                            },
                        }
                    })
                    .collect(),
            );
        }
        let nested = body
            .iter()
            .enumerate()
            .filter(|(_, statement)| {
                first_await_in_block(std::slice::from_ref(*statement)).is_some()
            })
            .collect::<Vec<_>>();
        if nested.is_empty() {
            return None;
        }
        let mut paths = Vec::new();
        for (path_index, (index, statement)) in nested.iter().enumerate() {
            let previous = path_index
                .checked_sub(1)
                .map_or(0, |previous| nested[previous].0 + 1);
            let next = nested
                .get(path_index + 1)
                .map_or(body.len(), |(index, _)| *index);
            let after = if path_index + 1 == nested.len() {
                body[index + 1..next].to_vec()
            } else {
                Vec::new()
            };
            contexts.push(AsyncControlContext::Block {
                before: body[previous..*index].to_vec(),
                after,
            });
            paths.extend(self.async_descend_control_statement(outer_index, statement, contexts)?);
            contexts.pop();
        }
        for site_index in 0..paths.len() {
            let previous_contexts = site_index
                .checked_sub(1)
                .and_then(|previous| control_flow_contexts(&paths[previous]))
                .map(<[AsyncControlContext]>::to_vec);
            let next_contexts = paths
                .get(site_index + 1)
                .and_then(control_flow_contexts)
                .map(<[AsyncControlContext]>::to_vec);
            let (contexts, enter_depth, exit_depth) =
                control_flow_contexts_mut(&mut paths[site_index])?;
            *enter_depth = previous_contexts.as_deref().map_or(0, |previous| {
                common_control_context_prefix(previous, contexts)
            });
            *exit_depth = next_contexts
                .as_deref()
                .map_or(0, |next| common_control_context_prefix(contexts, next));
        }
        Some(paths)
    }

    pub(super) fn async_descend_control_statement(
        &mut self,
        outer_index: usize,
        statement: &Stmt,
        contexts: &mut Vec<AsyncControlContext>,
    ) -> Option<Vec<AsyncAwaitSite>> {
        if let Some([(await_index, dependency, action)]) =
            direct_awaits_in_block(std::slice::from_ref(statement)).as_deref()
        {
            debug_assert_eq!(*await_index, 0);
            return Some(vec![AsyncAwaitSite {
                outer_index,
                dependency: dependency.clone(),
                kind: AsyncAwaitKind::ControlFlow {
                    contexts: contexts.clone(),
                    before: Vec::new(),
                    after: Vec::new(),
                    action: action.clone(),
                    enter_depth: 0,
                    exit_depth: 0,
                },
            }]);
        }
        if let StmtKind::Switch {
            expr,
            cases,
            default,
            default_index,
        } = statement.kind()
        {
            let sites = self.async_switch_await_sites(
                outer_index,
                expr,
                cases,
                default,
                *default_index,
                Some(AsyncSwitchContext::Recursive {
                    contexts: contexts.clone(),
                    enter_depth: 0,
                    exit_depth: 0,
                }),
            );
            return (!sites.is_empty()).then_some(sites);
        }
        if let StmtKind::TryCatch {
            try_body,
            catch_binding,
            catch_type,
            catch_type_span: _,
            catch_body,
            finally_body: Some(finally_body),
        } = statement.kind()
            && first_await_in_block(try_body).is_none()
            && first_await_in_block(catch_body).is_none()
            && let Some(finally_sites) =
                direct_awaits_in_block(finally_body).filter(|sites| !sites.is_empty())
        {
            let return_storage = self
                .return_types
                .last()
                .filter(|ty| !matches!(transparent_codegen_type(ty), Type::Void))
                .map(|_| format!("__tyrus_async_finally_return_{outer_index}"));
            return Some(
                finally_sites
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
                                contexts: contexts.clone(),
                                enter_depth: if site_index == 0 { 0 } else { contexts.len() },
                                exit_depth: if final_in_finally { 0 } else { contexts.len() },
                                try_body: try_body.clone(),
                                catch_binding: catch_binding.clone(),
                                catch_type: catch_type.clone(),
                                catch_body: catch_body.clone(),
                                before: finally_body[start..*await_index].to_vec(),
                                after: final_in_finally
                                    .then(|| finally_body[await_index + 1..].to_vec())
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
                    .collect(),
            );
        }
        let context_and_body = match statement.kind() {
            StmtKind::Expr(expression) => {
                let ExprKind::If {
                    cond,
                    then_block,
                    else_block,
                } = expression.kind()
                else {
                    return None;
                };
                if first_await_expression(cond).is_some() {
                    return None;
                }
                let else_body = else_block.as_deref().unwrap_or_default();
                let then_suspends = first_await_in_block(then_block).is_some();
                let else_suspends = first_await_in_block(else_body).is_some();
                if then_suspends && else_suspends {
                    let mut sites = self.async_if_both_branches_sites(
                        outer_index,
                        cond,
                        then_block,
                        else_body,
                        None,
                    )?;
                    for site in &mut sites {
                        let AsyncAwaitKind::IfBothBranches {
                            contexts: site_contexts,
                            ..
                        } = &mut site.kind
                        else {
                            unreachable!()
                        };
                        site_contexts.clone_from(contexts);
                    }
                    return Some(sites);
                }
                if !then_suspends && !else_suspends {
                    return None;
                }
                if then_suspends {
                    (
                        AsyncControlContext::If {
                            condition: cond.as_ref().clone(),
                            before: Vec::new(),
                            after: Vec::new(),
                            other: else_body.to_vec(),
                            selected_in_then: true,
                        },
                        then_block.as_slice(),
                    )
                } else {
                    (
                        AsyncControlContext::If {
                            condition: cond.as_ref().clone(),
                            before: Vec::new(),
                            after: Vec::new(),
                            other: then_block.clone(),
                            selected_in_then: false,
                        },
                        else_body,
                    )
                }
            }
            StmtKind::While { cond, body } if first_await_expression(cond).is_none() => (
                AsyncControlContext::While {
                    condition: cond.clone(),
                    before: Vec::new(),
                    after: Vec::new(),
                },
                body.as_slice(),
            ),
            StmtKind::DoWhile { body, cond } if first_await_expression(cond).is_none() => (
                AsyncControlContext::DoWhile {
                    condition: cond.clone(),
                    before: Vec::new(),
                    after: Vec::new(),
                },
                body.as_slice(),
            ),
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } if first_await_in_simple_async_statement(init).is_none()
                && first_await_expression(cond).is_none()
                && first_await_in_simple_async_statement(update).is_none() =>
            {
                (
                    AsyncControlContext::For {
                        init: init.clone(),
                        condition: cond.clone(),
                        update: update.clone(),
                        before: Vec::new(),
                        after: Vec::new(),
                    },
                    body.as_slice(),
                )
            }
            StmtKind::ForOf {
                name,
                iterable,
                body,
                ..
            } if first_await_expression(iterable).is_none()
                && matches!(iterable.kind(), ExprKind::Ident(_)) =>
            {
                let iterable_type = self.expr_type_hint(iterable)?;
                let Type::Array(element_type) = transparent_codegen_type(&iterable_type) else {
                    return None;
                };
                (
                    AsyncControlContext::ForOf {
                        name: name.clone(),
                        iterable: iterable.clone(),
                        element_type: self.resolve_type(element_type),
                        before: Vec::new(),
                        after: Vec::new(),
                    },
                    body.as_slice(),
                )
            }
            StmtKind::TryCatch {
                try_body,
                catch_binding,
                catch_type,
                catch_type_span: _,
                catch_body,
                finally_body,
            } if finally_body.as_deref().is_none_or(|body| {
                first_await_in_block(body).is_none() || direct_awaits_in_block(body).is_some()
            }) =>
            {
                (
                    AsyncControlContext::Try {
                        try_id: statement.span,
                        catch_binding: catch_binding.clone(),
                        catch_type: catch_type.clone(),
                        catch_body: catch_body.clone(),
                        finally_body: finally_body.clone().unwrap_or_default(),
                        before: Vec::new(),
                        after: Vec::new(),
                    },
                    try_body.as_slice(),
                )
            }
            _ => return None,
        };
        contexts.push(context_and_body.0);
        let result = self.async_descend_control_block(outer_index, context_and_body.1, contexts);
        contexts.pop();
        result
    }

    pub(super) fn async_if_both_branches_sites(
        &mut self,
        outer_index: usize,
        cond: &Expr,
        then_block: &[Stmt],
        else_block: &[Stmt],
        try_context: Option<AsyncTryContext>,
    ) -> Option<Vec<AsyncAwaitSite>> {
        let then_awaits = direct_awaits_in_block(then_block)?;
        let else_awaits = direct_awaits_in_block(else_block)?;
        if then_awaits.is_empty() || else_awaits.is_empty() {
            return None;
        }
        let if_id = cond.span;
        let total = then_awaits.len() + else_awaits.len();
        let mut sites = Vec::with_capacity(total);
        for (in_then, body, awaits) in [
            (true, then_block, &then_awaits),
            (false, else_block, &else_awaits),
        ] {
            for (site_index, (await_index, dependency, action)) in awaits.iter().enumerate() {
                let start = site_index
                    .checked_sub(1)
                    .map_or(0, |previous| awaits[previous].0 + 1);
                let final_in_branch = site_index + 1 == awaits.len();
                let end = if final_in_branch {
                    body.len()
                } else {
                    awaits[site_index + 1].0
                };
                sites.push(AsyncAwaitSite {
                    outer_index,
                    dependency: dependency.clone(),
                    kind: AsyncAwaitKind::IfBothBranches {
                        contexts: Vec::new(),
                        enter_depth: 0,
                        exit_depth: 0,
                        if_id,
                        cond: cond.clone(),
                        before: body[start..*await_index].to_vec(),
                        after: body[await_index + 1..end].to_vec(),
                        action: action.clone(),
                        in_then,
                        first_in_if: in_then && site_index == 0,
                        first_in_branch: site_index == 0,
                        final_in_branch,
                        try_context: try_context.clone(),
                    },
                });
            }
        }
        Some(sites)
    }

    pub(super) fn prescan_async_local_types(&mut self, body: &[Stmt]) {
        for statement in body {
            match statement.kind() {
                StmtKind::VarDecl {
                    name,
                    ty,
                    value,
                    onfail,
                    ..
                } => {
                    let resolved = if let Some(value) = value {
                        self.resolve_onfail_binding_type(ty, value, onfail.is_some())
                    } else {
                        self.resolve_binding_type(ty, None)
                    };
                    if let Ok(resolved) = resolved {
                        self.async_prescan_types.insert(name.clone(), resolved);
                    }
                }
                StmtKind::ArrayDestructure {
                    bindings, value, ..
                } => {
                    if let Some(tuple_ty) =
                        self.expr_type_hint(value).map(|ty| self.resolve_type(&ty))
                        && let Type::Tuple(elements) = transparent_codegen_type(&tuple_ty)
                    {
                        for (binding, element_ty) in bindings.iter().zip(elements.iter()) {
                            self.async_prescan_types
                                .insert(binding.name.clone(), element_ty.clone());
                        }
                    }
                }
                StmtKind::Expr(expr) => {
                    if let ExprKind::If {
                        then_block,
                        else_block,
                        ..
                    } = expr.kind()
                    {
                        self.prescan_async_local_types(then_block);
                        if let Some(else_block) = else_block {
                            self.prescan_async_local_types(else_block);
                        }
                    }
                }
                StmtKind::While { body, .. }
                | StmtKind::DoWhile { body, .. }
                | StmtKind::ForOf { body, .. }
                | StmtKind::ForIn { body, .. } => self.prescan_async_local_types(body),
                StmtKind::For { init, body, .. } => {
                    self.prescan_async_local_types(std::slice::from_ref(init));
                    self.prescan_async_local_types(body);
                }
                StmtKind::Switch { cases, default, .. } => {
                    for case in cases {
                        self.prescan_async_local_types(&case.body);
                    }
                    if let Some(default) = default {
                        self.prescan_async_local_types(default);
                    }
                }
                StmtKind::TryCatch {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                } => {
                    self.prescan_async_local_types(try_body);
                    self.prescan_async_local_types(catch_body);
                    if let Some(finally_body) = finally_body {
                        self.prescan_async_local_types(finally_body);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn lower_async_onfail_body(
        &mut self,
        body: &[Stmt],
        payload_type: &Type,
    ) -> Result<Vec<Stmt>, String> {
        let mut body = body.to_vec();
        self.lower_awaited_result_onfails(&mut body);
        let mut lowered = Vec::with_capacity(body.len());
        for statement in &body {
            let StmtKind::VarDecl {
                name,
                ty,
                value: Some(value),
                onfail: Some(onfail),
                ..
            } = statement.kind()
            else {
                lowered.push(statement.clone());
                continue;
            };
            let exception = match value.kind() {
                ExprKind::Await(_) => self
                    .awaited_result_type_hint(value)
                    .is_none()
                    .then(|| Type::Named("Error".to_string())),
                ExprKind::FnCall { callee, .. } => self
                    .fn_throws
                    .get(callee)
                    .filter(|effects| effects.len() == 1)
                    .map(|effects| effects[0].clone()),
                _ => None,
            };
            let Some(exception) = exception else {
                lowered.push(statement.clone());
                continue;
            };
            let binding_ty = self.resolve_binding_type(ty, Some(value))?;
            lowered.push(Stmt::synthetic(StmtKind::VarDecl {
                mutable: true,
                name: name.clone(),
                ty: binding_ty,
                value: None,
                onfail: None,
            }));
            let mut catch_body = match &onfail.handler {
                OnFailHandler::Expr(handler) => {
                    vec![Stmt::synthetic(StmtKind::Expr(handler.clone()))]
                }
                OnFailHandler::Block(handler) => handler.clone(),
            };
            catch_body.push(
                if matches!(transparent_codegen_type(payload_type), Type::Void) {
                    Stmt::synthetic(StmtKind::Return(None))
                } else {
                    Stmt::synthetic(StmtKind::Throw(Expr::synthetic(ExprKind::Ident(
                        name.clone(),
                    ))))
                },
            );
            lowered.push(Stmt::synthetic(StmtKind::TryCatch {
                try_body: vec![Stmt::synthetic(StmtKind::Assign {
                    name: name.clone(),
                    op: tyrus_core::ast::AssignOp::Assign,
                    value: value.clone(),
                })],
                catch_binding: name.clone(),
                catch_type: exception,
                catch_type_span: tyrus_core::span::Span::DUMMY,
                catch_body,
                finally_body: None,
            }));
        }
        Ok(lowered)
    }

    fn lower_awaited_result_onfails(&self, body: &mut Vec<Stmt>) {
        let mut index = 0;
        while index < body.len() {
            if let Some(lowered) = self.lower_awaited_result_onfail(&body[index]) {
                let count = lowered.len();
                body.splice(index..=index, lowered);
                index += count - 1;
            }
            self.lower_nested_awaited_result_onfails(&mut body[index]);
            index += 1;
        }
    }

    fn lower_nested_awaited_result_onfails(&self, statement: &mut Stmt) {
        match &mut statement.kind {
            StmtKind::VarDecl {
                onfail: Some(onfail),
                ..
            } => {
                if let OnFailHandler::Block(handler) = &mut onfail.handler {
                    self.lower_awaited_result_onfails(handler);
                }
            }
            StmtKind::Defer { body, .. }
            | StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::For { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::ForIn { body, .. } => self.lower_awaited_result_onfails(body),
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                self.lower_awaited_result_onfails(try_body);
                self.lower_awaited_result_onfails(catch_body);
                if let Some(finally_body) = finally_body {
                    self.lower_awaited_result_onfails(finally_body);
                }
            }
            StmtKind::Switch { cases, default, .. } => {
                for case in cases {
                    self.lower_awaited_result_onfails(&mut case.body);
                }
                if let Some(default) = default {
                    self.lower_awaited_result_onfails(default);
                }
            }
            StmtKind::Match { arms, default, .. } => {
                for arm in arms {
                    self.lower_awaited_result_onfails(&mut arm.body);
                }
                if let Some(default) = default {
                    self.lower_awaited_result_onfails(default);
                }
            }
            StmtKind::Expr(expression) => match &mut expression.kind {
                ExprKind::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    self.lower_awaited_result_onfails(then_block);
                    if let Some(else_block) = else_block {
                        self.lower_awaited_result_onfails(else_block);
                    }
                }
                ExprKind::Loop { body } => self.lower_awaited_result_onfails(body),
                _ => {}
            },
            _ => {}
        }
    }

    /// `const name = await source onfail handler` where the promise settles
    /// with a `Result`: nothing rejects, so the `Err` is separated after the
    /// `await` instead of by a `catch`. The awaited `Result` lands in a hidden
    /// frame local, and the synchronous `onfail` over that local binds the
    /// error in the handler and the success value after it.
    fn lower_awaited_result_onfail(&self, statement: &Stmt) -> Option<Vec<Stmt>> {
        let StmtKind::VarDecl {
            mutable,
            name,
            ty,
            value: Some(value),
            onfail: Some(onfail),
        } = statement.kind()
        else {
            return None;
        };
        if !matches!(value.kind(), ExprKind::Await(_)) {
            return None;
        }
        let awaited = self
            .awaited_result_type_hint(value)
            .map(|ty| self.resolve_type(&ty))?;
        let Type::Result { success, .. } = transparent_codegen_type(&awaited).clone() else {
            return None;
        };
        let hidden = format!(
            "{}onfail{}",
            tyrus_core::await_hoist::HOISTED_AWAIT_PREFIX,
            statement.span.start
        );
        let binding_ty = if matches!(ty, Type::Infer) {
            *success
        } else {
            ty.clone()
        };
        Some(vec![
            Stmt::synthetic(StmtKind::VarDecl {
                mutable: false,
                name: hidden.clone(),
                ty: awaited,
                value: Some(value.clone()),
                onfail: None,
            }),
            Stmt::new(
                StmtKind::VarDecl {
                    mutable: *mutable,
                    name: name.clone(),
                    ty: binding_ty,
                    value: Some(Expr::synthetic(ExprKind::Ident(hidden))),
                    onfail: Some(onfail.clone()),
                },
                statement.span,
            ),
        ])
    }

    pub(super) fn async_switch_in_context_block<F>(
        &mut self,
        outer_index: usize,
        body: &[Stmt],
        context: F,
    ) -> Vec<AsyncAwaitSite>
    where
        F: FnOnce(Vec<Stmt>, Vec<Stmt>) -> AsyncSwitchContext,
    {
        let mut found = None;
        for (index, statement) in body.iter().enumerate() {
            if matches!(statement.kind(), StmtKind::Switch { .. }) {
                if found.is_some() {
                    return Vec::new();
                }
                found = Some((index, statement));
            } else if first_await_in_block(std::slice::from_ref(statement)).is_some() {
                return Vec::new();
            }
        }
        let Some((index, statement)) = found else {
            return Vec::new();
        };
        let StmtKind::Switch {
            expr,
            cases,
            default,
            default_index,
        } = statement.kind()
        else {
            return Vec::new();
        };
        self.async_switch_await_sites(
            outer_index,
            expr,
            cases,
            default,
            *default_index,
            Some(context(body[..index].to_vec(), body[index + 1..].to_vec())),
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_lines)]
    pub(super) fn begin_async_switch_context(
        &mut self,
        outer_index: usize,
        switch_id: Span,
        context: Option<&AsyncSwitchContext>,
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &[AsyncFinallyAwaitGroup],
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_for_of_slots: &HashMap<
            AsyncForOfSlotKey,
            (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
        >,
    ) -> Result<(), String> {
        let Some(context) = context else {
            return Ok(());
        };
        if blocks.switch_context_blocks.contains_key(&switch_id) {
            return Ok(());
        }
        match context {
            AsyncSwitchContext::Recursive {
                contexts,
                enter_depth,
                ..
            } => {
                self.begin_recursive_control_path_from(
                    outer_index,
                    contexts,
                    *enter_depth,
                    await_sites,
                    catch_await_groups,
                    finally_await_groups,
                    blocks,
                    poll,
                    frame_type,
                    frame,
                    state_pointer,
                    async_for_of_slots,
                )?;
                blocks
                    .switch_context_blocks
                    .insert(switch_id, AsyncSwitchContextBlocks::Recursive);
            }
            AsyncSwitchContext::If {
                cond,
                before,
                other,
                switch_in_then,
                ..
            } => {
                let selected = self
                    .context
                    .append_basic_block(poll, "async.switch.context.if.selected");
                let alternative = self
                    .context
                    .append_basic_block(poll, "async.switch.context.if.other");
                let join = self
                    .context
                    .append_basic_block(poll, "async.switch.context.if.join");
                let condition = self.emit_condition_value(cond)?;
                let (then_block, else_block) = if *switch_in_then {
                    (selected, alternative)
                } else {
                    (alternative, selected)
                };
                self.builder
                    .build_conditional_branch(condition, then_block, else_block)
                    .unwrap();
                self.builder.position_at_end(alternative);
                self.push_scope();
                for statement in other {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.pop_scope();
                if !self.current_block_terminated() {
                    self.builder.build_unconditional_branch(join).unwrap();
                }
                self.builder.position_at_end(selected);
                self.push_scope();
                for statement in before {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.pop_scope();
                blocks
                    .switch_context_blocks
                    .insert(switch_id, AsyncSwitchContextBlocks::If { join });
            }
            AsyncSwitchContext::While { cond, before, .. } => {
                self.push_scope();
                let condition = self
                    .context
                    .append_basic_block(poll, "async.switch.context.while.condition");
                let body = self
                    .context
                    .append_basic_block(poll, "async.switch.context.while.body");
                let after = self
                    .context
                    .append_basic_block(poll, "async.switch.context.while.after");
                self.builder.build_unconditional_branch(condition).unwrap();
                self.builder.position_at_end(condition);
                let tested = self.emit_condition_value(cond)?;
                self.builder
                    .build_conditional_branch(tested, body, after)
                    .unwrap();
                self.builder.position_at_end(body);
                self.push_loop_control(condition, after);
                for statement in before {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.pop_loop_control();
                blocks.switch_context_blocks.insert(
                    switch_id,
                    AsyncSwitchContextBlocks::While { condition, after },
                );
            }
            AsyncSwitchContext::DoWhile { before, .. } => {
                self.push_scope();
                let body = self
                    .context
                    .append_basic_block(poll, "async.switch.context.do.body");
                let condition = self
                    .context
                    .append_basic_block(poll, "async.switch.context.do.condition");
                let after = self
                    .context
                    .append_basic_block(poll, "async.switch.context.do.after");
                self.builder.build_unconditional_branch(body).unwrap();
                self.builder.position_at_end(body);
                self.push_loop_control(condition, after);
                for statement in before {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.pop_loop_control();
                blocks.switch_context_blocks.insert(
                    switch_id,
                    AsyncSwitchContextBlocks::DoWhile {
                        condition,
                        body,
                        after,
                    },
                );
            }
            AsyncSwitchContext::For {
                init, cond, before, ..
            } => {
                self.push_scope();
                self.emit_stmt(init, poll)?;
                let condition = self
                    .context
                    .append_basic_block(poll, "async.switch.context.for.condition");
                let body = self
                    .context
                    .append_basic_block(poll, "async.switch.context.for.body");
                let update = self
                    .context
                    .append_basic_block(poll, "async.switch.context.for.update");
                let after = self
                    .context
                    .append_basic_block(poll, "async.switch.context.for.after");
                self.builder.build_unconditional_branch(condition).unwrap();
                self.builder.position_at_end(condition);
                let tested = self.emit_condition_value(cond)?;
                self.builder
                    .build_conditional_branch(tested, body, after)
                    .unwrap();
                self.builder.position_at_end(body);
                self.push_loop_control(update, after);
                for statement in before {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.pop_loop_control();
                blocks.switch_context_blocks.insert(
                    switch_id,
                    AsyncSwitchContextBlocks::For {
                        condition,
                        update,
                        after,
                    },
                );
            }
            AsyncSwitchContext::ForOf {
                name,
                iterable,
                element_type,
                before,
                ..
            } => {
                self.push_scope();
                let (data_slot, length_slot, index_slot) = async_for_of_slots
                    .get(&AsyncForOfSlotKey::SwitchContext(outer_index))
                    .copied()
                    .ok_or_else(|| {
                        "internal: nested async for...of switch has no frame slots".to_string()
                    })?;
                let array = self.emit_expr(iterable)?.into_struct_value();
                let data = self
                    .builder
                    .build_extract_value(array, 0, "async.switch.for_of.data")
                    .unwrap();
                let length = self
                    .builder
                    .build_extract_value(array, 1, "async.switch.for_of.length")
                    .unwrap();
                self.builder.build_store(data_slot, data).unwrap();
                self.builder.build_store(length_slot, length).unwrap();
                self.builder
                    .build_store(index_slot, self.context.i64_type().const_zero())
                    .unwrap();
                let condition = self
                    .context
                    .append_basic_block(poll, "async.switch.context.for_of.condition");
                let body = self
                    .context
                    .append_basic_block(poll, "async.switch.context.for_of.body");
                let update = self
                    .context
                    .append_basic_block(poll, "async.switch.context.for_of.update");
                let after = self
                    .context
                    .append_basic_block(poll, "async.switch.context.for_of.after");
                self.builder.build_unconditional_branch(condition).unwrap();
                self.builder.position_at_end(condition);
                let index = self
                    .builder
                    .build_load(
                        self.context.i64_type(),
                        index_slot,
                        "async.switch.for_of.index",
                    )
                    .unwrap()
                    .into_int_value();
                let length = self
                    .builder
                    .build_load(
                        self.context.i64_type(),
                        length_slot,
                        "async.switch.for_of.length",
                    )
                    .unwrap()
                    .into_int_value();
                let has_next = self
                    .builder
                    .build_int_compare(
                        IntPredicate::ULT,
                        index,
                        length,
                        "async.switch.for_of.has_next",
                    )
                    .unwrap();
                self.builder
                    .build_conditional_branch(has_next, body, after)
                    .unwrap();
                self.builder.position_at_end(body);
                let data = self
                    .builder
                    .build_load(
                        self.context.ptr_type(inkwell::AddressSpace::default()),
                        data_slot,
                        "async.switch.for_of.data.load",
                    )
                    .unwrap()
                    .into_pointer_value();
                let element_pointer = unsafe {
                    self.builder
                        .build_gep(
                            self.llvm_type(element_type),
                            data,
                            &[index],
                            "async.switch.for_of.element.pointer",
                        )
                        .unwrap()
                };
                let element = self
                    .builder
                    .build_load(
                        self.llvm_type(element_type),
                        element_pointer,
                        "async.switch.for_of.element",
                    )
                    .unwrap();
                let (binding_slot, _) = self.async_frame_slot(name).ok_or_else(|| {
                    "internal: nested async for...of binding has no frame slot".to_string()
                })?;
                self.builder.build_store(binding_slot, element).unwrap();
                self.store_var(name, binding_slot, element_type.clone());
                self.push_loop_control(update, after);
                for statement in before {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.pop_loop_control();
                blocks.switch_context_blocks.insert(
                    switch_id,
                    AsyncSwitchContextBlocks::ForOf {
                        condition,
                        update,
                        after,
                        index: index_slot,
                    },
                );
            }
            AsyncSwitchContext::Try {
                before,
                finally_body,
                ..
            } => {
                let join = self
                    .context
                    .append_basic_block(poll, "async.switch.context.try.join");
                blocks.try_blocks.insert(outer_index, join);
                blocks
                    .switch_context_blocks
                    .insert(switch_id, AsyncSwitchContextBlocks::Try { join });
                if !finally_body.is_empty() {
                    self.finally_scopes.push(FinallyScope {
                        body: finally_body.clone(),
                        catches_throw: true,
                        loop_depth: self.loop_stack.len(),
                    });
                }
                for statement in before {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                if !finally_body.is_empty() {
                    self.finally_scopes.pop();
                }
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_lines)]
    pub(super) fn finish_async_switch_context(
        &mut self,
        outer_index: usize,
        switch_id: Span,
        context: Option<&AsyncSwitchContext>,
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &[AsyncFinallyAwaitGroup],
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_for_of_slots: &HashMap<
            AsyncForOfSlotKey,
            (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
        >,
    ) -> Result<(), String> {
        let Some(context) = context else {
            return Ok(());
        };
        let context_blocks = blocks
            .switch_context_blocks
            .get(&switch_id)
            .copied()
            .ok_or_else(|| "internal: nested async switch context is missing".to_string())?;
        match (context, context_blocks) {
            (
                AsyncSwitchContext::Recursive {
                    contexts,
                    exit_depth,
                    ..
                },
                AsyncSwitchContextBlocks::Recursive,
            ) => {
                self.finish_recursive_control_path_to(
                    outer_index,
                    contexts,
                    *exit_depth,
                    await_sites,
                    catch_await_groups,
                    finally_await_groups,
                    blocks,
                    poll,
                    frame_type,
                    frame,
                    state_pointer,
                    async_for_of_slots,
                )?;
            }
            (AsyncSwitchContext::If { after, .. }, AsyncSwitchContextBlocks::If { join }) => {
                self.push_scope();
                for statement in after {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.pop_scope();
                if !self.current_block_terminated() {
                    self.builder.build_unconditional_branch(join).unwrap();
                }
                self.builder.position_at_end(join);
            }
            (
                AsyncSwitchContext::While { after, .. },
                AsyncSwitchContextBlocks::While {
                    condition,
                    after: loop_after,
                    ..
                },
            ) => {
                self.push_loop_control(condition, loop_after);
                for statement in after {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.pop_loop_control();
                if !self.current_block_terminated() {
                    self.builder.build_unconditional_branch(condition).unwrap();
                }
                self.pop_scope();
                self.builder.position_at_end(loop_after);
            }
            (
                AsyncSwitchContext::DoWhile { cond, after, .. },
                AsyncSwitchContextBlocks::DoWhile {
                    condition,
                    body,
                    after: loop_after,
                },
            ) => {
                self.push_loop_control(condition, loop_after);
                for statement in after {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.pop_loop_control();
                if !self.current_block_terminated() {
                    self.builder.build_unconditional_branch(condition).unwrap();
                }
                self.builder.position_at_end(condition);
                let tested = self.emit_condition_value(cond)?;
                self.builder
                    .build_conditional_branch(tested, body, loop_after)
                    .unwrap();
                self.pop_scope();
                self.builder.position_at_end(loop_after);
            }
            (
                AsyncSwitchContext::For { update, after, .. },
                AsyncSwitchContextBlocks::For {
                    condition,
                    update: update_block,
                    after: loop_after,
                    ..
                },
            ) => {
                self.push_loop_control(update_block, loop_after);
                for statement in after {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.pop_loop_control();
                if !self.current_block_terminated() {
                    self.builder
                        .build_unconditional_branch(update_block)
                        .unwrap();
                }
                self.builder.position_at_end(update_block);
                self.emit_stmt(update, poll)?;
                if !self.current_block_terminated() {
                    self.builder.build_unconditional_branch(condition).unwrap();
                }
                self.pop_scope();
                self.builder.position_at_end(loop_after);
            }
            (
                AsyncSwitchContext::ForOf { after, .. },
                AsyncSwitchContextBlocks::ForOf {
                    condition,
                    update,
                    after: loop_after,
                    index,
                },
            ) => {
                self.push_loop_control(update, loop_after);
                for statement in after {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                self.pop_loop_control();
                if !self.current_block_terminated() {
                    self.builder.build_unconditional_branch(update).unwrap();
                }
                self.builder.position_at_end(update);
                let current = self
                    .builder
                    .build_load(
                        self.context.i64_type(),
                        index,
                        "async.switch.for_of.current",
                    )
                    .unwrap()
                    .into_int_value();
                let next = self
                    .builder
                    .build_int_add(
                        current,
                        self.context.i64_type().const_int(1, false),
                        "async.switch.for_of.next",
                    )
                    .unwrap();
                self.builder.build_store(index, next).unwrap();
                self.builder.build_unconditional_branch(condition).unwrap();
                self.pop_scope();
                self.builder.position_at_end(loop_after);
            }
            (
                AsyncSwitchContext::Try {
                    after,
                    finally_body,
                    ..
                },
                AsyncSwitchContextBlocks::Try { join },
            ) => {
                let finally_group = finally_await_groups
                    .iter()
                    .find(|group| group.outer_index == outer_index);
                let async_completion = finally_group.and_then(|group| group.completion.as_ref());
                if !finally_body.is_empty() && async_completion.is_none() {
                    self.finally_scopes.push(FinallyScope {
                        body: finally_body.clone(),
                        catches_throw: true,
                        loop_depth: self.loop_stack.len(),
                    });
                }
                if let Some(completion) = async_completion {
                    let entry = self
                        .context
                        .append_basic_block(poll, "async.switch.finally.completion.entry");
                    let catch_group = catch_await_groups.iter().find(|group| {
                        group.outer_index == outer_index && group.recursive_try_depth.is_none()
                    });
                    let throw_entry = catch_group.map(|_| {
                        self.context
                            .append_basic_block(poll, "async.switch.catch.throw")
                    });
                    let (mode_slot, _) = self
                        .async_frame_slot(&completion.mode_storage)
                        .ok_or_else(|| {
                            "internal: async switch finally completion mode has no frame slot"
                                .to_string()
                        })?;
                    let (throw_slot, _) = self
                        .async_frame_slot(&completion.throw_storage)
                        .ok_or_else(|| {
                            "internal: async switch finally completion throw has no frame slot"
                                .to_string()
                        })?;
                    let return_slot = self
                        .async_frame_slot(&completion.return_storage)
                        .map(|(slot, ty)| (slot, self.resolve_type(&ty)));
                    let return_storage = return_slot
                        .is_some()
                        .then(|| completion.return_storage.clone());
                    blocks.finally_completion_blocks.insert(
                        finally_group.unwrap().mode_storage.clone(),
                        AsyncFinallyCompletionBlocks {
                            entry,
                            normal: join,
                            break_target: self.break_targets.last().map(|target| target.block),
                            continue_target: self.loop_stack.last().map(|(target, _, _)| *target),
                        },
                    );
                    self.async_finally_completion_scopes
                        .push(AsyncFinallyCompletionScope {
                            mode_slot,
                            return_slot,
                            return_storage,
                            entry,
                            throw_entry,
                            throw_storage: completion.throw_storage.clone(),
                            break_target: self.break_targets.last().cloned(),
                            continue_target: self
                                .loop_stack
                                .last()
                                .map(|(target, _, depth)| (*target, *depth)),
                            throw_slot,
                            intercept_throw: true,
                        });
                    for statement in after {
                        if self.current_block_terminated() {
                            break;
                        }
                        self.emit_stmt(statement, poll)?;
                    }
                    self.async_finally_completion_scopes.pop();
                    if !self.current_block_terminated() {
                        self.builder
                            .build_store(mode_slot, self.context.i32_type().const_zero())
                            .unwrap();
                        self.builder.build_unconditional_branch(entry).unwrap();
                    }
                    self.builder.position_at_end(entry);
                    let preceding = finally_await_groups
                        .iter()
                        .take_while(|candidate| candidate.outer_index != outer_index)
                        .map(|candidate| candidate.sites.len())
                        .sum::<usize>();
                    let finally_state = await_sites.len()
                        + catch_await_groups
                            .iter()
                            .map(|group| group.sites.len())
                            .sum::<usize>()
                        + preceding
                        + 1;
                    self.emit_async_begin_finally(
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        finally_state as u64,
                        finally_group.ok_or_else(|| {
                            "internal: async switch finally completion has no await group"
                                .to_string()
                        })?,
                        None,
                    )?;
                    if let (Some(catch_group), Some(throw_entry)) = (catch_group, throw_entry) {
                        self.builder.position_at_end(throw_entry);
                        let message = self
                            .builder
                            .build_load(
                                self.string_abi_type(),
                                throw_slot,
                                "async.switch.throw.message",
                            )
                            .unwrap()
                            .into_struct_value();
                        let (error_slot, _) = self
                            .async_frame_slot(&catch_group.storage_name)
                            .ok_or_else(|| {
                                "internal: async switch catch error has no frame slot".to_string()
                            })?;
                        self.push_scope();
                        let resolved_catch_type = self.resolve_type(&catch_group.catch_type);
                        let catch_message: BasicValueEnum<'ctx> = match &resolved_catch_type {
                            Type::Str => message.into(),
                            Type::Named(name) if name == "Error" => self
                                .builder
                                .build_insert_value(
                                    self.llvm_type(&resolved_catch_type)
                                        .into_struct_type()
                                        .const_zero(),
                                    message,
                                    0,
                                    "async.switch.catch.error",
                                )
                                .map_err(|error| {
                                    format!(
                                        "error: failed to construct async switch `Error` binding: {error}"
                                    )
                                })?
                                .into_struct_value()
                                .into(),
                            _ => {
                                return Err(format!(
                                    "error: async switch catch binding `{}` cannot preserve a synchronous throw across suspension",
                                    catch_group.catch_binding
                                ));
                            }
                        };
                        let catch_message = if matches!(&resolved_catch_type, Type::Named(name) if name == "Error")
                        {
                            self.builder
                                .build_insert_value(
                                    catch_message.into_struct_value(),
                                    self.const_string_abi(
                                        "ERR_RUNTIME",
                                        "async.switch.catch.error.code",
                                    ),
                                    1,
                                    "async.switch.catch.error.code",
                                )
                                .map_err(|error| error.to_string())?
                                .into_struct_value()
                                .into()
                        } else {
                            catch_message
                        };
                        self.builder.build_store(error_slot, catch_message).unwrap();
                        self.mark_async_frame_value_initialized(&catch_group.storage_name, true);
                        self.store_var(&catch_group.catch_binding, error_slot, resolved_catch_type);
                        self.builder
                            .build_store(throw_slot, self.null_string())
                            .unwrap();
                        self.mark_async_frame_value_initialized(&completion.throw_storage, false);
                        self.builder
                            .build_store(mode_slot, self.context.i32_type().const_zero())
                            .unwrap();
                        for statement in &catch_group.sites[0].before {
                            if self.current_block_terminated() {
                                break;
                            }
                            self.emit_stmt(statement, poll)?;
                        }
                        self.pop_scope();
                        if !self.current_block_terminated() {
                            let preceding_catch_sites = catch_await_groups
                                .iter()
                                .take_while(|candidate| {
                                    candidate.storage_name != catch_group.storage_name
                                })
                                .map(|candidate| candidate.sites.len())
                                .sum::<usize>();
                            self.emit_async_suspend_dependency(
                                poll,
                                frame_type,
                                frame,
                                state_pointer,
                                (await_sites.len() + preceding_catch_sites + 1) as u64,
                                &catch_group.sites[0].dependency,
                            )?;
                        }
                    }
                    self.builder.position_at_end(join);
                    return Ok(());
                }
                for statement in after {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(statement, poll)?;
                }
                if !finally_body.is_empty() {
                    self.finally_scopes.pop();
                }
                if !self.current_block_terminated() {
                    if let Some(finally_group) = finally_group {
                        let preceding = finally_await_groups
                            .iter()
                            .take_while(|candidate| {
                                candidate.mode_storage != finally_group.mode_storage
                            })
                            .map(|candidate| candidate.sites.len())
                            .sum::<usize>();
                        let finally_state = await_sites.len()
                            + catch_await_groups
                                .iter()
                                .map(|group| group.sites.len())
                                .sum::<usize>()
                            + preceding
                            + 1;
                        self.emit_async_begin_finally(
                            poll,
                            frame_type,
                            frame,
                            state_pointer,
                            finally_state as u64,
                            finally_group,
                            None,
                        )?;
                    } else {
                        for statement in finally_body {
                            self.emit_stmt(statement, poll)?;
                        }
                        if !self.current_block_terminated() {
                            self.builder.build_unconditional_branch(join).unwrap();
                        }
                    }
                }
                self.builder.position_at_end(join);
            }
            _ => {
                return Err("internal: nested async switch context block mismatch".to_string());
            }
        }
        Ok(())
    }

    pub(super) fn push_async_switch_context_control(
        &mut self,
        outer_index: usize,
        switch_id: Span,
        blocks: &AsyncBlockRegistry<'ctx>,
    ) -> bool {
        match blocks.switch_context_blocks.get(&switch_id).copied() {
            Some(AsyncSwitchContextBlocks::Recursive) => {
                let target = blocks
                    .recursive_control_blocks
                    .iter()
                    .filter_map(|(&(candidate_outer, depth), block)| {
                        if candidate_outer != outer_index {
                            return None;
                        }
                        let AsyncRecursiveControlBlocks::Loop {
                            continue_target,
                            after,
                            finally_depth,
                            ..
                        } = block
                        else {
                            return None;
                        };
                        Some((depth, *continue_target, *after, *finally_depth))
                    })
                    .max_by_key(|(depth, _, _, _)| *depth);
                if let Some((_, continue_target, after, finally_depth)) = target {
                    self.push_loop_control_with_finally_depth(
                        continue_target,
                        after,
                        finally_depth,
                    );
                    true
                } else {
                    false
                }
            }
            Some(
                AsyncSwitchContextBlocks::While {
                    condition, after, ..
                }
                | AsyncSwitchContextBlocks::DoWhile {
                    condition, after, ..
                },
            ) => {
                self.push_loop_control(condition, after);
                true
            }
            Some(AsyncSwitchContextBlocks::For { update, after, .. }) => {
                self.push_loop_control(update, after);
                true
            }
            Some(AsyncSwitchContextBlocks::ForOf { update, after, .. }) => {
                self.push_loop_control(update, after);
                true
            }
            _ => false,
        }
    }

    pub(super) fn store_async_switch_subject(
        &mut self,
        poll: FunctionValue<'ctx>,
        storage: &str,
        subject_type: &Type,
        subject: BasicValueEnum<'ctx>,
        transfer_ownership: bool,
        release_source: bool,
    ) -> Result<(), String> {
        let (slot, stored_type) = self.async_frame_slot(storage).ok_or_else(|| {
            "internal: suspending switch subject has no async frame slot".to_string()
        })?;
        let stored_type = self.resolve_type(&stored_type);
        if stored_type != self.resolve_type(subject_type) {
            return Err("internal: suspending switch subject frame type mismatch".to_string());
        }
        self.clear_async_frame_local_before_store(poll, storage, slot, &stored_type)?;
        if stored_type.is_copy() || transfer_ownership {
            self.builder.build_store(slot, subject).unwrap();
        } else {
            self.emit_async_clone_value_into(slot, subject, &stored_type)?;
            if release_source {
                self.emit_owned_native_value_free(subject, &stored_type);
            }
        }
        self.store_var(storage, slot, stored_type);
        // Without the flag the frame drop skips this slot, and a task cancelled
        // while a case expression is suspended leaks the subject.
        self.mark_async_frame_slot_initialized(slot, true);
        let _ = poll;
        Ok(())
    }

    pub(super) fn finish_async_switch_subject(
        &mut self,
        poll: FunctionValue<'ctx>,
        storage: &str,
        subject_type: &Type,
    ) -> Result<(), String> {
        if subject_type.is_copy() {
            return Ok(());
        }
        let (slot, _) = self.async_frame_slot(storage).ok_or_else(|| {
            "internal: suspending switch subject has no async frame slot".to_string()
        })?;
        self.emit_async_drop_value_in_place(poll, slot, subject_type)?;
        self.mark_async_frame_slot_uninitialized(slot);
        let empty = self.emit_zero(subject_type)?;
        self.builder.build_store(slot, empty).unwrap();
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn initialize_async_switch_case_blocks(
        &mut self,
        outer_index: usize,
        switch_id: Span,
        cases: &[SwitchCase],
        default: &Option<Vec<Stmt>>,
        default_index: Option<usize>,
        await_sites: &[AsyncAwaitSite],
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_for_of_slots: &HashMap<
            AsyncForOfSlotKey,
            (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
        >,
    ) -> Result<(), String> {
        if blocks.switch_blocks.contains_key(&switch_id) {
            return Ok(());
        }
        let dispatch = self
            .builder
            .get_insert_block()
            .ok_or_else(|| "internal: async switch dispatch has no insertion block".to_string())?;
        let end = self.context.append_basic_block(poll, "async.switch.end");
        let case_blocks = (0..cases.len())
            .map(|index| {
                self.context
                    .append_basic_block(poll, &format!("async.switch.case.{index}"))
            })
            .collect::<Vec<_>>();
        let default_block = default.as_ref().map(|_| {
            self.context
                .append_basic_block(poll, "async.switch.default")
        });
        let unmatched = default_block.unwrap_or(end);
        let mut clauses = cases
            .iter()
            .enumerate()
            .map(|(index, case)| (case_blocks[index], case.body.as_slice()))
            .collect::<Vec<_>>();
        if let (Some(default_body), Some(default_block), Some(default_index)) =
            (default.as_ref(), default_block, default_index)
        {
            clauses.insert(default_index, (default_block, default_body));
        }
        let clause_fallthrough = (0..clauses.len())
            .map(|clause_index| {
                clauses
                    .get(clause_index + 1)
                    .map_or(end, |(block, _)| *block)
            })
            .collect::<Vec<_>>();
        blocks.switch_blocks.insert(
            switch_id,
            AsyncSwitchBlocks {
                end,
                case_blocks,
                unmatched,
                clause_fallthrough: clause_fallthrough.clone(),
            },
        );
        let first_sites = await_sites
            .iter()
            .enumerate()
            .filter_map(|(site_index, site)| match &site.kind {
                AsyncAwaitKind::Switch {
                    await_clause,
                    before,
                    entry_guard,
                    first_in_clause: true,
                    ..
                } if site.outer_index == outer_index => Some((
                    *await_clause,
                    (site_index, site, before.as_slice(), entry_guard.as_ref()),
                )),
                _ => None,
            })
            .collect::<HashMap<_, _>>();
        let context_control =
            self.push_async_switch_context_control(outer_index, switch_id, blocks);
        self.push_break_target(end);
        for (clause_index, (block, body)) in clauses.iter().enumerate() {
            self.builder.position_at_end(*block);
            self.push_scope();
            let (statements, suspension) = if let Some((site_index, site, before, entry_guard)) =
                first_sites.get(&clause_index)
            {
                (*before, Some((*site_index, *site, *entry_guard)))
            } else {
                (*body, None)
            };
            for statement in statements {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(statement, poll)?;
            }
            if !self.current_block_terminated()
                && let Some((site_index, site, entry_guard)) = suspension
            {
                if let Some(entry_guard) = entry_guard {
                    self.emit_async_switch_clause_guard_entry(
                        outer_index,
                        switch_id,
                        clause_index,
                        site_index,
                        &site.dependency,
                        entry_guard,
                        clause_fallthrough[clause_index],
                        blocks,
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        async_for_of_slots,
                    )?;
                } else {
                    self.emit_async_suspend_dependency(
                        poll,
                        frame_type,
                        frame,
                        state_pointer,
                        (site_index + 1) as u64,
                        &site.dependency,
                    )?;
                }
            }
            self.pop_scope();
            if !self.current_block_terminated() {
                self.builder
                    .build_unconditional_branch(clause_fallthrough[clause_index])
                    .unwrap();
            }
        }
        self.pop_break_target();
        if context_control {
            self.pop_loop_control();
        }
        self.builder.position_at_end(dispatch);
        Ok(())
    }

    pub(super) fn store_async_expression_prefixes(
        &mut self,
        prefixes: &[(Expr, Type)],
        async_expression_slots: &HashMap<Span, PointerValue<'ctx>>,
    ) -> Result<(), String> {
        for (prefix, prefix_type) in prefixes {
            if self
                .async_await_values
                .last()
                .is_some_and(|values| values.contains_key(&prefix.span))
            {
                continue;
            }
            let value = self.emit_expr_as(prefix, prefix_type)?;
            let slot = *async_expression_slots
                .get(&prefix.span)
                .ok_or_else(|| "internal: async evaluation prefix has no frame slot".to_string())?;
            self.builder.build_store(slot, value).unwrap();
            self.async_await_values
                .last_mut()
                .unwrap()
                .insert(prefix.span, (slot, prefix_type.clone()));
        }
        Ok(())
    }

    pub(super) fn drop_async_switch_case_expression_temporaries(
        &mut self,
        poll: FunctionValue<'ctx>,
        case_index: usize,
        await_sites: &[AsyncAwaitSite],
        async_expression_slots: &HashMap<Span, PointerValue<'ctx>>,
    ) -> Result<(), String> {
        let mut temporaries = Vec::new();
        for site in await_sites {
            let AsyncAwaitKind::SwitchCaseExpression {
                case_index: candidate,
                span,
                payload_type,
                prefixes,
                ..
            } = &site.kind
            else {
                continue;
            };
            if *candidate != case_index {
                continue;
            }
            temporaries.extend(
                prefixes
                    .iter()
                    .map(|(prefix, ty)| (prefix.span, ty.clone())),
            );
            temporaries.push((*span, payload_type.clone()));
        }
        temporaries.sort_by_key(|(span, _)| span.start);
        temporaries.dedup_by_key(|(span, _)| *span);
        for (span, ty) in temporaries.into_iter().rev() {
            if self
                .async_await_values
                .last()
                .is_none_or(|values| !values.contains_key(&span))
            {
                continue;
            }
            let slot = *async_expression_slots
                .get(&span)
                .ok_or_else(|| "internal: async switch temporary has no frame slot".to_string())?;
            if !ty.is_copy() && !self.current_block_terminated() {
                self.emit_async_drop_value_in_place(poll, slot, &ty)?;
                self.mark_async_frame_slot_uninitialized(slot);
                self.builder
                    .build_store(slot, self.llvm_type(&ty).const_zero())
                    .unwrap();
            }
            self.async_await_values.last_mut().unwrap().remove(&span);
        }
        Ok(())
    }

    pub(super) fn bind_async_switch_previous_locals(&mut self, previous: &[Stmt]) {
        for statement in previous {
            let StmtKind::VarDecl { name, .. } = statement.kind() else {
                continue;
            };
            if let Some((slot, ty)) = self.async_frame_slot(name) {
                self.store_var(name, slot, ty);
            }
        }
    }

    pub(super) fn bind_async_switch_guard_for_of(
        &mut self,
        guard: Option<&AsyncSwitchClauseGuard>,
    ) -> Result<(), String> {
        let Some(for_of) = guard.and_then(|guard| guard.for_of.as_ref()) else {
            return Ok(());
        };
        let iterable_type = self
            .expr_type_hint(&for_of.iterable)
            .map(|ty| self.resolve_type(&ty))
            .ok_or_else(|| {
                "internal: cannot infer resumed nested switch for...of iterable type".to_string()
            })?;
        let Type::Array(element_type) = transparent_codegen_type(&iterable_type) else {
            return Err(
                "internal: resumed nested switch for...of iterable is not an array".to_string(),
            );
        };
        let element_type = self.resolve_type(element_type);
        let (binding_slot, _) = self.async_frame_slot(&for_of.name).ok_or_else(|| {
            "internal: resumed nested switch for...of binding has no frame slot".to_string()
        })?;
        self.store_var(&for_of.name, binding_slot, element_type);
        Ok(())
    }

    pub(crate) fn async_object_fields(&self, ty: &Type) -> Option<Vec<Field>> {
        match transparent_codegen_type(ty) {
            Type::Named(name) => self.interfaces.get(name).cloned(),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        }
    }

    pub(super) fn is_copy_for_async(&self, ty: &Type) -> bool {
        ty.is_copy()
            || matches!(transparent_codegen_type(ty), Type::Named(name) if self.enums.contains_key(name))
    }

    pub(crate) fn async_owned_value_supported(&self, ty: &Type) -> bool {
        let ty = self.resolve_type(ty);
        match transparent_codegen_type(&ty) {
            Type::Void => true,
            Type::Named(name) if name == "DigestStream" => false,
            Type::Named(name) if name == "TcpStream" => true,
            Type::Named(name) if matches!(name.as_str(), "TcpListener" | "UdpSocket") => true,
            Type::Named(name) if name == "Error" => true,
            Type::Named(name) if name == "ObjectId" => true,
            Type::Named(name) if matches!(name.as_str(), "Semaphore" | "SemaphorePermit") => true,
            Type::Named(name)
                if matches!(
                    name.as_str(),
                    "WebSocket" | "HttpClientResponse" | "HttpClientStreamResponse"
                ) =>
            {
                true
            }
            Type::Apply { name, args }
                if matches!(
                    name.as_str(),
                    "Sender"
                        | "Receiver"
                        | "Mutex"
                        | "MutexGuard"
                        | "RwLock"
                        | "RwReadGuard"
                        | "RwWriteGuard"
                        | "ReadableStream"
                        | "WritableStream"
                ) =>
            {
                args.len() == 1
            }
            Type::Apply { name, args }
                if matches!(name.as_str(), "DuplexStream" | "TransformStream") =>
            {
                args.len() == 2
            }
            Type::Lit(_) => true,
            Type::Str => true,
            Type::Array(element) => {
                self.is_copy_for_async(element) || self.async_owned_value_supported(element)
            }
            Type::Optional(inner) => {
                self.is_copy_for_async(inner) || self.async_owned_value_supported(inner)
            }
            Type::Result { success, error } => {
                (matches!(success.as_ref(), Type::Void)
                    || self.is_copy_for_async(success)
                    || self.async_owned_value_supported(success))
                    && (self.is_copy_for_async(error) || self.async_owned_value_supported(error))
            }
            Type::Union(variants) if Self::union_is_payload_free(variants) => false,
            Type::Union(variants) => variants.iter().all(|variant| {
                self.is_copy_for_async(variant) || self.async_owned_value_supported(variant)
            }),
            Type::Tuple(elements) => elements.iter().all(|element| {
                self.is_copy_for_async(element) || self.async_owned_value_supported(element)
            }),
            Type::Named(_) | Type::Object(_) => self.async_frame_object(&ty),
            _ => false,
        }
    }

    pub(super) fn async_owned_local_expression(&self, ty: &Type, expression: &Expr) -> bool {
        if matches!(transparent_codegen_type(ty), Type::Named(name) if name == "DigestStream")
            && matches!(expression.kind(), ExprKind::MethodCall { .. })
        {
            return true;
        }
        if !self.async_owned_value_supported(ty) {
            return false;
        }
        if tyrus_core::ast::is_borrowed_place(expression) {
            // The declaration stores a clone; the storage read from keeps its own.
            return true;
        }
        match (transparent_codegen_type(ty), expression.kind()) {
            (_, ExprKind::FnCall { .. }) if self.owned_user_function_call(expression) => true,
            (Type::Array(element), ExprKind::ArrayLit(values)) if !element.is_copy() => values
                .iter()
                .all(|value| !matches!(value.kind(), ExprKind::Spread(_))),
            (Type::Array(_), ExprKind::ArrayRepeat { .. }) => true,
            (Type::Optional(_), _) => true,
            (Type::Named(_) | Type::Object(_), ExprKind::ObjectLit(_)) => true,
            _ => false,
        }
    }

    pub(super) fn async_payload_supported(&self, ty: &Type) -> bool {
        let ty = self.resolve_type(ty);
        if matches!(transparent_codegen_type(&ty), Type::Void | Type::Str) || ty.is_copy() {
            return true;
        }
        match transparent_codegen_type(&ty) {
            Type::Array(_)
            | Type::Optional(_)
            | Type::Result { .. }
            | Type::Union(_)
            | Type::Named(_)
            | Type::Object(_) => self.async_owned_value_supported(&ty),
            _ => false,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn async_pending_finally<'a>(
        &self,
        outer_index: usize,
        try_id: Option<Span>,
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &'a [AsyncFinallyAwaitGroup],
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
    ) -> Option<AsyncPendingFinally<'a, 'ctx>> {
        let group = finally_await_groups.iter().find(|group| {
            group.recursive_try_id == try_id
                && (try_id.is_some() || group.outer_index == outer_index)
        })?;
        let preceding = finally_await_groups
            .iter()
            .take_while(|candidate| candidate.mode_storage != group.mode_storage)
            .map(|candidate| candidate.sites.len())
            .sum::<usize>();
        let state = await_sites.len()
            + catch_await_groups
                .iter()
                .map(|candidate| candidate.sites.len())
                .sum::<usize>()
            + preceding
            + 1;
        Some(AsyncPendingFinally {
            group,
            state: state as u64,
            poll,
            frame_type,
            frame,
            state_pointer,
        })
    }

    pub(crate) fn transfer_active_async_catch_binding(&mut self, expr: &Expr) {
        let ExprKind::Ident(name) = expr.kind() else {
            return;
        };
        let slot = {
            let Some(scope) = self.async_catch_cleanup_scopes.last_mut() else {
                return;
            };
            if scope.binding != *name || scope.transferred {
                return;
            }
            scope.transferred = true;
            scope.slot
        };
        self.mark_async_frame_slot_uninitialized(slot);
    }

    pub(crate) fn async_owned_array_removal(&mut self, expression: &Expr) -> bool {
        let ExprKind::MethodCall { object, method, .. } = expression.kind() else {
            return false;
        };
        if !matches!(method.as_str(), "pop" | "shift") {
            return false;
        }
        if !self.expr_type_hint(object).is_some_and(|ty| {
            matches!(
                transparent_codegen_type(&self.resolve_type(&ty)),
                Type::Array(element) if matches!(transparent_codegen_type(element), Type::Str)
            )
        }) {
            return false;
        }
        tyrus_core::ast::place_root_ident(object).is_some_and(|name| {
            self.async_frame_string_array_ownership(name).is_some()
                || self.async_frame_owns_value(name)
        })
    }
}
