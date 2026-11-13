use super::*;

pub(super) fn block_can_end(body: &[Stmt]) -> bool {
    body.iter().any(statement_can_end_loop)
}

pub(super) fn statement_can_end_loop(statement: &Stmt) -> bool {
    match statement.kind() {
        StmtKind::Break(_) | StmtKind::Return(_) | StmtKind::Throw(_) => true,
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            block_can_end(try_body)
                || block_can_end(catch_body)
                || finally_body.as_deref().is_some_and(block_can_end)
        }
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::ForIn { body, .. } => block_can_end(body),
        StmtKind::Switch { cases, default, .. } => {
            cases.iter().any(|case| block_can_end(&case.body))
                || default.as_deref().is_some_and(block_can_end)
        }
        StmtKind::Match { arms, default, .. } => {
            arms.iter().any(|arm| block_can_end(&arm.body))
                || default.as_deref().is_some_and(block_can_end)
        }
        StmtKind::Expr(expression) => expression_can_end_loop(expression),
        StmtKind::VarDecl { value, .. } => value.as_ref().is_some_and(expression_can_end_loop),
        _ => false,
    }
}

pub(super) fn expression_can_end_loop(expression: &Expr) -> bool {
    match expression.kind() {
        ExprKind::If {
            then_block,
            else_block,
            ..
        } => block_can_end(then_block) || else_block.as_deref().is_some_and(block_can_end),
        _ => false,
    }
}

pub(super) fn async_promise_handle(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Apply { name, args } if matches!(name.as_str(), "Promise" | "DbPromise" | "JoinHandle") && args.len() == 1
    )
}

pub(super) fn async_http_reply_handle(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Apply { name, args } if name == "HttpReply" && args.len() == 1
    )
}

pub(super) fn async_copy_array(ty: &Type) -> bool {
    matches!(ty, Type::Array(element) if element.is_copy())
}

pub(super) fn async_string_array(ty: &Type) -> bool {
    matches!(ty, Type::Array(element) if matches!(element.as_ref(), Type::Str))
}

pub(super) fn async_statement_await_is_lowerable(statement: &Stmt) -> bool {
    if async_recursive_control_statement_is_lowerable(statement) {
        return true;
    }
    match statement.kind() {
        StmtKind::While { cond, body } => {
            matches!(cond.kind(), ExprKind::Await(dependency) if first_await_in_expr(dependency).is_none())
                && first_await_in_block(body).is_none()
                || first_await_in_expr(cond).is_none()
                    && (async_direct_void_await_block_is_lowerable(body)
                        || async_block_with_one_switch_is_lowerable(body)
                        || async_if_in_loop_body_is_lowerable(body))
        }
        StmtKind::DoWhile { body, cond } => {
            matches!(cond.kind(), ExprKind::Await(dependency) if first_await_in_expr(dependency).is_none())
                && first_await_in_block(body).is_none()
                || first_await_in_expr(cond).is_none()
                    && (async_direct_void_await_block_is_lowerable(body)
                        || async_block_with_one_switch_is_lowerable(body)
                        || async_if_in_loop_body_is_lowerable(body))
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            first_await_in_statement(init).is_none()
                && matches!(cond.kind(), ExprKind::Await(dependency) if first_await_in_expr(dependency).is_none())
                && first_await_in_statement(update).is_none()
                && first_await_in_block(body).is_none()
                || first_await_in_statement(init).is_none()
                    && first_await_in_expr(cond).is_none()
                    && first_await_in_statement(update).is_none()
                    && (async_direct_void_await_block_is_lowerable(body)
                        || async_block_with_one_switch_is_lowerable(body)
                        || async_if_in_loop_body_is_lowerable(body))
        }
        StmtKind::ForOf { iterable, body, .. } => {
            first_await_in_expr(iterable).is_none()
                && (async_block_with_one_switch_is_lowerable(body)
                    || async_if_in_loop_body_is_lowerable(body))
        }
        StmtKind::Expr(expression) => match expression.kind() {
            ExprKind::Await(dependency) => first_await_in_expr(dependency).is_none(),
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                first_await_in_expr(cond).is_none()
                    && (async_if_branch_await_is_lowerable(then_block, else_block.as_deref())
                        || async_if_both_branches_is_lowerable(then_block, else_block.as_deref())
                        || async_if_branch_switch_is_lowerable(then_block, else_block.as_deref()))
            }
            _ => first_await_in_expr(expression).is_none(),
        },
        StmtKind::VarDecl {
            value: Some(value), ..
        } => match value.kind() {
            ExprKind::Await(dependency) => first_await_in_expr(dependency).is_none(),
            _ => first_await_in_expr(value).is_none(),
        },
        StmtKind::Assign {
            op: AssignOp::Assign,
            value,
            ..
        } => match value.kind() {
            ExprKind::Await(dependency) => first_await_in_expr(dependency).is_none(),
            _ => first_await_in_expr(value).is_none(),
        },
        StmtKind::Return(Some(value)) => match value.kind() {
            ExprKind::Await(dependency) => first_await_in_expr(dependency).is_none(),
            _ => first_await_in_expr(value).is_none(),
        },
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            let try_has_await = first_await_in_block(try_body).is_some();
            let switch_try = async_block_with_one_switch_is_lowerable(try_body)
                && (first_await_in_block(catch_body).is_none()
                    || async_direct_void_await_block_is_lowerable(catch_body))
                && finally_body.as_deref().is_none_or(|body| {
                    first_await_in_block(body).is_none()
                        || async_direct_void_await_block_is_lowerable(body)
                });
            (!try_has_await
                || async_direct_void_await_block_is_lowerable(try_body)
                || async_try_body_single_if_await_is_lowerable(try_body)
                || switch_try)
                && (catch_body
                    .iter()
                    .all(|statement| first_await_in_statement(statement).is_none())
                    || async_direct_void_await_block_is_lowerable(catch_body))
                && finally_body.as_ref().is_none_or(|body| {
                    body.iter()
                        .all(|statement| first_await_in_statement(statement).is_none())
                        || async_direct_void_await_block_is_lowerable(body)
                })
        }
        StmtKind::Switch { .. } => {
            async_switch_await_is_lowerable(statement)
                || first_await_in_statement(statement).is_none()
        }
        _ => first_await_in_statement(statement).is_none(),
    }
}

pub(super) fn async_recursive_control_block_is_lowerable(body: &[Stmt]) -> bool {
    if async_direct_void_await_block_is_lowerable(body) {
        return true;
    }
    let nested = body
        .iter()
        .filter(|statement| first_await_in_statement(statement).is_some())
        .collect::<Vec<_>>();
    !nested.is_empty()
        && nested
            .iter()
            .all(|statement| async_recursive_control_statement_is_lowerable(statement))
}

pub(super) fn block_awaits_inside_a_loop(body: &[Stmt]) -> bool {
    body.iter().any(statement_awaits_inside_a_loop)
}

pub(super) fn statement_awaits_inside_a_loop(statement: &Stmt) -> bool {
    match statement.kind() {
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::ForOf { body, .. } => first_await_in_block(body).is_some(),
        StmtKind::Expr(expression) => match expression.kind() {
            ExprKind::If {
                then_block,
                else_block,
                ..
            } => {
                block_awaits_inside_a_loop(then_block)
                    || else_block
                        .as_deref()
                        .is_some_and(block_awaits_inside_a_loop)
            }
            _ => false,
        },
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            block_awaits_inside_a_loop(try_body)
                || block_awaits_inside_a_loop(catch_body)
                || finally_body
                    .as_deref()
                    .is_some_and(block_awaits_inside_a_loop)
        }
        _ => false,
    }
}

pub(super) fn async_recursive_control_statement_is_lowerable(statement: &Stmt) -> bool {
    if async_direct_void_await_block_is_lowerable(std::slice::from_ref(statement)) {
        return true;
    }
    match statement.kind() {
        StmtKind::Expr(expression) => {
            let ExprKind::If {
                cond,
                then_block,
                else_block,
            } = expression.kind()
            else {
                return false;
            };
            if first_await_in_expr(cond).is_some() {
                return false;
            }
            let else_body = else_block.as_deref().unwrap_or_default();
            let then_suspends = first_await_in_block(then_block).is_some();
            let else_suspends = first_await_in_block(else_body).is_some();
            if then_suspends && else_suspends {
                return async_direct_void_await_block_is_lowerable(then_block)
                    && async_direct_void_await_block_is_lowerable(else_body);
            }
            if !then_suspends && !else_suspends {
                return false;
            }
            async_recursive_control_block_is_lowerable(if then_suspends {
                then_block
            } else {
                else_body
            })
        }
        StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
            first_await_in_expr(cond).is_none() && async_recursive_control_block_is_lowerable(body)
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => {
            first_await_in_statement(init).is_none()
                && first_await_in_expr(cond).is_none()
                && first_await_in_statement(update).is_none()
                && async_recursive_control_block_is_lowerable(body)
        }
        StmtKind::ForOf { iterable, body, .. } => {
            first_await_in_expr(iterable).is_none()
                && async_recursive_control_block_is_lowerable(body)
        }
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            let suspending_finally = first_await_in_block(try_body).is_none()
                && first_await_in_block(catch_body).is_none()
                && finally_body
                    .as_deref()
                    .is_some_and(async_direct_void_await_block_is_lowerable);
            suspending_finally
                || !block_awaits_inside_a_loop(try_body)
                    && async_recursive_control_block_is_lowerable(try_body)
                    && (first_await_in_block(catch_body).is_none()
                        || async_direct_void_await_block_is_lowerable(catch_body))
                    && finally_body.as_deref().is_none_or(|body| {
                        first_await_in_block(body).is_none()
                            || async_direct_void_await_block_is_lowerable(body)
                    })
        }
        StmtKind::Switch { .. } => async_switch_await_is_lowerable(statement),
        _ => false,
    }
}

pub(super) fn async_block_with_one_switch_is_lowerable(body: &[Stmt]) -> bool {
    let mut switches = 0usize;
    for statement in body {
        if matches!(statement.kind(), StmtKind::Switch { .. })
            && first_await_in_statement(statement).is_some()
        {
            if !async_switch_await_is_lowerable(statement) {
                return false;
            }
            switches += 1;
        } else if first_await_in_statement(statement).is_some() {
            return false;
        }
    }
    switches == 1
}

pub(super) fn async_if_in_loop_body_is_lowerable(body: &[Stmt]) -> bool {
    let mut found = false;
    for statement in body {
        if first_await_in_statement(statement).is_none() {
            continue;
        }
        if found {
            return false;
        }
        let StmtKind::Expr(expression) = statement.kind() else {
            return false;
        };
        let ExprKind::If {
            cond,
            then_block,
            else_block,
        } = expression.kind()
        else {
            return false;
        };
        if first_await_in_expr(cond).is_some()
            || !async_if_branch_await_is_lowerable(then_block, else_block.as_deref())
        {
            return false;
        }
        found = true;
    }
    found
}

pub(super) fn async_if_branch_switch_is_lowerable(
    then_block: &[Stmt],
    else_block: Option<&[Stmt]>,
) -> bool {
    let then_switch = async_block_with_one_switch_is_lowerable(then_block);
    let else_body = else_block.unwrap_or_default();
    let else_switch = async_block_with_one_switch_is_lowerable(else_body);
    (then_switch && first_await_in_block(else_body).is_none())
        || (else_switch && first_await_in_block(then_block).is_none())
}

pub(super) fn async_switch_await_is_lowerable(statement: &Stmt) -> bool {
    let StmtKind::Switch {
        expr,
        cases,
        default,
        ..
    } = statement.kind()
    else {
        return false;
    };
    let subject_await = matches!(
        expr.kind(),
        ExprKind::Await(dependency) if first_await_in_expr(dependency).is_none()
    );
    if !subject_await && first_await_in_expr(expr).is_some() {
        return false;
    }
    let mut case_await = false;
    for case in cases {
        if first_await_in_expr(&case.value).is_some() {
            case_await = true;
        }
    }
    let bodies = cases
        .iter()
        .map(|case| case.body.as_slice())
        .chain(default.as_deref());
    let mut has_direct_await = subject_await || case_await;
    for body in bodies {
        if first_await_in_block(body).is_some() {
            if !async_direct_void_await_block_is_lowerable(body)
                && !async_nested_if_in_switch_clause_is_lowerable(body)
                && !async_nested_while_in_switch_clause_is_lowerable(body)
                && !async_nested_do_while_in_switch_clause_is_lowerable(body)
                && !async_nested_for_in_switch_clause_is_lowerable(body)
                && !async_nested_for_of_in_switch_clause_is_lowerable(body)
            {
                return false;
            }
            has_direct_await = true;
        }
    }
    has_direct_await
}

pub(super) fn async_nested_for_of_in_switch_clause_is_lowerable(body: &[Stmt]) -> bool {
    let mut nested_loop = false;
    for statement in body {
        if let StmtKind::ForOf {
            iterable,
            body: loop_body,
            ..
        } = statement.kind()
        {
            if first_await_in_block(loop_body).is_some() {
                if nested_loop
                    || !matches!(iterable.kind(), ExprKind::Ident(_))
                    || first_await_in_expr(iterable).is_some()
                    || !async_direct_void_await_block_is_lowerable(loop_body)
                {
                    return false;
                }
                nested_loop = true;
            }
        } else if first_await_in_statement(statement).is_some() {
            return false;
        }
    }
    nested_loop
}

pub(super) fn async_nested_for_in_switch_clause_is_lowerable(body: &[Stmt]) -> bool {
    let mut nested_loop = false;
    for statement in body {
        if let StmtKind::For {
            init,
            cond,
            update,
            body: loop_body,
        } = statement.kind()
        {
            if first_await_in_block(loop_body).is_some() {
                if nested_loop
                    || first_await_in_statement(init).is_some()
                    || first_await_in_expr(cond).is_some()
                    || first_await_in_statement(update).is_some()
                    || !async_direct_void_await_block_is_lowerable(loop_body)
                {
                    return false;
                }
                nested_loop = true;
            }
        } else if first_await_in_statement(statement).is_some() {
            return false;
        }
    }
    nested_loop
}

pub(super) fn async_nested_do_while_in_switch_clause_is_lowerable(body: &[Stmt]) -> bool {
    let mut nested_loop = false;
    for statement in body {
        if let StmtKind::DoWhile {
            body: loop_body,
            cond,
        } = statement.kind()
        {
            if first_await_in_block(loop_body).is_some() {
                if nested_loop
                    || first_await_in_expr(cond).is_some()
                    || !async_direct_void_await_block_is_lowerable(loop_body)
                {
                    return false;
                }
                nested_loop = true;
            }
        } else if first_await_in_statement(statement).is_some() {
            return false;
        }
    }
    nested_loop
}

pub(super) fn async_nested_while_in_switch_clause_is_lowerable(body: &[Stmt]) -> bool {
    let mut nested_loop = false;
    for statement in body {
        if let StmtKind::While {
            cond,
            body: loop_body,
        } = statement.kind()
        {
            if first_await_in_block(loop_body).is_some() {
                if nested_loop
                    || first_await_in_expr(cond).is_some()
                    || !async_direct_void_await_block_is_lowerable(loop_body)
                {
                    return false;
                }
                nested_loop = true;
            }
        } else if first_await_in_statement(statement).is_some() {
            return false;
        }
    }
    nested_loop
}

pub(super) fn async_nested_if_in_switch_clause_is_lowerable(body: &[Stmt]) -> bool {
    let mut nested_if = None;
    for statement in body {
        if let StmtKind::Expr(Expr {
            kind:
                ExprKind::If {
                    then_block,
                    else_block,
                    ..
                },
            ..
        }) = statement.kind()
        {
            if first_await_in_statement(statement).is_some() {
                if nested_if.is_some() {
                    return false;
                }
                nested_if = Some((then_block.as_slice(), else_block.as_deref()));
            }
        } else if first_await_in_statement(statement).is_some() {
            return false;
        }
    }
    nested_if.is_some_and(|(then_block, else_block)| {
        async_if_branch_await_is_lowerable(then_block, else_block)
            || async_switch_clause_both_branches_are_direct_void_awaits(
                then_block,
                else_block.unwrap_or_default(),
            )
    })
}

pub(super) fn async_switch_clause_both_branches_are_direct_void_awaits(
    then_block: &[Stmt],
    else_block: &[Stmt],
) -> bool {
    [then_block, else_block].into_iter().all(|branch| {
        matches!(
            branch,
            [Stmt {
                kind: StmtKind::Expr(Expr {
                    kind: ExprKind::Await(dependency),
                    ..
                }),
                ..
            }] if first_await_in_expr(dependency).is_none()
        )
    })
}

pub(super) fn async_if_branch_await_is_lowerable(
    then_block: &[Stmt],
    else_block: Option<&[Stmt]>,
) -> bool {
    let bodies = [then_block, else_block.unwrap_or_default()];
    let mut body_awaits = [0usize; 2];
    for (body_index, body) in bodies.into_iter().enumerate() {
        for statement in body {
            match statement.kind() {
                StmtKind::Expr(expression) => match expression.kind() {
                    ExprKind::Await(dependency) if first_await_in_expr(dependency).is_none() => {
                        body_awaits[body_index] += 1;
                    }
                    _ if first_await_in_statement(statement).is_some() => return false,
                    _ => {}
                },
                StmtKind::VarDecl {
                    value: Some(value), ..
                } => match value.kind() {
                    ExprKind::Await(dependency) if first_await_in_expr(dependency).is_none() => {
                        body_awaits[body_index] += 1;
                    }
                    _ if first_await_in_statement(statement).is_some() => return false,
                    _ => {}
                },
                StmtKind::Assign {
                    op: AssignOp::Assign,
                    value,
                    ..
                } => match value.kind() {
                    ExprKind::Await(dependency) if first_await_in_expr(dependency).is_none() => {
                        body_awaits[body_index] += 1;
                    }
                    _ if first_await_in_statement(statement).is_some() => return false,
                    _ => {}
                },
                _ if first_await_in_statement(statement).is_some() => return false,
                _ => {}
            }
        }
    }
    (body_awaits[0] > 0) != (body_awaits[1] > 0)
}

pub(super) fn async_loop_guard_before_await(statement: &Stmt) -> Option<Span> {
    let cond = match statement.kind() {
        StmtKind::While { cond, .. }
        | StmtKind::DoWhile { cond, .. }
        | StmtKind::For { cond, .. } => cond,
        _ => return None,
    };
    let ExprKind::BinOp {
        op: BinOp::And,
        left,
        right,
    } = cond.kind()
    else {
        return None;
    };
    if first_await_in_expr(left).is_some() {
        return None;
    }
    first_await_in_expr(right)
}

pub(super) fn async_if_both_branches_is_lowerable(
    then_block: &[Stmt],
    else_block: Option<&[Stmt]>,
) -> bool {
    let Some(else_block) = else_block else {
        return false;
    };
    async_direct_void_await_block_is_lowerable(then_block)
        && async_direct_void_await_block_is_lowerable(else_block)
}

pub(super) fn async_try_body_single_if_await_is_lowerable(try_body: &[Stmt]) -> bool {
    let [only] = try_body else {
        return false;
    };
    let StmtKind::Expr(expression) = only.kind() else {
        return false;
    };
    let ExprKind::If {
        cond,
        then_block,
        else_block,
    } = expression.kind()
    else {
        return false;
    };
    first_await_in_expr(cond).is_none()
        && (async_if_branch_await_is_lowerable(then_block, else_block.as_deref())
            || async_if_both_branches_is_lowerable(then_block, else_block.as_deref()))
}

pub(super) fn async_direct_void_await_block_is_lowerable(body: &[Stmt]) -> bool {
    let mut direct_awaits = 0usize;
    for statement in body {
        match statement.kind() {
            StmtKind::Expr(expression) => match expression.kind() {
                ExprKind::Await(dependency) if first_await_in_expr(dependency).is_none() => {
                    direct_awaits += 1;
                }
                _ if first_await_in_statement(statement).is_some() => return false,
                _ => {}
            },
            StmtKind::VarDecl {
                value: Some(value), ..
            } => match value.kind() {
                ExprKind::Await(dependency) if first_await_in_expr(dependency).is_none() => {
                    direct_awaits += 1;
                }
                _ if first_await_in_statement(statement).is_some() => return false,
                _ => {}
            },
            StmtKind::Assign {
                op: AssignOp::Assign,
                value,
                ..
            } => match value.kind() {
                ExprKind::Await(dependency) if first_await_in_expr(dependency).is_none() => {
                    direct_awaits += 1;
                }
                _ if first_await_in_statement(statement).is_some() => return false,
                _ => {}
            },
            _ if first_await_in_statement(statement).is_some() => return false,
            _ => {}
        }
    }
    direct_awaits >= 1
}

pub(super) fn first_await_in_statement(statement: &Stmt) -> Option<Span> {
    match statement.kind() {
        StmtKind::VarDecl { value, .. } => value.as_ref().and_then(first_await_in_expr),
        StmtKind::ArrayDestructure { value, .. }
        | StmtKind::ObjectDestructure { value, .. }
        | StmtKind::Throw(value)
        | StmtKind::Expr(value) => first_await_in_expr(value),
        StmtKind::Assign { value, .. } => first_await_in_expr(value),
        StmtKind::IndexAssign { index, value, .. } => {
            first_await_in_expr(index).or_else(|| first_await_in_expr(value))
        }
        StmtKind::FieldAssign { object, value, .. } => {
            first_await_in_expr(object).or_else(|| first_await_in_expr(value))
        }
        StmtKind::Return(value) => value.as_ref().and_then(first_await_in_expr),
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => first_await_in_block(try_body)
            .or_else(|| first_await_in_block(catch_body))
            .or_else(|| finally_body.as_deref().and_then(first_await_in_block)),
        StmtKind::While { cond, body } => {
            first_await_in_expr(cond).or_else(|| first_await_in_block(body))
        }
        StmtKind::DoWhile { body, cond } => {
            first_await_in_block(body).or_else(|| first_await_in_expr(cond))
        }
        StmtKind::For {
            init,
            cond,
            update,
            body,
        } => first_await_in_statement(init)
            .or_else(|| first_await_in_expr(cond))
            .or_else(|| first_await_in_statement(update))
            .or_else(|| first_await_in_block(body)),
        StmtKind::ForOf { iterable, body, .. } => {
            first_await_in_expr(iterable).or_else(|| first_await_in_block(body))
        }
        StmtKind::ForIn { object, body, .. } => {
            first_await_in_expr(object).or_else(|| first_await_in_block(body))
        }
        StmtKind::Switch {
            expr,
            cases,
            default,
            ..
        } => first_await_in_expr(expr)
            .or_else(|| {
                cases.iter().find_map(|case| {
                    first_await_in_expr(&case.value).or_else(|| first_await_in_block(&case.body))
                })
            })
            .or_else(|| default.as_deref().and_then(first_await_in_block)),
        StmtKind::Match {
            expr,
            arms,
            default,
        } => first_await_in_expr(expr)
            .or_else(|| {
                arms.iter().find_map(|arm| {
                    arm.guard
                        .as_ref()
                        .and_then(first_await_in_expr)
                        .or_else(|| first_await_in_block(&arm.body))
                })
            })
            .or_else(|| default.as_deref().and_then(first_await_in_block)),
        StmtKind::Break(value) => value.as_ref().and_then(first_await_in_expr),
        StmtKind::Defer { body, .. } => first_await_in_block(body),
        StmtKind::Continue => None,
    }
}

/// An `onfail` handler is emitted in the middle of the statement that binds
/// the value, so it cannot hold a suspension point of its own.
pub(super) fn first_await_in_onfail_handler(statement: &Stmt) -> Option<Span> {
    let in_block = |body: &[Stmt]| body.iter().find_map(first_await_in_onfail_handler);
    match statement.kind() {
        StmtKind::VarDecl {
            onfail: Some(onfail),
            ..
        } => match &onfail.handler {
            OnFailHandler::Expr(handler) => first_await_in_expr(handler),
            OnFailHandler::Block(handler) => first_await_in_block(handler),
        },
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => in_block(try_body)
            .or_else(|| in_block(catch_body))
            .or_else(|| finally_body.as_deref().and_then(in_block)),
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::For { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::ForIn { body, .. }
        | StmtKind::Defer { body, .. } => in_block(body),
        StmtKind::Switch { cases, default, .. } => cases
            .iter()
            .find_map(|case| in_block(&case.body))
            .or_else(|| default.as_deref().and_then(in_block)),
        StmtKind::Match { arms, default, .. } => arms
            .iter()
            .find_map(|arm| in_block(&arm.body))
            .or_else(|| default.as_deref().and_then(in_block)),
        StmtKind::Expr(expression) => match expression.kind() {
            ExprKind::If {
                then_block,
                else_block,
                ..
            } => in_block(then_block).or_else(|| else_block.as_deref().and_then(in_block)),
            ExprKind::Loop { body } => in_block(body),
            _ => None,
        },
        _ => None,
    }
}

pub(super) type TryHandlers = (Vec<Type>, Vec<bool>, Vec<bool>, Vec<bool>);

pub(super) fn first_await_in_block(body: &[Stmt]) -> Option<Span> {
    body.iter().find_map(first_await_in_statement)
}

pub(super) fn first_await_in_expr(expression: &Expr) -> Option<Span> {
    match expression.kind() {
        ExprKind::Await(_) => Some(expression.span),
        ExprKind::CallValue { callee, args, .. } => {
            first_await_in_expr(callee).or_else(|| args.iter().find_map(first_await_in_expr))
        }
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            initial.as_deref().and_then(first_await_in_expr)
        }
        ExprKind::TemplateLit(parts) => parts.iter().find_map(|part| match part {
            crate::ast::TemplatePart::Text(_) => None,
            crate::ast::TemplatePart::Expr(expression, _) => first_await_in_expr(expression),
        }),
        ExprKind::ObjectLit(fields) => fields
            .iter()
            .find_map(|(_, value)| first_await_in_expr(value)),
        ExprKind::ObjectUpdate { base, fields } => first_await_in_expr(base).or_else(|| {
            fields
                .iter()
                .find_map(|(_, value)| first_await_in_expr(value))
        }),
        ExprKind::ArrayLit(values) => values.iter().find_map(first_await_in_expr),
        ExprKind::ArrayRepeat { value, count } => {
            first_await_in_expr(value).or_else(|| first_await_in_expr(count))
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            first_await_in_expr(object)
        }
        ExprKind::IndexAccess { object, index } => {
            first_await_in_expr(object).or_else(|| first_await_in_expr(index))
        }
        ExprKind::BinOp { left, right, .. }
        | ExprKind::Pipeline { left, right }
        | ExprKind::In {
            key: left,
            object: right,
        } => first_await_in_expr(left).or_else(|| first_await_in_expr(right)),
        ExprKind::Range { start, end, .. } => {
            first_await_in_expr(start).or_else(|| first_await_in_expr(end))
        }
        ExprKind::ChainedComparison { terms, .. } => terms.iter().find_map(first_await_in_expr),
        ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::BitNot(inner)
        | ExprKind::PrefixIncr(inner)
        | ExprKind::PrefixDecr(inner)
        | ExprKind::PostfixIncr(inner)
        | ExprKind::PostfixDecr(inner)
        | ExprKind::Unwrap(inner)
        | ExprKind::Typeof(inner)
        | ExprKind::Try(inner)
        | ExprKind::Spread(inner)
        | ExprKind::TypeAssert { value: inner, .. }
        | ExprKind::Is { expr: inner, .. }
        | ExprKind::ConstAssert(inner)
        | ExprKind::Comptime(inner) => first_await_in_expr(inner),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => first_await_in_expr(cond)
            .or_else(|| first_await_in_expr(then_expr))
            .or_else(|| first_await_in_expr(else_expr)),
        ExprKind::OptionalMethodCall { object, args, .. }
        | ExprKind::MethodCall { object, args, .. } => {
            first_await_in_expr(object).or_else(|| args.iter().find_map(first_await_in_expr))
        }
        ExprKind::FnCall { args, .. } => args.iter().find_map(first_await_in_expr),
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => first_await_in_expr(cond)
            .or_else(|| first_await_in_block(then_block))
            .or_else(|| else_block.as_deref().and_then(first_await_in_block)),
        ExprKind::Loop { body } => first_await_in_block(body),
        ExprKind::ArrowFn { .. }
        | ExprKind::StrLit(_)
        | ExprKind::RegexLit { .. }
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::DecimalLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::None
        | ExprKind::Ident(_)
        | ExprKind::Topic
        | ExprKind::Borrow { .. } => None,
    }
}

pub(super) fn first_await_dependency_in_expr(expression: &Expr) -> Option<&Expr> {
    match expression.kind() {
        ExprKind::Await(dependency) => Some(dependency),
        ExprKind::CallValue { callee, args, .. } => first_await_dependency_in_expr(callee)
            .or_else(|| args.iter().find_map(first_await_dependency_in_expr)),
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            initial.as_deref().and_then(first_await_dependency_in_expr)
        }
        ExprKind::TemplateLit(parts) => parts.iter().find_map(|part| match part {
            crate::ast::TemplatePart::Text(_) => None,
            crate::ast::TemplatePart::Expr(expression, _) => {
                first_await_dependency_in_expr(expression)
            }
        }),
        ExprKind::ObjectLit(fields) => fields
            .iter()
            .find_map(|(_, value)| first_await_dependency_in_expr(value)),
        ExprKind::ObjectUpdate { base, fields } => {
            first_await_dependency_in_expr(base).or_else(|| {
                fields
                    .iter()
                    .find_map(|(_, value)| first_await_dependency_in_expr(value))
            })
        }
        ExprKind::ArrayLit(values) => values.iter().find_map(first_await_dependency_in_expr),
        ExprKind::ArrayRepeat { value, count } => {
            first_await_dependency_in_expr(value).or_else(|| first_await_dependency_in_expr(count))
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            first_await_dependency_in_expr(object)
        }
        ExprKind::IndexAccess { object, index } => {
            first_await_dependency_in_expr(object).or_else(|| first_await_dependency_in_expr(index))
        }
        ExprKind::BinOp { left, right, .. }
        | ExprKind::Pipeline { left, right }
        | ExprKind::In {
            key: left,
            object: right,
        } => first_await_dependency_in_expr(left).or_else(|| first_await_dependency_in_expr(right)),
        ExprKind::Range { start, end, .. } => {
            first_await_dependency_in_expr(start).or_else(|| first_await_dependency_in_expr(end))
        }
        ExprKind::ChainedComparison { terms, .. } => {
            terms.iter().find_map(first_await_dependency_in_expr)
        }
        ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::BitNot(inner)
        | ExprKind::PrefixIncr(inner)
        | ExprKind::PrefixDecr(inner)
        | ExprKind::PostfixIncr(inner)
        | ExprKind::PostfixDecr(inner)
        | ExprKind::Unwrap(inner)
        | ExprKind::Typeof(inner)
        | ExprKind::Spread(inner)
        | ExprKind::TypeAssert { value: inner, .. }
        | ExprKind::Is { expr: inner, .. }
        | ExprKind::ConstAssert(inner)
        | ExprKind::Comptime(inner) => first_await_dependency_in_expr(inner),
        ExprKind::Try(inner) => first_await_dependency_in_expr(inner),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => first_await_dependency_in_expr(cond)
            .or_else(|| first_await_dependency_in_expr(then_expr))
            .or_else(|| first_await_dependency_in_expr(else_expr)),
        ExprKind::OptionalMethodCall { object, args, .. }
        | ExprKind::MethodCall { object, args, .. } => first_await_dependency_in_expr(object)
            .or_else(|| args.iter().find_map(first_await_dependency_in_expr)),
        ExprKind::FnCall { args, .. } => args.iter().find_map(first_await_dependency_in_expr),
        ExprKind::If { cond, .. } => first_await_dependency_in_expr(cond),
        ExprKind::Loop { .. }
        | ExprKind::ArrowFn { .. }
        | ExprKind::StrLit(_)
        | ExprKind::RegexLit { .. }
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::DecimalLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::None
        | ExprKind::Ident(_)
        | ExprKind::Topic
        | ExprKind::Borrow { .. } => None,
    }
}

pub(super) fn native_function_type(name: &str) -> Option<Type> {
    match name {
        "__tyrus_fs_readFileSync" => Some(Type::Function {
            params: vec![Type::Str, Type::Str],
            ret: Box::new(Type::Str),
        }),
        "__tyrus_fs_writeFileSync" => Some(Type::Function {
            params: vec![Type::Str, Type::Str, Type::Str],
            ret: Box::new(Type::Void),
        }),
        "__tyrus_fs_readdir" => Some(Type::Function {
            params: vec![
                Type::Str,
                Type::Optional(Box::new(Type::Named("ReaddirOptions".to_string()))),
            ],
            ret: Box::new(Type::Array(Box::new(Type::Named("DirEntry".to_string())))),
        }),
        "__tyrus_fs_mkdir" => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Void),
        }),
        "__tyrus_fs_rm" => Some(Type::Function {
            params: vec![
                Type::Str,
                Type::Optional(Box::new(Type::Named("RmOptions".to_string()))),
            ],
            ret: Box::new(Type::Void),
        }),
        "__tyrus_fs_cp" => Some(Type::Function {
            params: vec![
                Type::Str,
                Type::Str,
                Type::Optional(Box::new(Type::Named("CpOptions".to_string()))),
            ],
            ret: Box::new(Type::Void),
        }),
        "__tyrus_fs_mv" => Some(Type::Function {
            params: vec![Type::Str, Type::Str],
            ret: Box::new(Type::Void),
        }),
        "__tyrus_fs_removeAll" => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Void),
        }),
        "__tyrus_fs_stat" => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Result {
                success: Box::new(Type::Named("FileStats".to_string())),
                error: Box::new(Type::Named("FsError".to_string())),
            }),
        }),
        "__tyrus_fs_metadata" => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("Metadata".to_string())),
        }),
        "__tyrus_fs_disk_usage" => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("DiskUsage".to_string())),
        }),
        "__tyrus_fs_mime" => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Optional(Box::new(Type::Str))),
        }),
        "__tyrus_fs_chmod" => Some(Type::Function {
            params: vec![Type::Str, Type::Named("FileMode".to_string())],
            ret: Box::new(Type::Void),
        }),
        "__tyrus_fs_chown" => Some(Type::Function {
            params: vec![Type::Str, Type::U32, Type::U32],
            ret: Box::new(Type::Void),
        }),
        "__tyrus_fs_readlink" => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Result {
                success: Box::new(Type::Str),
                error: Box::new(Type::Named("FsError".to_string())),
            }),
        }),
        "__tyrus_fs_symlink" => Some(Type::Function {
            params: vec![Type::Str, Type::Str],
            ret: Box::new(Type::Void),
        }),
        "__tyrus_fs_symlink_dir" => Some(Type::Function {
            params: vec![Type::Str, Type::Str],
            ret: Box::new(Type::Void),
        }),
        "__tyrus_fs_hard_link" => Some(Type::Function {
            params: vec![Type::Str, Type::Str],
            ret: Box::new(Type::Void),
        }),
        "__tyrus_fs_canonicalize" => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Str),
        }),
        "__tyrus_fs_glob" => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Result {
                success: Box::new(Type::Array(Box::new(Type::Str))),
                error: Box::new(Type::Named("FsError".to_string())),
            }),
        }),
        "__tyrus_fs_search" => Some(Type::Function {
            params: vec![
                Type::Str,
                Type::Optional(Box::new(Type::Named("SearchOptions".to_string()))),
            ],
            ret: Box::new(Type::Array(Box::new(Type::Str))),
        }),
        "__tyrus_fs_diff" => Some(Type::Function {
            params: vec![Type::Str, Type::Str],
            ret: Box::new(Type::Named("DiffResult".to_string())),
        }),
        "__tyrus_fs_atomic_write" => Some(Type::Function {
            params: vec![Type::Str, Type::Str],
            ret: Box::new(Type::Void),
        }),
        "__tyrus_fs_temp_dir" => Some(Type::Function {
            params: vec![],
            ret: Box::new(Type::Named("TempDir".to_string())),
        }),
        "__tyrus_fs_temp_file" => Some(Type::Function {
            params: vec![],
            ret: Box::new(Type::Str),
        }),
        "__tyrus_fs_watch" => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Named("FSWatcher".to_string())),
        }),
        "__tyrus_http_createRouter" => Some(Type::Function {
            params: Vec::new(),
            ret: Box::new(Type::Named("HttpRouter".to_string())),
        }),
        "__tyrus_algorithm_suffix_array" => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::Array(Box::new(Type::Usize))),
        }),
        "__tyrus_algorithm_suffix_tree_new" => Some(Type::Function {
            params: vec![Type::Str],
            ret: Box::new(Type::SuffixTree),
        }),
        "__tyrus_algorithm_lca_new" => Some(Type::Function {
            params: vec![
                Type::Array(Box::new(Type::Array(Box::new(Type::Usize)))),
                Type::Usize,
            ],
            ret: Box::new(Type::Lca),
        }),
        "__tyrus_algorithm_hld_new" => Some(Type::Function {
            params: vec![
                Type::Array(Box::new(Type::Array(Box::new(Type::Usize)))),
                Type::Usize,
            ],
            ret: Box::new(Type::Hld),
        }),
        "__tyrus_algorithm_centroid_new" => Some(Type::Function {
            params: vec![Type::Array(Box::new(Type::Array(Box::new(Type::Usize))))],
            ret: Box::new(Type::Centroid),
        }),
        "__tyrus_algorithm_bloom_filter_new" => Some(Type::Function {
            params: vec![Type::Usize, Type::Usize],
            ret: Box::new(Type::BloomFilter),
        }),
        _ => None,
    }
}

pub(super) fn native_optional_trailing_param_count(callee: &str) -> usize {
    match callee {
        "fetch" | "__tyrus_http_fetch" => 1,
        "connectWebSocket" | "__tyrus_http_connectWebSocket" => 1,
        "exec" | "__tyrus_process_exec" => 1,
        "cpuTemp" => 1,
        "notify" => 1,
        "approxEqual" => 1,
        "readdir" | "rm" | "cp" | "__tyrus_fs_readdir" | "__tyrus_fs_rm" | "__tyrus_fs_cp" => 1,
        "search" | "__tyrus_fs_search" => 1,
        "tcpConnect" | "tcpListen" | "__tyrus_net_tcpConnect" | "__tyrus_net_tcpListen" => 1,
        "resolve"
        | "reverseLookup"
        | "__tyrus_net_resolve"
        | "__tyrus_net_reverseLookup"
        | "ping"
        | "traceroute"
        | "__tyrus_net_ping"
        | "__tyrus_net_traceroute" => 1,
        _ => 0,
    }
}

pub(super) fn is_supported_http_scalar_type(ty: &Type) -> bool {
    matches!(
        transparent_type(ty),
        Type::Str
            | Type::Bool
            | Type::F32
            | Type::F64
            | Type::Decimal
            | Type::I32
            | Type::I64
            | Type::Isize
            | Type::U32
            | Type::U64
            | Type::Usize
    )
}

pub(super) fn transparent_type(ty: &Type) -> &Type {
    match ty {
        Type::Ref(inner) | Type::MutRef(inner) | Type::Readonly(inner) => transparent_type(inner),
        Type::Throws { function, .. } => transparent_type(function),
        _ => ty,
    }
}

pub(super) fn literal_target_type(ty: &Type) -> &Type {
    match transparent_type(ty) {
        Type::Optional(inner) => transparent_type(inner),
        ty => ty,
    }
}

pub(super) fn promise_payload(ty: &Type) -> Option<&Type> {
    match transparent_type(ty) {
        Type::Apply { name, args }
            if matches!(name.as_str(), "Promise" | "DbPromise") && args.len() == 1 =>
        {
            args.first()
        }
        _ => None,
    }
}

pub(super) fn is_external_thenable(ty: &Type, interfaces: &HashMap<String, Vec<Field>>) -> bool {
    match transparent_type(ty) {
        Type::Object(fields) => fields.iter().any(|field| field.name == "then"),
        Type::Named(name) => interfaces
            .get(name)
            .is_some_and(|fields| fields.iter().any(|field| field.name == "then")),
        _ => false,
    }
}

pub(super) fn promise_of(payload: Type) -> Type {
    Type::Apply {
        name: "Promise".to_string(),
        args: vec![payload],
    }
}

pub(super) fn join_handle_payload(ty: &Type) -> Option<&Type> {
    match transparent_type(ty) {
        Type::Apply { name, args } if name == "JoinHandle" && args.len() == 1 => args.first(),
        _ => None,
    }
}

pub(super) fn sender_payload(ty: &Type) -> Option<&Type> {
    match transparent_type(ty) {
        Type::Apply { name, args } if name == "Sender" && args.len() == 1 => args.first(),
        _ => None,
    }
}

pub(super) fn receiver_payload(ty: &Type) -> Option<&Type> {
    match transparent_type(ty) {
        Type::Apply { name, args } if name == "Receiver" && args.len() == 1 => args.first(),
        _ => None,
    }
}

pub(super) fn readable_stream_payload(ty: &Type) -> Option<&Type> {
    match transparent_type(ty) {
        Type::Apply { name, args } if name == "ReadableStream" && args.len() == 1 => args.first(),
        _ => None,
    }
}

pub(super) fn writable_stream_payload(ty: &Type) -> Option<&Type> {
    match transparent_type(ty) {
        Type::Apply { name, args } if name == "WritableStream" && args.len() == 1 => args.first(),
        _ => None,
    }
}

pub(super) fn mutex_payload(ty: &Type) -> Option<&Type> {
    match transparent_type(ty) {
        Type::Apply { name, args } if name == "Mutex" && args.len() == 1 => args.first(),
        _ => None,
    }
}

pub(super) fn mutex_guard_payload(ty: &Type) -> Option<&Type> {
    match transparent_type(ty) {
        Type::Apply { name, args } if name == "MutexGuard" && args.len() == 1 => args.first(),
        _ => None,
    }
}

pub(crate) fn is_mutex_guard_typed(ty: &Type) -> bool {
    match transparent_type(ty) {
        Type::Optional(inner) => mutex_guard_payload(inner).is_some(),
        other => mutex_guard_payload(other).is_some(),
    }
}

pub(super) fn rwlock_payload(ty: &Type) -> Option<&Type> {
    match transparent_type(ty) {
        Type::Apply { name, args } if name == "RwLock" && args.len() == 1 => args.first(),
        _ => None,
    }
}

pub(super) fn rwlock_read_guard_payload(ty: &Type) -> Option<&Type> {
    match transparent_type(ty) {
        Type::Apply { name, args } if name == "RwReadGuard" && args.len() == 1 => args.first(),
        _ => None,
    }
}

pub(super) fn rwlock_write_guard_payload(ty: &Type) -> Option<&Type> {
    match transparent_type(ty) {
        Type::Apply { name, args } if name == "RwWriteGuard" && args.len() == 1 => args.first(),
        _ => None,
    }
}

pub(crate) fn is_rwlock_read_guard_typed(ty: &Type) -> bool {
    match transparent_type(ty) {
        Type::Optional(inner) => rwlock_read_guard_payload(inner).is_some(),
        other => rwlock_read_guard_payload(other).is_some(),
    }
}

pub(crate) fn is_rwlock_write_guard_typed(ty: &Type) -> bool {
    match transparent_type(ty) {
        Type::Optional(inner) => rwlock_write_guard_payload(inner).is_some(),
        other => rwlock_write_guard_payload(other).is_some(),
    }
}

pub(super) fn is_rwlock_guard_typed(ty: &Type) -> bool {
    is_rwlock_read_guard_typed(ty) || is_rwlock_write_guard_typed(ty)
}

pub(super) fn is_spawn_shaped(
    type_params: &[crate::ast::TypeParam],
    params: &[Type],
    ret: &Type,
) -> bool {
    let [
        crate::ast::TypeParam {
            name: type_param, ..
        },
    ] = type_params
    else {
        return false;
    };
    let [
        Type::Function {
            params: task_params,
            ret: task_ret,
        },
    ] = params
    else {
        return false;
    };
    if !task_params.is_empty() {
        return false;
    }
    let task_returns_promise_of_t = matches!(
        task_ret.as_ref(),
        Type::Apply { name, args }
            if name == "Promise"
                && args.len() == 1
                && matches!(&args[0], Type::Generic(name) if name == type_param)
    );
    let returns_join_handle_of_t = matches!(
        ret,
        Type::Apply { name, args }
            if name == "JoinHandle"
                && args.len() == 1
                && matches!(&args[0], Type::Generic(name) if name == type_param)
    );
    task_returns_promise_of_t && returns_join_handle_of_t
}

pub(super) fn is_channel_shaped(
    type_params: &[crate::ast::TypeParam],
    params: &[Type],
    ret: &Type,
) -> bool {
    let [
        crate::ast::TypeParam {
            name: type_param, ..
        },
    ] = type_params
    else {
        return false;
    };
    let [Type::Optional(capacity)] = params else {
        return false;
    };
    if !matches!(capacity.as_ref(), Type::Usize) {
        return false;
    }
    matches!(
        ret,
        Type::Tuple(elements)
            if elements.len() == 2
                && matches!(
                    &elements[0],
                    Type::Apply { name, args }
                        if name == "Sender" && args.len() == 1
                            && matches!(&args[0], Type::Generic(n) if n == type_param)
                )
                && matches!(
                    &elements[1],
                    Type::Apply { name, args }
                        if name == "Receiver" && args.len() == 1
                            && matches!(&args[0], Type::Generic(n) if n == type_param)
                )
    )
}

pub(super) fn is_mutex_shaped(
    type_params: &[crate::ast::TypeParam],
    params: &[Type],
    ret: &Type,
) -> bool {
    let [
        crate::ast::TypeParam {
            name: type_param, ..
        },
    ] = type_params
    else {
        return false;
    };
    let [Type::Generic(param_name)] = params else {
        return false;
    };
    if param_name != type_param {
        return false;
    }
    matches!(
        ret,
        Type::Apply { name, args }
            if name == "Mutex" && args.len() == 1
                && matches!(&args[0], Type::Generic(n) if n == type_param)
    )
}

pub(super) fn is_supported_mutex_payload(ty: &Type) -> bool {
    is_supported_channel_payload(ty)
}

pub(super) fn is_rwlock_shaped(
    type_params: &[crate::ast::TypeParam],
    params: &[Type],
    ret: &Type,
) -> bool {
    let [
        crate::ast::TypeParam {
            name: type_param, ..
        },
    ] = type_params
    else {
        return false;
    };
    let [Type::Generic(param_name)] = params else {
        return false;
    };
    if param_name != type_param {
        return false;
    }
    matches!(
        ret,
        Type::Apply { name, args }
            if name == "RwLock" && args.len() == 1
                && matches!(&args[0], Type::Generic(n) if n == type_param)
    )
}

pub(super) fn is_supported_rwlock_payload(ty: &Type) -> bool {
    is_supported_channel_payload(ty)
}

pub(super) fn is_join_handle_array_consuming_shaped(
    type_params: &[crate::ast::TypeParam],
    params: &[Type],
) -> bool {
    let [
        crate::ast::TypeParam {
            name: type_param, ..
        },
    ] = type_params
    else {
        return false;
    };
    matches!(
        params,
        [Type::Array(element)]
            if matches!(
                element.as_ref(),
                Type::Apply { name, args }
                    if name == "JoinHandle" && args.len() == 1
                        && matches!(&args[0], Type::Generic(n) if n == type_param)
            )
    )
}

pub(super) fn is_supported_channel_payload(ty: &Type) -> bool {
    match transparent_type(ty) {
        Type::Str => true,
        Type::Array(element) => element.is_copy(),
        ty => ty.is_copy(),
    }
}

pub(super) fn function_return_type(value: ValueType) -> Option<Type> {
    match value {
        ValueType::Known(Type::Function { ret, .. }) => Some(*ret),
        _ => None,
    }
}

pub(super) fn native_field(name: &str, ty: Type) -> Field {
    Field {
        name: name.to_string(),
        ty,
        readonly: true,
        optional: false,
        decorators: Vec::new(),
        span: Span::DUMMY,
    }
}

pub(super) fn promise_settled_result(payload: Type) -> Type {
    Type::Union(vec![
        Type::Object(vec![
            native_field(
                "status",
                Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(
                    "fulfilled".to_string(),
                )))),
            ),
            native_field("value", payload),
        ]),
        Type::Object(vec![
            native_field(
                "status",
                Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(
                    "rejected".to_string(),
                )))),
            ),
            native_field("reason", Type::Named("Error".to_string())),
        ]),
    ])
}

pub(super) struct SequenceReceiver {
    pub(super) element: Type,
    pub(super) readonly: bool,
    pub(super) fixed_length: bool,
}

pub(super) fn tuple_positions(ty: &ValueType) -> Option<&[Type]> {
    let ValueType::Known(known) = ty else {
        return None;
    };
    match known {
        Type::Tuple(elements) => Some(elements),
        Type::Readonly(inner) => match inner.as_ref() {
            Type::Tuple(elements) => Some(elements),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn ordered_tree_element_allowed(element: &Type) -> bool {
    matches!(
        element,
        Type::Bool
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
            | Type::Str
    )
}

pub(super) fn sequence_receiver(ty: &Type) -> Option<SequenceReceiver> {
    let (inner, readonly) = match ty {
        Type::Readonly(inner) => (inner.as_ref(), true),
        other => (other, false),
    };
    match inner {
        Type::Array(element) => Some(SequenceReceiver {
            element: element.as_ref().clone(),
            readonly,
            fixed_length: false,
        }),
        Type::Tuple(elements) => match tuple_sequence(elements) {
            TupleSequence::Uniform { element } => Some(SequenceReceiver {
                element,
                readonly,
                fixed_length: true,
            }),
            TupleSequence::Mixed { .. } | TupleSequence::Empty => None,
        },
        _ => None,
    }
}

pub(super) fn deep_readonly(ty: Type) -> Type {
    match ty {
        Type::Readonly(_) => ty,
        Type::Named(_)
        | Type::Array(_)
        | Type::Tuple(_)
        | Type::Object(_)
        | Type::Map(_, _)
        | Type::Set(_)
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
        | Type::BitSet => Type::Readonly(Box::new(ty)),
        other => other,
    }
}

pub(super) fn object_type_fields_assignable(
    expected_fields: &[crate::ast::Field],
    actual_fields: &[crate::ast::Field],
) -> bool {
    expected_fields.iter().all(|field| {
        actual_fields
            .iter()
            .find(|candidate| candidate.name == field.name)
            .map(|actual| {
                is_assignable(
                    &field_value_type(field),
                    &ValueType::Known(actual.ty.clone()),
                )
            })
            .unwrap_or(field.optional)
    }) && actual_fields.iter().all(|actual| {
        expected_fields
            .iter()
            .any(|field| field.name == actual.name)
    })
}

pub(super) fn is_const_assertion_value(expr: &Expr) -> bool {
    match expr.kind() {
        ExprKind::StrLit(_)
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::DecimalLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::None => true,
        ExprKind::ArrayLit(values) => values.iter().all(|value| match value {
            Expr {
                kind: ExprKind::Spread(inner),
                ..
            } => is_const_assertion_value(inner),
            other => is_const_assertion_value(other),
        }),
        ExprKind::ObjectLit(fields) => fields
            .iter()
            .all(|(_, value)| is_const_assertion_value(value)),
        _ => false,
    }
}

pub fn const_assertion_type(expr: &Expr) -> Type {
    match expr.kind() {
        ExprKind::StrLit(_)
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::DecimalLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::None => Type::Lit(Box::new(expr.clone())),
        ExprKind::ArrayLit(values) => {
            let mut positions = Vec::new();
            for value in values {
                match value.kind() {
                    ExprKind::Spread(inner) => match const_assertion_type(inner) {
                        Type::Readonly(spread) => match *spread {
                            Type::Tuple(mut spread_positions) => {
                                positions.append(&mut spread_positions);
                            }
                            other => positions.push(other),
                        },
                        other => positions.push(other),
                    },
                    _ => positions.push(const_assertion_type(value)),
                }
            }
            Type::Readonly(Box::new(Type::Tuple(positions)))
        }
        ExprKind::ObjectLit(fields) => Type::Object(
            fields
                .iter()
                .map(|(name, value)| crate::ast::Field {
                    name: name.clone(),
                    ty: const_assertion_type(value),
                    readonly: true,
                    optional: false,
                    decorators: Vec::new(),
                    span: Span::DUMMY,
                })
                .collect(),
        ),
        _ => value_type_to_type(&literal_value_type(expr)).unwrap_or(Type::Unknown),
    }
}

pub(super) fn literal_value_type(expr: &Expr) -> ValueType {
    match expr.kind() {
        ExprKind::StrLit(_) => ValueType::Known(Type::Str),
        ExprKind::RegexLit { .. } => ValueType::Known(Type::Regex),
        ExprKind::IntLit(value) if *value < 0 => ValueType::NegativeIntegerLiteral,
        ExprKind::IntLit(_) => ValueType::IntegerLiteral,
        ExprKind::UIntLit(_) => ValueType::UnsignedIntegerLiteral,
        ExprKind::FloatLit(_) => ValueType::FloatLiteral,
        ExprKind::BoolLit(_) => ValueType::Known(Type::Bool),
        ExprKind::Null | ExprKind::None => ValueType::Null,
        _ => ValueType::Unknown,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TupleSequence {
    Uniform { element: Type },
    Mixed { element: Type },
    Empty,
}

impl TupleSequence {
    pub fn element(&self) -> Option<&Type> {
        match self {
            TupleSequence::Uniform { element } | TupleSequence::Mixed { element } => Some(element),
            TupleSequence::Empty => None,
        }
    }

    pub fn is_uniform(&self) -> bool {
        matches!(self, TupleSequence::Uniform { .. })
    }
}

pub fn tuple_sequence(elements: &[Type]) -> TupleSequence {
    let Some(first) = elements.first() else {
        return TupleSequence::Empty;
    };
    let element = elements
        .iter()
        .cloned()
        .reduce(common_const_array_element_type)
        .unwrap_or(Type::Never);
    if elements
        .iter()
        .all(|candidate| tuple_storage_type(candidate) == tuple_storage_type(first))
    {
        TupleSequence::Uniform { element }
    } else {
        TupleSequence::Mixed { element }
    }
}

pub(super) fn tuple_storage_type(ty: &Type) -> Type {
    let base = transparent_type(ty);
    literal_type_underlying(base).unwrap_or_else(|| base.clone())
}

pub(super) fn common_const_array_element_type(left: Type, right: Type) -> Type {
    if left == right {
        return left;
    }
    let left_underlying = literal_type_underlying(&left).unwrap_or(left);
    let right_underlying = literal_type_underlying(&right).unwrap_or(right);
    if transparent_type(&left_underlying) == transparent_type(&right_underlying) {
        left_underlying
    } else {
        Type::Union(vec![left_underlying, right_underlying])
    }
}

pub(super) fn literal_type_underlying(ty: &Type) -> Option<Type> {
    match ty {
        Type::Lit(expr) => literal_underlying_type(expr),
        _ => None,
    }
}

pub(super) fn field_value_type(field: &crate::ast::Field) -> Type {
    if field.optional && !matches!(field.ty, Type::Optional(_)) {
        Type::Optional(Box::new(field.ty.clone()))
    } else {
        field.ty.clone()
    }
}

pub(super) fn http_path_params(path: &str) -> Vec<String> {
    path.split('/')
        .filter_map(|segment| segment.strip_prefix(':'))
        .filter(|name| !name.is_empty())
        .map(|name| {
            name.split(['?', '#', '.'])
                .next()
                .unwrap_or(name)
                .to_string()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

pub(super) fn http_response_union_type(responses: &[(u16, Type)]) -> Type {
    match responses {
        [] => Type::Unknown,
        [(_, ty)] => ty.clone(),
        responses => Type::Union(responses.iter().map(|(_, ty)| ty.clone()).collect()),
    }
}

pub(super) fn http_response_map_type(responses: &[(u16, Type)]) -> Type {
    Type::Object(
        responses
            .iter()
            .map(|(status, ty)| Field {
                name: status.to_string(),
                ty: ty.clone(),
                readonly: true,
                optional: false,
                decorators: Vec::new(),
                span: Span::DUMMY,
            })
            .collect(),
    )
}

pub(super) fn http_status_forbids_body(status: u16) -> bool {
    (100..=199).contains(&status) || matches!(status, 204 | 205 | 304)
}

pub(super) fn http_reply_payload(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Apply { name, args } if name == "HttpReply" && args.len() == 1 => args.first(),
        _ => None,
    }
}

pub(super) fn http_named_param(param: &Param) -> Option<String> {
    match &param.pattern {
        ParamPattern::Name(name) => Some(name.clone()),
        ParamPattern::Array(_) | ParamPattern::Object(_) => None,
    }
}

pub(super) fn http_reply_finalization_chain_count(expr: &Expr, reply_name: &str) -> usize {
    match expr.kind() {
        ExprKind::MethodCall { object, method, .. }
            if method == "send" || method == "stream" || method == "error" =>
        {
            let inner_count = http_reply_finalization_chain_count(object, reply_name);
            if inner_count > 0 || http_reply_chain_rooted_expr(object, reply_name) {
                inner_count + 1
            } else {
                inner_count
            }
        }
        _ => 0,
    }
}

pub(super) fn http_reply_chain_opens_writable_stream(expr: &Expr, reply_name: &str) -> bool {
    match expr.kind() {
        ExprKind::MethodCall { object, method, .. } if method == "stream" => {
            http_reply_chain_rooted_expr(object, reply_name)
                || http_reply_chain_opens_writable_stream(object, reply_name)
        }
        ExprKind::MethodCall { object, .. } | ExprKind::OptionalMethodCall { object, .. } => {
            http_reply_chain_opens_writable_stream(object, reply_name)
        }
        _ => false,
    }
}

pub(super) fn http_reply_method_span(expr: &Expr, reply_name: &str) -> Option<Span> {
    match expr.kind() {
        ExprKind::MethodCall {
            object,
            method_span,
            ..
        } if http_reply_chain_rooted_expr(object, reply_name) && !method_span.is_dummy() => {
            Some(*method_span)
        }
        ExprKind::MethodCall { object, .. } | ExprKind::OptionalMethodCall { object, .. } => {
            http_reply_method_span(object, reply_name)
        }
        _ => None,
    }
}

pub(super) fn http_reply_status_without_send_expr(expr: &Expr, reply_name: &str) -> bool {
    http_reply_status_expr(expr, reply_name)
}

pub(super) fn http_reply_status_expr(expr: &Expr, reply_name: &str) -> bool {
    match expr.kind() {
        ExprKind::MethodCall { object, method, .. } if method == "status" => {
            http_reply_chain_rooted_expr(object, reply_name)
        }
        _ => false,
    }
}

pub(super) fn parse_trusted_proxy(value: &str) -> Option<(std::net::IpAddr, u8)> {
    let (address, prefix) = match value.split_once('/') {
        Some((address, prefix)) => (address, Some(prefix)),
        None => (value, None),
    };
    let network: std::net::IpAddr = address.trim().parse().ok()?;
    let full = if network.is_ipv4() { 32 } else { 128 };
    let prefix = match prefix {
        None => full,
        Some(prefix) => {
            let prefix: u8 = prefix.trim().parse().ok()?;
            if prefix > full {
                return None;
            }
            prefix
        }
    };
    Some((network, prefix))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HttpSchemaUse {
    Read,
    Write,
}

impl HttpSchemaUse {
    pub(super) fn allows_optional(self, payload: &Type) -> bool {
        match self {
            Self::Write => true,
            Self::Read => is_supported_http_scalar_type(payload),
        }
    }
}

pub(super) fn optional_payload_type(ty: &Type) -> &Type {
    match transparent_type(ty) {
        Type::Optional(inner) => inner,
        other => other,
    }
}

pub(super) fn invalid_cors_origin(origin: &str) -> Option<String> {
    if origin == "*" || origin == "null" {
        return None;
    }
    if origin.trim() != origin || origin.is_empty() {
        return Some("an origin carries no surrounding whitespace".to_string());
    }
    if origin
        .bytes()
        .any(|byte| byte.is_ascii_control() || byte == b' ')
    {
        return Some("an origin carries no spaces or control characters".to_string());
    }
    let Some((scheme, rest)) = origin.split_once("://") else {
        return Some(format!(
            "write the scheme too, as `https://{origin}`; a browser sends the origin with one"
        ));
    };
    if scheme.is_empty()
        || !scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
    {
        return Some(format!("`{scheme}` is not a valid scheme"));
    }
    if rest.is_empty() {
        return Some("write the host after the scheme".to_string());
    }
    if let Some(index) = rest.find(['/', '?', '#']) {
        let tail = &rest[index..];
        return Some(format!(
            "drop the trailing `{tail}`: a browser sends only the scheme, host and port, so this entry would never match"
        ));
    }
    None
}

pub(super) fn is_http_header_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

pub(super) fn is_runtime_owned_http_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection" | "content-length" | "set-cookie" | "transfer-encoding"
    )
}

pub(super) fn http_reply_send_span(expr: &Expr, reply_name: &str) -> Option<Span> {
    match expr.kind() {
        ExprKind::MethodCall {
            object,
            method,
            method_span,
            ..
        } if method == "send" || method == "stream" || method == "error" => {
            http_reply_send_span(object, reply_name).or_else(|| {
                (http_reply_chain_rooted_expr(object, reply_name) && !method_span.is_dummy())
                    .then_some(*method_span)
            })
        }
        ExprKind::OptionalMethodCall { object, method, .. }
            if method == "send" || method == "stream" || method == "error" =>
        {
            http_reply_send_span(object, reply_name)
        }
        _ => None,
    }
}

pub(super) fn http_reply_chain_sets_status(expr: &Expr, reply_name: &str) -> bool {
    match expr.kind() {
        ExprKind::MethodCall { object, method, .. }
        | ExprKind::OptionalMethodCall { object, method, .. } => {
            (method == "status" && http_reply_chain_rooted_expr(object, reply_name))
                || http_reply_chain_sets_status(object, reply_name)
        }
        _ => false,
    }
}

pub(super) fn http_reply_chain_rooted_expr(expr: &Expr, reply_name: &str) -> bool {
    match expr.kind() {
        ExprKind::Ident(name) if name == reply_name => true,
        ExprKind::MethodCall { object, .. } | ExprKind::OptionalMethodCall { object, .. } => {
            http_reply_chain_rooted_expr(object, reply_name)
        }
        _ => false,
    }
}

pub(super) fn merge_http_reply_flow_paths(paths: Vec<HttpReplyFlowPath>) -> Vec<HttpReplyFlowPath> {
    paths.into_iter().fold(Vec::new(), |mut merged, path| {
        if !merged.iter().any(|candidate| candidate == &path) {
            merged.push(path);
        }
        merged
    })
}

pub(super) fn http_reply_flow_contains_complex_control(body: &[Stmt]) -> bool {
    body.iter().any(http_reply_stmt_contains_complex_control)
}

pub(super) fn http_reply_stmt_contains_complex_control(statement: &Stmt) -> bool {
    match statement.kind() {
        StmtKind::TryCatch { .. }
        | StmtKind::While { .. }
        | StmtKind::DoWhile { .. }
        | StmtKind::For { .. }
        | StmtKind::ForOf { .. }
        | StmtKind::ForIn { .. }
        | StmtKind::Switch { .. }
        | StmtKind::Match { .. } => true,
        StmtKind::Expr(Expr {
            kind:
                ExprKind::If {
                    then_block,
                    else_block,
                    ..
                },
            ..
        }) => {
            http_reply_flow_contains_complex_control(then_block)
                || else_block
                    .as_deref()
                    .is_some_and(http_reply_flow_contains_complex_control)
        }
        StmtKind::Expr(Expr {
            kind: ExprKind::ArrowFn { body, .. },
            ..
        }) => http_reply_flow_contains_complex_control(body),
        _ => false,
    }
}

pub(super) fn http_reply_status_payload(ty: &Type) -> Option<(u16, Type)> {
    let Type::Apply { name, args } = ty else {
        return None;
    };
    if name != "__HttpReplyStatus" || args.len() != 2 {
        return None;
    }
    let status = match args.first()? {
        Type::Lit(expr) => match expr.as_ref() {
            Expr {
                kind: ExprKind::IntLit(value),
                ..
            } => u16::try_from(*value).ok()?,
            _ => return None,
        },
        _ => return None,
    };
    Some((status, args.get(1)?.clone()))
}

pub(super) fn http_reply_schema_for_status(reply: &Type, status: u16) -> Option<Type> {
    match transparent_type(reply) {
        Type::Object(fields) => fields
            .iter()
            .find(|field| field.name == status.to_string())
            .map(field_value_type),
        other => Some(other.clone()),
    }
}

pub(super) fn http_reply_payload_assignable(expected: &Type, actual: &Type) -> bool {
    match transparent_type(expected) {
        Type::Object(fields) => fields
            .iter()
            .map(field_value_type)
            .any(|schema| is_assignable(&schema, &ValueType::Known(actual.clone()))),
        Type::Union(variants) => variants
            .iter()
            .any(|variant| is_assignable(variant, &ValueType::Known(actual.clone()))),
        other => is_assignable(other, &ValueType::Known(actual.clone())),
    }
}

pub(super) fn infer_binding_type(value: &Expr, actual: &ValueType) -> Option<Type> {
    if matches!(
        actual,
        ValueType::IntegerLiteral | ValueType::NegativeIntegerLiteral
    ) && let Some(literal) = const_int_value(value)
    {
        if literal < i128::from(i64::MIN) || literal > i128::from(i64::MAX) {
            return Some(Type::I128);
        }
        if literal < i128::from(i32::MIN) || literal > i128::from(i32::MAX) {
            return Some(Type::I64);
        }
    }
    value_type_to_type(actual)
}

pub(super) fn const_int_value(expr: &Expr) -> Option<i128> {
    match expr.kind() {
        ExprKind::IntLit(value) => Some(*value),
        ExprKind::Neg(inner) => match inner.as_ref() {
            Expr {
                kind: ExprKind::IntLit(value),
                ..
            } => Some(-value),
            _ => None,
        },
        _ => None,
    }
}

pub const MAX_ARRAY_REPEAT_COUNT: usize = 65_536;

pub fn integer_constant_value(expr: &Expr) -> Option<Result<i128, ()>> {
    match expr.kind() {
        ExprKind::IntLit(value) => Some(Ok(*value)),
        ExprKind::UIntLit(value) => i128::try_from(*value).ok().map(Ok),
        ExprKind::Neg(inner) => integer_constant_value(inner)
            .map(|value| value.and_then(|value| value.checked_neg().ok_or(()))),
        ExprKind::BitNot(inner) => {
            integer_constant_value(inner).map(|value| value.map(|value| !value))
        }
        ExprKind::FieldAccess { object, field } => {
            let ExprKind::Ident(type_name) = object.kind() else {
                return None;
            };
            let ty = numeric_constant_type(type_name, field)?;
            let (min, max) = integer_type_bounds(&ty)?;
            match field.as_str() {
                "MIN" => Some(Ok(min)),
                "MAX" => max.map(Ok),
                _ => None,
            }
        }
        ExprKind::BinOp { op, left, right } => {
            let left = integer_constant_value(left)?;
            let right = integer_constant_value(right)?;
            Some(left.and_then(|left| {
                right.and_then(|right| {
                    let value = match op {
                        BinOp::Add => left.checked_add(right),
                        BinOp::Sub => left.checked_sub(right),
                        BinOp::Mul => left.checked_mul(right),
                        BinOp::Div => left.checked_div(right),
                        BinOp::Mod => left.checked_rem(right),
                        BinOp::Pow => u32::try_from(right)
                            .ok()
                            .and_then(|power| left.checked_pow(power)),
                        BinOp::BitAnd => Some(left & right),
                        BinOp::BitOr => Some(left | right),
                        BinOp::BitXor => Some(left ^ right),
                        BinOp::Shl => u32::try_from(right)
                            .ok()
                            .and_then(|shift| left.checked_shl(shift)),
                        BinOp::Shr => u32::try_from(right)
                            .ok()
                            .and_then(|shift| left.checked_shr(shift)),
                        _ => return Err(()),
                    };
                    value.ok_or(())
                })
            }))
        }
        ExprKind::TypeAssert { value, .. } | ExprKind::ConstAssert(value) => {
            integer_constant_value(value)
        }
        _ => None,
    }
}

pub(super) fn is_integer_literal(expr: &Expr) -> bool {
    matches!(expr.kind(), ExprKind::IntLit(_) | ExprKind::UIntLit(_))
        || matches!(expr.kind(), ExprKind::Neg(inner) if matches!(inner.kind(), ExprKind::IntLit(_)))
}

pub(super) fn value_type_to_type(value: &ValueType) -> Option<Type> {
    match value {
        ValueType::Known(ty) => Some(ty.clone()),
        ValueType::IntegerLiteral => Some(Type::I32),
        ValueType::NegativeIntegerLiteral => Some(Type::I32),
        ValueType::FloatLiteral => Some(Type::F64),
        ValueType::Null => Some(Type::Optional(Box::new(Type::Never))),
        ValueType::Object(fields) => Some(Type::Object(
            fields
                .iter()
                .filter_map(|(name, value)| {
                    value_type_to_type(value).map(|ty| crate::ast::Field {
                        name: name.clone(),
                        ty,
                        readonly: false,
                        optional: false,
                        decorators: Vec::new(),
                        span: Span::DUMMY,
                    })
                })
                .collect(),
        )),
        ValueType::Array(values) => {
            infer_array_element_type(values).map(|ty| Type::Array(Box::new(ty)))
        }
        ValueType::Union(_)
        | ValueType::UnsignedIntegerLiteral
        | ValueType::Unknown
        | ValueType::Error => None,
    }
}

pub(super) fn literal_expr_matches(expected: &Expr, actual: &Expr) -> bool {
    match (expected, actual) {
        (
            Expr {
                kind: ExprKind::StrLit(expected),
                ..
            },
            Expr {
                kind: ExprKind::StrLit(actual),
                ..
            },
        ) => expected == actual,
        (
            Expr {
                kind: ExprKind::IntLit(expected),
                ..
            },
            Expr {
                kind: ExprKind::IntLit(actual),
                ..
            },
        ) => expected == actual,
        (
            Expr {
                kind: ExprKind::UIntLit(expected),
                ..
            },
            Expr {
                kind: ExprKind::UIntLit(actual),
                ..
            },
        ) => expected == actual,
        (
            Expr {
                kind: ExprKind::FloatLit(expected),
                ..
            },
            Expr {
                kind: ExprKind::FloatLit(actual),
                ..
            },
        ) => expected == actual,
        (
            Expr {
                kind: ExprKind::BoolLit(expected),
                ..
            },
            Expr {
                kind: ExprKind::BoolLit(actual),
                ..
            },
        ) => expected == actual,
        (
            Expr {
                kind: ExprKind::Null,
                ..
            },
            Expr {
                kind: ExprKind::Null,
                ..
            },
        ) => true,
        (
            Expr {
                kind: ExprKind::None,
                ..
            },
            Expr {
                kind: ExprKind::None,
                ..
            },
        ) => true,
        _ => false,
    }
}

pub(super) fn literal_underlying_type(lit_expr: &Expr) -> Option<Type> {
    match lit_expr.kind() {
        ExprKind::StrLit(_) => Some(Type::Str),
        ExprKind::RegexLit { .. } => Some(Type::Regex),
        ExprKind::IntLit(_) => Some(Type::I32),
        ExprKind::UIntLit(_) => Some(Type::U128),
        ExprKind::FloatLit(_) => Some(Type::F64),
        ExprKind::BoolLit(_) => Some(Type::Bool),
        ExprKind::Null | ExprKind::None => Some(Type::Never),
        _ => None,
    }
}

pub(super) fn is_lit_assignable_to(ty: &Type, expected: &Type) -> bool {
    match ty {
        Type::Lit(expr) => match expr.as_ref() {
            Expr {
                kind: ExprKind::StrLit(_),
                ..
            } => matches!(transparent_type(expected), Type::Str),
            Expr {
                kind: ExprKind::RegexLit { .. },
                ..
            } => matches!(transparent_type(expected), Type::Regex),
            Expr {
                kind: ExprKind::IntLit(_),
                ..
            } => is_integer_type(transparent_type(expected)),
            Expr {
                kind: ExprKind::UIntLit(_),
                ..
            } => matches!(transparent_type(expected), Type::U128 | Type::Usize),
            Expr {
                kind: ExprKind::FloatLit(_),
                ..
            } => is_float_type(transparent_type(expected)),
            Expr {
                kind: ExprKind::BoolLit(_),
                ..
            } => matches!(transparent_type(expected), Type::Bool),
            _ => false,
        },
        _ => false,
    }
}

pub(super) fn is_assignable(expected: &Type, actual: &ValueType) -> bool {
    if matches!(expected, Type::Infer | Type::Unknown) {
        return true;
    }
    match actual {
        ValueType::Error | ValueType::Unknown => true,
        ValueType::IntegerLiteral => {
            !matches!(literal_target_type(expected), Type::Lit(_))
                && is_integer_type(literal_target_type(expected))
        }
        ValueType::NegativeIntegerLiteral => {
            !matches!(literal_target_type(expected), Type::Lit(_))
                && matches!(
                    literal_target_type(expected),
                    Type::I8 | Type::I16 | Type::I32 | Type::I64 | Type::I128 | Type::Isize
                )
        }
        ValueType::UnsignedIntegerLiteral => {
            !matches!(literal_target_type(expected), Type::Lit(_))
                && matches!(literal_target_type(expected), Type::U128 | Type::Usize)
        }
        ValueType::FloatLiteral => {
            !matches!(literal_target_type(expected), Type::Lit(_))
                && (is_float_type(literal_target_type(expected))
                    || matches!(literal_target_type(expected), Type::Decimal))
        }
        ValueType::Null => {
            matches!(expected, Type::Optional(_))
        }
        ValueType::Known(actual) => {
            if matches!(actual, Type::Never) {
                return true;
            }
            match (expected, actual) {
                (Type::Ref(expected), Type::Ref(actual) | Type::MutRef(actual))
                | (Type::MutRef(expected), Type::MutRef(actual)) => {
                    return transparent_type(expected) == transparent_type(actual)
                        || is_lit_assignable_to(actual, expected);
                }
                (Type::MutRef(_), Type::Ref(_)) => return false,
                (Type::Ref(_) | Type::MutRef(_), _) => return false,
                (_, Type::Ref(_) | Type::MutRef(_)) => return false,
                _ => {}
            }
            if let Type::Readonly(inner) = actual {
                if matches!(inner.as_ref(), Type::Array(_) | Type::Tuple(_)) {
                    return match expected {
                        Type::Readonly(expected_inner) => {
                            is_assignable(expected_inner, &ValueType::Known(*inner.clone()))
                        }
                        _ => false,
                    };
                }
                return is_assignable(expected, &ValueType::Known(*inner.clone()));
            }
            if let (Type::Array(element), Type::Tuple(positions)) = (expected, actual) {
                return !positions.is_empty()
                    && positions.iter().all(|position| {
                        is_assignable(element, &ValueType::Known(position.clone()))
                    });
            }
            match expected {
                Type::Apply { name, args } if name == "HttpReply" && args.len() == 1 => {
                    match actual {
                        Type::Apply {
                            name: actual_name,
                            args: actual_args,
                        } if actual_name == "HttpReply" && actual_args.len() == 1 => {
                            http_reply_payload_assignable(&args[0], &actual_args[0])
                        }
                        _ => false,
                    }
                }
                Type::Apply { name, args }
                    if matches!(name.as_str(), "Promise" | "DbPromise") && args.len() == 1 =>
                {
                    match actual {
                        Type::Apply {
                            name: actual_name,
                            args: actual_args,
                        } if matches!(actual_name.as_str(), "Promise" | "DbPromise")
                            && actual_args.len() == 1 =>
                        {
                            is_assignable(&args[0], &ValueType::Known(actual_args[0].clone()))
                        }
                        _ => false,
                    }
                }
                Type::Readonly(inner) => is_assignable(inner, &ValueType::Known(actual.clone())),
                Type::Optional(inner) => match actual {
                    Type::Optional(actual_inner) => {
                        transparent_type(inner) == transparent_type(actual_inner)
                            || is_lit_assignable_to(actual_inner, inner)
                    }
                    _ => {
                        transparent_type(inner) == transparent_type(actual)
                            || is_lit_assignable_to(actual, inner)
                    }
                },
                Type::Never => false,
                Type::Lit(expected_lit) => {
                    if let Type::Lit(actual_lit) = actual {
                        expected_lit == actual_lit
                    } else {
                        false
                    }
                }
                Type::Graph(expected_node, expected_weight) => match actual {
                    Type::Graph(actual_node, actual_weight) => {
                        is_assignable(expected_node, &ValueType::Known((**actual_node).clone()))
                            && is_assignable(
                                expected_weight,
                                &ValueType::Known((**actual_weight).clone()),
                            )
                    }
                    _ => false,
                },
                _ => {
                    transparent_type(expected) == transparent_type(actual)
                        || is_lit_assignable_to(actual, expected)
                }
            }
        }
        ValueType::Object(fields) => match expected {
            Type::Object(expected_fields) => {
                expected_fields.iter().all(|expected_field| {
                    fields
                        .iter()
                        .find(|(actual_name, _)| actual_name == &expected_field.name)
                        .map(|(_, actual_ty)| {
                            is_assignable(&field_value_type(expected_field), actual_ty)
                        })
                        .unwrap_or(expected_field.optional)
                }) && fields.iter().all(|(actual_name, _)| {
                    expected_fields
                        .iter()
                        .any(|expected_field| expected_field.name == *actual_name)
                })
            }
            Type::Readonly(inner) => is_assignable(inner, actual),
            _ => false,
        },
        ValueType::Array(elements) => match expected {
            Type::Array(element) => elements.iter().all(|elem| is_assignable(element, elem)),
            Type::Readonly(inner) => is_assignable(inner, actual),
            _ => false,
        },
        ValueType::Union(variants) => variants
            .iter()
            .all(|variant| is_assignable(expected, variant)),
    }
}

pub(super) fn common_union_field_type(variants: &[Type], field: &str) -> Option<Type> {
    let mut common = None;
    for variant in variants {
        let Type::Object(fields) = variant else {
            return None;
        };
        let field_ty = fields
            .iter()
            .find(|candidate| candidate.name == field)
            .map(|candidate| candidate.ty.clone())?;
        common = match common {
            None => Some(field_ty),
            Some(existing) => common_field_type(existing, field_ty),
        };
    }
    common
}

pub(super) fn common_underlying_type(variants: &[Type]) -> Option<Type> {
    let mut common: Option<Type> = None;
    for variant in variants {
        let underlying = match transparent_type(variant) {
            Type::Lit(expr) => literal_underlying_type(expr)?,
            other => other.clone(),
        };
        match &common {
            None => common = Some(underlying),
            Some(existing) if *existing == underlying => {}
            Some(_) => return None,
        }
    }
    common
}

pub(super) fn common_field_type(left: Type, right: Type) -> Option<Type> {
    if left == right {
        return Some(left);
    }
    let left_underlying = match &left {
        Type::Lit(expr) => literal_underlying_type(expr),
        other => Some(other.clone()),
    };
    let right_underlying = match &right {
        Type::Lit(expr) => literal_underlying_type(expr),
        other => Some(other.clone()),
    };
    match (left_underlying, right_underlying) {
        (Some(left), Some(right)) if transparent_type(&left) == transparent_type(&right) => {
            Some(left)
        }
        _ => None,
    }
}

pub(super) fn types_overlap(left: &ValueType, right: &ValueType) -> bool {
    match (left, right) {
        (ValueType::Error | ValueType::Unknown, _) | (_, ValueType::Error | ValueType::Unknown) => {
            true
        }
        (ValueType::Null, ValueType::Known(Type::Optional(_)) | ValueType::Null)
        | (ValueType::Known(Type::Optional(_)), ValueType::Null) => true,
        (
            ValueType::IntegerLiteral
            | ValueType::NegativeIntegerLiteral
            | ValueType::UnsignedIntegerLiteral,
            other,
        )
        | (
            other,
            ValueType::IntegerLiteral
            | ValueType::NegativeIntegerLiteral
            | ValueType::UnsignedIntegerLiteral,
        ) => is_numeric(other),
        (ValueType::FloatLiteral, other) | (other, ValueType::FloatLiteral) => is_numeric(other),
        (ValueType::Known(left), ValueType::Known(right)) => {
            if matches!(left, Type::Unknown) || matches!(right, Type::Unknown) {
                true
            } else if matches!(left, Type::Never) || matches!(right, Type::Never) {
                false
            } else if transparent_type(left) == transparent_type(right)
                || is_lit_assignable_to(left, right)
                || is_lit_assignable_to(right, left)
            {
                true
            } else {
                let overlaps_variant = |variants: &[Type], other: &Type| {
                    variants.iter().any(|variant| {
                        types_overlap(
                            &ValueType::Known(variant.clone()),
                            &ValueType::Known(other.clone()),
                        )
                    })
                };
                union_variants_of(left).is_some_and(|variants| overlaps_variant(variants, right))
                    || union_variants_of(right)
                        .is_some_and(|variants| overlaps_variant(variants, left))
            }
        }
        (ValueType::Array(left), ValueType::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right)
                    .all(|(left, right)| types_overlap(left, right))
        }
        (ValueType::Union(left), right) => left.iter().any(|l| types_overlap(l, right)),
        (left, ValueType::Union(right)) => right.iter().any(|r| types_overlap(left, r)),
        _ => false,
    }
}

pub(super) fn union_variants_of(ty: &Type) -> Option<&[Type]> {
    match transparent_type(ty) {
        Type::Union(variants) => Some(variants),
        _ => None,
    }
}

pub(super) fn transparent_value_type(value: &ValueType) -> Option<&Type> {
    match value {
        ValueType::Known(ty) => Some(transparent_type(ty)),
        _ => None,
    }
}

pub(super) fn infer_generic_types(
    expected: &Type,
    actual: &ValueType,
    substitutions: &mut HashMap<String, Type>,
) {
    let actual = match actual {
        ValueType::Known(ty) => Some(ty.clone()),
        ValueType::IntegerLiteral => Some(Type::I32),
        ValueType::NegativeIntegerLiteral => Some(Type::I32),
        ValueType::FloatLiteral => Some(Type::F64),
        ValueType::Object(_) | ValueType::Array(_) | ValueType::Union(_) => {
            value_type_to_type(actual).map(widen_generic_inference_type)
        }
        _ => None,
    };
    match (expected, actual) {
        (Type::Generic(name), Some(actual)) => {
            substitutions.entry(name.clone()).or_insert(actual);
        }
        (Type::Array(expected), Some(Type::Array(actual)))
        | (Type::Ref(expected), Some(Type::Ref(actual)))
        | (Type::MutRef(expected), Some(Type::MutRef(actual)))
        | (Type::Optional(expected), Some(Type::Optional(actual))) => {
            infer_generic_types(expected, &ValueType::Known(*actual), substitutions);
        }
        (Type::Array(expected), Some(Type::Tuple(actual))) => {
            for actual in actual {
                infer_generic_types(expected, &ValueType::Known(actual), substitutions);
            }
        }
        (Type::Tuple(expected), Some(Type::Tuple(actual))) if expected.len() == actual.len() => {
            for (expected, actual) in expected.iter().zip(actual) {
                infer_generic_types(expected, &ValueType::Known(actual.clone()), substitutions);
            }
        }
        (
            Type::Function {
                ret: expected_ret, ..
            },
            Some(Type::Function {
                ret: actual_ret, ..
            }),
        ) => {
            infer_generic_types(expected_ret, &ValueType::Known(*actual_ret), substitutions);
        }
        (
            Type::Apply {
                name: expected_name,
                args: expected_args,
            },
            Some(Type::Apply {
                name: actual_name,
                args: actual_args,
            }),
        ) if expected_name == &actual_name && expected_args.len() == actual_args.len() => {
            for (expected, actual) in expected_args.iter().zip(actual_args) {
                infer_generic_types(expected, &ValueType::Known(actual), substitutions);
            }
        }
        _ => {}
    }
}

pub(super) fn widen_generic_inference_type(ty: Type) -> Type {
    match ty {
        Type::Lit(expression) => match expression.kind() {
            ExprKind::StrLit(_) => Type::Str,
            ExprKind::BoolLit(_) => Type::Bool,
            ExprKind::IntLit(_) => Type::I32,
            ExprKind::UIntLit(_) => Type::U128,
            ExprKind::FloatLit(_) => Type::F64,
            _ => Type::Lit(expression),
        },
        Type::Object(fields) => Type::Object(
            fields
                .into_iter()
                .map(|field| Field {
                    ty: widen_generic_inference_type(field.ty),
                    ..field
                })
                .collect(),
        ),
        Type::Array(inner) => Type::Array(Box::new(widen_generic_inference_type(*inner))),
        Type::Tuple(elements) => Type::Tuple(
            elements
                .into_iter()
                .map(widen_generic_inference_type)
                .collect(),
        ),
        other => other,
    }
}

pub(super) fn jwt_claim_field_type_supported(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Str
            | Type::Bool
            | Type::F32
            | Type::F64
            | Type::Decimal
            | Type::I32
            | Type::I64
            | Type::U32
            | Type::U64
    )
}

pub(super) fn substitute_generics(ty: &Type, substitutions: &HashMap<String, Type>) -> Type {
    match ty {
        Type::Generic(name) => substitutions
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        Type::Array(inner) => Type::Array(Box::new(substitute_generics(inner, substitutions))),
        Type::Tuple(elements) => Type::Tuple(
            elements
                .iter()
                .map(|element| substitute_generics(element, substitutions))
                .collect(),
        ),
        Type::Ref(inner) => Type::Ref(Box::new(substitute_generics(inner, substitutions))),
        Type::MutRef(inner) => Type::MutRef(Box::new(substitute_generics(inner, substitutions))),
        Type::Optional(inner) => {
            Type::Optional(Box::new(substitute_generics(inner, substitutions)))
        }
        Type::Result { success, error } => Type::Result {
            success: Box::new(substitute_generics(success, substitutions)),
            error: Box::new(substitute_generics(error, substitutions)),
        },
        Type::Function { params, ret } => Type::Function {
            params: params
                .iter()
                .map(|param| substitute_generics(param, substitutions))
                .collect(),
            ret: Box::new(substitute_generics(ret, substitutions)),
        },
        Type::Apply { name, args } => Type::Apply {
            name: name.clone(),
            args: args
                .iter()
                .map(|argument| substitute_generics(argument, substitutions))
                .collect(),
        },
        _ => ty.clone(),
    }
}

pub(super) fn infer_array_element_type(values: &[ValueType]) -> Option<Type> {
    let first = values.first()?;
    let inferred = match first {
        ValueType::IntegerLiteral => Type::I32,
        ValueType::NegativeIntegerLiteral => Type::I32,
        ValueType::FloatLiteral => Type::F64,
        ValueType::Known(ty) => ty.clone(),
        _ => return None,
    };
    values
        .iter()
        .all(|value| is_assignable(&inferred, value))
        .then_some(inferred)
}

pub(super) fn infer_map_entry_types(entries: &ValueType) -> Option<(Type, Type)> {
    let entries = match entries {
        ValueType::Array(entries) => entries,
        ValueType::Known(Type::Array(entry)) => match transparent_type(entry) {
            Type::Tuple(pair) if pair.len() == 2 => {
                return Some((pair[0].clone(), pair[1].clone()));
            }
            _ => return None,
        },
        _ => return None,
    };
    let mut keys = Vec::with_capacity(entries.len());
    let mut values = Vec::with_capacity(entries.len());
    for entry in entries {
        let (key, value) = match entry {
            ValueType::Array(pair) if pair.len() == 2 => (pair[0].clone(), pair[1].clone()),
            ValueType::Known(ty) => match transparent_type(ty) {
                Type::Tuple(pair) if pair.len() == 2 => (
                    ValueType::Known(pair[0].clone()),
                    ValueType::Known(pair[1].clone()),
                ),
                _ => return None,
            },
            _ => return None,
        };
        keys.push(key);
        values.push(value);
    }
    Some((
        infer_array_element_type(&keys)?,
        infer_array_element_type(&values)?,
    ))
}

pub(super) fn infer_callback_return(values: &[ValueType]) -> Option<Type> {
    let first = values.first()?;
    let inferred = match first {
        ValueType::IntegerLiteral => Type::I32,
        ValueType::NegativeIntegerLiteral => Type::I32,
        ValueType::FloatLiteral => Type::F64,
        ValueType::Known(ty) => ty.clone(),
        ValueType::Null => return None,
        ValueType::Array(values) => Type::Array(Box::new(infer_array_element_type(values)?)),
        ValueType::UnsignedIntegerLiteral
        | ValueType::Unknown
        | ValueType::Error
        | ValueType::Object(_)
        | ValueType::Union(_) => return None,
    };
    values
        .iter()
        .all(|value| concrete_value_type(value).as_ref() == Some(&inferred))
        .then_some(inferred)
}

pub(super) fn concrete_value_type(value: &ValueType) -> Option<Type> {
    match value {
        ValueType::Known(ty) => Some(ty.clone()),
        ValueType::IntegerLiteral => Some(Type::I32),
        ValueType::NegativeIntegerLiteral => Some(Type::I32),
        ValueType::FloatLiteral => Some(Type::F64),
        ValueType::Array(values) => Some(Type::Array(Box::new(infer_array_element_type(values)?))),
        ValueType::Null
        | ValueType::UnsignedIntegerLiteral
        | ValueType::Unknown
        | ValueType::Error
        | ValueType::Object(_)
        | ValueType::Union(_) => None,
    }
}

pub(super) fn static_object_projection_key_names(value: &ValueType) -> Option<Vec<String>> {
    let ValueType::Known(ty) = value else {
        return None;
    };
    let element = match transparent_type(ty) {
        Type::Array(element) => element.as_ref(),
        _ => return None,
    };
    static_string_literal_type_names(element)
}

pub(super) fn static_string_literal_type_names(ty: &Type) -> Option<Vec<String>> {
    match transparent_type(ty) {
        Type::Never => Some(Vec::new()),
        Type::Lit(expr) => match expr.kind() {
            ExprKind::StrLit(name) => Some(vec![name.clone()]),
            _ => None,
        },
        Type::Union(variants) => variants
            .iter()
            .map(static_string_literal_type_names)
            .collect::<Option<Vec<_>>>()
            .map(|names| names.into_iter().flatten().collect()),
        _ => None,
    }
}

pub(super) fn is_numeric(ty: &ValueType) -> bool {
    matches!(
        ty,
        ValueType::IntegerLiteral
            | ValueType::UnsignedIntegerLiteral
            | ValueType::FloatLiteral
            | ValueType::NegativeIntegerLiteral
    ) || matches!(ty, ValueType::Known(known) if is_numeric_type(transparent_type(known)))
        || matches!(ty, ValueType::Unknown | ValueType::Error)
}

pub(super) fn is_integer_value(ty: &ValueType) -> bool {
    matches!(
        ty,
        ValueType::IntegerLiteral
            | ValueType::NegativeIntegerLiteral
            | ValueType::UnsignedIntegerLiteral
    ) || matches!(ty, ValueType::Known(known) if is_integer_type(transparent_type(known)))
        || matches!(ty, ValueType::Unknown | ValueType::Error)
}

pub(super) fn is_optional_value(ty: &ValueType) -> bool {
    matches!(ty, ValueType::Known(Type::Optional(_)))
}

pub(super) fn is_result_value(ty: &ValueType) -> bool {
    matches!(ty, ValueType::Known(Type::Result { .. }))
}

pub(super) fn binop_display_name(op: &BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Pow => "**",
        BinOp::Eq => "===",
        BinOp::Neq => "!==",
        BinOp::Gt => ">",
        BinOp::Lt => "<",
        BinOp::Gte => ">=",
        BinOp::Lte => "<=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::NullCoalesce => "??",
        BinOp::BitAnd => "&",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::Shl => "<<",
        BinOp::Shr => ">>",
        BinOp::UShr => ">>>",
    }
}

pub(super) fn is_numeric_type(ty: &Type) -> bool {
    is_integer_type(ty) || is_float_type(ty) || matches!(ty, Type::Decimal)
}

pub(super) fn value_type_is_decimal(value: &ValueType) -> bool {
    matches!(value, ValueType::Known(ty) if matches!(transparent_type(ty), Type::Decimal))
}

pub(super) fn is_builtin_type_name(name: &str) -> bool {
    matches!(
        name,
        "Promise"
            | "Mutex"
            | "MutexGuard"
            | "RwLock"
            | "RwReadGuard"
            | "RwWriteGuard"
            | "Semaphore"
            | "SemaphorePermit"
            | "Tuple"
            | "Optional"
            | "Result"
            | "Range"
            | "Error"
            | "AggregateError"
            | "__HttpReplyStatus"
            | "DigestStream"
    )
}

pub(super) fn value_type_is_numeric(value: &ValueType) -> bool {
    match value {
        ValueType::Known(ty) => is_numeric_type(transparent_type(ty)),
        ValueType::IntegerLiteral
        | ValueType::NegativeIntegerLiteral
        | ValueType::UnsignedIntegerLiteral
        | ValueType::FloatLiteral => true,
        _ => false,
    }
}

pub(super) fn is_collection_key_type(ty: &Type) -> bool {
    let ty = transparent_type(ty);
    matches!(ty, Type::Str | Type::Bool | Type::Char | Type::Decimal)
        || is_integer_type(ty)
        || is_float_type(ty)
}

pub(super) fn is_stringifiable_scalar(value: &ValueType) -> bool {
    if matches!(
        value,
        ValueType::IntegerLiteral
            | ValueType::NegativeIntegerLiteral
            | ValueType::UnsignedIntegerLiteral
            | ValueType::FloatLiteral
    ) {
        return true;
    }
    match transparent_value_type(value) {
        Some(ty) => {
            matches!(ty, Type::Str | Type::Bool | Type::Char)
                || is_integer_type(ty)
                || is_float_type(ty)
                || matches!(ty, Type::Decimal)
        }
        None => false,
    }
}

pub(super) fn narrowest_integer_type_for(literal: i128, rejected: &Type) -> Option<&'static str> {
    let unsigned: [&str; 5] = ["u8", "u16", "u32", "u64", "u128"];
    let signed: [&str; 5] = ["i8", "i16", "i32", "i64", "i128"];
    let candidates = if is_unsigned_integer_type(rejected) && literal >= 0 {
        unsigned
    } else {
        signed
    };
    candidates.into_iter().find(|name| {
        type_name_to_type(name)
            .and_then(|ty| integer_type_bounds(&ty))
            .is_some_and(|(min, max)| literal >= min && max.is_none_or(|max| literal <= max))
    })
}

pub(super) fn type_name_to_type(name: &str) -> Option<Type> {
    match name {
        "i8" => Some(Type::I8),
        "i16" => Some(Type::I16),
        "i32" => Some(Type::I32),
        "i64" => Some(Type::I64),
        "i128" => Some(Type::I128),
        "u8" => Some(Type::U8),
        "u16" => Some(Type::U16),
        "u32" => Some(Type::U32),
        "u64" => Some(Type::U64),
        "u128" => Some(Type::U128),
        _ => None,
    }
}

pub(super) fn is_string(ty: &ValueType) -> bool {
    matches!(ty, ValueType::Known(known) if matches!(transparent_type(known), Type::Str))
}

pub(super) fn is_console_value(ty: &ValueType) -> bool {
    matches!(ty, ValueType::Unknown | ValueType::Error)
        || is_numeric(ty)
        || is_string(ty)
        || matches!(ty, ValueType::Known(known) if matches!(transparent_type(known), Type::Regex | Type::Bool | Type::Char))
        || matches!(
            ty,
            ValueType::Known(known)
                if matches!(transparent_type(known), Type::Unknown | Type::Lit(_))
        )
        || matches!(ty, ValueType::Known(Type::Union(variants)) if variants.iter().all(|v| {
            is_console_value(&ValueType::Known(v.clone()))
        }))
        || matches!(ty, ValueType::Known(Type::Tuple(elements)) if elements.iter().all(|element| {
            is_console_value(&ValueType::Known(element.clone()))
        }))
        || matches!(ty, ValueType::Known(Type::Array(element) | Type::Readonly(element))
            if is_console_value(&ValueType::Known((**element).clone())))
        || matches!(
            ty,
            ValueType::Known(Type::Named(_) | Type::Object(_)) | ValueType::Object(_)
        )
}

pub(super) fn is_template_value(ty: &ValueType) -> bool {
    if matches!(ty, ValueType::Known(known) if matches!(transparent_type(known), Type::Array(_))) {
        return false;
    }
    is_console_value(ty) || matches!(ty, ValueType::Null)
}

pub(super) fn expression_path(expr: &Expr) -> Option<String> {
    match expr.kind() {
        ExprKind::Ident(name) => Some(name.clone()),
        ExprKind::FieldAccess { object, field } => {
            Some(format!("{}.{field}", expression_path(object)?))
        }
        _ => None,
    }
}

pub(super) fn expression_subject(expr: &Expr) -> Option<&str> {
    match expr.kind() {
        ExprKind::Ident(name) | ExprKind::Borrow { name, .. } => Some(name),
        ExprKind::FnCall { callee, .. } => Some(callee),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FactKind {
    Value,
    Length,
}

impl ValidatorFacts {
    pub(super) fn evaluate_value_condition(
        &self,
        op: &BinOp,
        literal: &Expr,
    ) -> Option<FactCondition> {
        if matches!(literal.kind(), ExprKind::Null) && self.non_null {
            return match op {
                BinOp::Eq => Some(FactCondition::AlwaysFalse),
                BinOp::Neq => Some(FactCondition::AlwaysTrue),
                _ => None,
            };
        }
        if let Some(allowed) = &self.allowed_literals {
            let contains = allowed
                .iter()
                .any(|candidate| literal_expr_matches(candidate, literal));
            match op {
                BinOp::Eq if !contains => return Some(FactCondition::AlwaysFalse),
                BinOp::Neq if !contains => return Some(FactCondition::AlwaysTrue),
                BinOp::Eq if allowed.len() == 1 && contains => {
                    return Some(FactCondition::AlwaysTrue);
                }
                BinOp::Neq if allowed.len() == 1 && contains => {
                    return Some(FactCondition::AlwaysFalse);
                }
                _ => {}
            }
        }
        if self
            .disallowed_literals
            .iter()
            .any(|candidate| literal_expr_matches(candidate, literal))
        {
            return match op {
                BinOp::Eq => Some(FactCondition::AlwaysFalse),
                BinOp::Neq => Some(FactCondition::AlwaysTrue),
                _ => None,
            };
        }
        let value = numeric_literal_value(literal)?;
        evaluate_numeric_bounds(op, value, self.numeric_min, self.numeric_max)
    }

    pub(super) fn evaluate_length_condition(
        &self,
        op: &BinOp,
        literal: &Expr,
    ) -> Option<FactCondition> {
        let value = integer_literal_value(literal)? as f64;
        evaluate_numeric_bounds(
            op,
            value,
            self.length_min.map(|value| value as f64),
            self.length_max.map(|value| value as f64),
        )
    }
}

pub(super) fn facts_from_decorators(decorators: &[ValidatorDecorator]) -> ValidatorFacts {
    let mut facts = ValidatorFacts::default();
    for decorator in decorators {
        match decorator.name.as_str() {
            "equal" => {
                if let Some(literal) = primitive_fact_literal(decorator.args.first()) {
                    facts.allowed_literals = Some(vec![literal]);
                }
            }
            "notEqual" => {
                if let Some(literal) = primitive_fact_literal(decorator.args.first()) {
                    facts.disallowed_literals.push(literal);
                }
            }
            "in" => {
                if let Some(Expr {
                    kind: ExprKind::ArrayLit(values),
                    ..
                }) = decorator.args.first()
                {
                    let values = values
                        .iter()
                        .filter_map(|value| primitive_fact_literal(Some(value)))
                        .collect::<Vec<_>>();
                    if !values.is_empty() {
                        facts.allowed_literals = Some(values);
                    }
                }
            }
            "notIn" => {
                if let Some(Expr {
                    kind: ExprKind::ArrayLit(values),
                    ..
                }) = decorator.args.first()
                {
                    facts.disallowed_literals.extend(
                        values
                            .iter()
                            .filter_map(|value| primitive_fact_literal(Some(value))),
                    );
                }
            }
            "min" => {
                if let Some(value) = numeric_arg_value(&decorator.args, 0) {
                    facts.numeric_min = Some(
                        facts
                            .numeric_min
                            .map_or(value, |current| current.max(value)),
                    );
                }
            }
            "max" => {
                if let Some(value) = numeric_arg_value(&decorator.args, 0) {
                    facts.numeric_max = Some(
                        facts
                            .numeric_max
                            .map_or(value, |current| current.min(value)),
                    );
                }
            }
            "range" => {
                if let Some(value) = numeric_arg_value(&decorator.args, 0) {
                    facts.numeric_min = Some(
                        facts
                            .numeric_min
                            .map_or(value, |current| current.max(value)),
                    );
                }
                if let Some(value) = numeric_arg_value(&decorator.args, 1) {
                    facts.numeric_max = Some(
                        facts
                            .numeric_max
                            .map_or(value, |current| current.min(value)),
                    );
                }
            }
            "positive" => {
                facts.numeric_min = Some(facts.numeric_min.map_or(0.0, |current| current.max(0.0)));
                facts
                    .disallowed_literals
                    .push(Expr::synthetic(ExprKind::IntLit(0)));
            }
            "negative" => {
                facts.numeric_max = Some(facts.numeric_max.map_or(0.0, |current| current.min(0.0)));
                facts
                    .disallowed_literals
                    .push(Expr::synthetic(ExprKind::IntLit(0)));
            }
            "nonNegative" => {
                facts.numeric_min = Some(facts.numeric_min.map_or(0.0, |current| current.max(0.0)));
            }
            "nonPositive" => {
                facts.numeric_max = Some(facts.numeric_max.map_or(0.0, |current| current.min(0.0)));
            }
            "minLength" | "minItems" => {
                if let Some(value) = integer_arg_value(&decorator.args, 0) {
                    facts.length_min =
                        Some(facts.length_min.map_or(value, |current| current.max(value)));
                }
            }
            "maxLength" | "maxItems" => {
                if let Some(value) = integer_arg_value(&decorator.args, 0) {
                    facts.length_max =
                        Some(facts.length_max.map_or(value, |current| current.min(value)));
                }
            }
            "length" | "lengthItems" => {
                if let Some(value) = integer_arg_value(&decorator.args, 0) {
                    facts.length_min = Some(value);
                    facts.length_max = Some(value);
                }
            }
            "notEmpty" | "notEmptyItems" => {
                facts.length_min = Some(facts.length_min.map_or(1, |current| current.max(1)));
            }
            "notNull" | "required" => facts.non_null = true,
            _ => {}
        }
    }
    facts
}

pub(super) fn type_narrowed_by_validator_facts(ty: &Type, facts: &ValidatorFacts) -> Type {
    if facts.non_null
        && let Type::Optional(inner) = transparent_type(ty)
    {
        return inner.as_ref().clone();
    }
    ty.clone()
}

pub(super) fn fact_condition_parts(
    op: &BinOp,
    left: &Expr,
    right: &Expr,
) -> Option<(String, FactKind, Expr, BinOp)> {
    if !matches!(
        op,
        BinOp::Eq | BinOp::Neq | BinOp::Gt | BinOp::Lt | BinOp::Gte | BinOp::Lte
    ) {
        return None;
    }
    if let Some((name, kind)) = fact_target(left)
        && is_fact_literal(right)
    {
        return Some((name, kind, right.clone(), op.clone()));
    }
    if let Some((name, kind)) = fact_target(right)
        && is_fact_literal(left)
    {
        return Some((name, kind, left.clone(), reverse_comparison_op(op)?));
    }
    None
}

pub(super) fn fact_target(expr: &Expr) -> Option<(String, FactKind)> {
    match expr.kind() {
        ExprKind::Ident(name) => Some((name.clone(), FactKind::Value)),
        ExprKind::FieldAccess { object, field } if field == "length" => match object.as_ref() {
            Expr {
                kind: ExprKind::Ident(name),
                ..
            } => Some((name.clone(), FactKind::Length)),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn is_fact_literal(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr {
            kind: ExprKind::StrLit(_),
            ..
        } | Expr {
            kind: ExprKind::IntLit(_),
            ..
        } | Expr {
            kind: ExprKind::UIntLit(_),
            ..
        } | Expr {
            kind: ExprKind::FloatLit(_),
            ..
        } | Expr {
            kind: ExprKind::BoolLit(_),
            ..
        } | Expr {
            kind: ExprKind::Null,
            ..
        } | Expr {
            kind: ExprKind::Neg(_),
            ..
        }
    )
}

pub(super) fn is_validator_literal(expr: &Expr) -> bool {
    match expr.kind() {
        ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::StrLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::RegexLit { .. } => true,
        ExprKind::ArrayLit(values) => values.iter().all(is_validator_literal),
        ExprKind::Neg(inner) => is_validator_literal(inner),
        _ => false,
    }
}

pub(super) fn reverse_comparison_op(op: &BinOp) -> Option<BinOp> {
    match op {
        BinOp::Eq => Some(BinOp::Eq),
        BinOp::Neq => Some(BinOp::Neq),
        BinOp::Gt => Some(BinOp::Lt),
        BinOp::Lt => Some(BinOp::Gt),
        BinOp::Gte => Some(BinOp::Lte),
        BinOp::Lte => Some(BinOp::Gte),
        _ => None,
    }
}

pub(super) fn evaluate_numeric_bounds(
    op: &BinOp,
    literal: f64,
    min: Option<f64>,
    max: Option<f64>,
) -> Option<FactCondition> {
    if let (Some(min), Some(max)) = (min, max)
        && min == max
    {
        return match op {
            BinOp::Eq if min == literal => Some(FactCondition::AlwaysTrue),
            BinOp::Eq => Some(FactCondition::AlwaysFalse),
            BinOp::Neq if min == literal => Some(FactCondition::AlwaysFalse),
            BinOp::Neq => Some(FactCondition::AlwaysTrue),
            _ => None,
        }
        .or_else(|| evaluate_numeric_order_bounds(op, literal, Some(min), Some(max)));
    }
    evaluate_numeric_order_bounds(op, literal, min, max)
}

pub(super) fn evaluate_numeric_order_bounds(
    op: &BinOp,
    literal: f64,
    min: Option<f64>,
    max: Option<f64>,
) -> Option<FactCondition> {
    match op {
        BinOp::Lt => {
            if min.is_some_and(|min| min >= literal) {
                Some(FactCondition::AlwaysFalse)
            } else if max.is_some_and(|max| max < literal) {
                Some(FactCondition::AlwaysTrue)
            } else {
                None
            }
        }
        BinOp::Lte => {
            if min.is_some_and(|min| min > literal) {
                Some(FactCondition::AlwaysFalse)
            } else if max.is_some_and(|max| max <= literal) {
                Some(FactCondition::AlwaysTrue)
            } else {
                None
            }
        }
        BinOp::Gt => {
            if max.is_some_and(|max| max <= literal) {
                Some(FactCondition::AlwaysFalse)
            } else if min.is_some_and(|min| min > literal) {
                Some(FactCondition::AlwaysTrue)
            } else {
                None
            }
        }
        BinOp::Gte => {
            if max.is_some_and(|max| max < literal) {
                Some(FactCondition::AlwaysFalse)
            } else if min.is_some_and(|min| min >= literal) {
                Some(FactCondition::AlwaysTrue)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(super) fn primitive_fact_literal(expr: Option<&Expr>) -> Option<Expr> {
    match expr?.kind() {
        ExprKind::StrLit(_)
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null => Some(expr?.clone()),
        ExprKind::Neg(inner) => match inner.as_ref() {
            Expr {
                kind: ExprKind::IntLit(value),
                ..
            } => Some(Expr::synthetic(ExprKind::IntLit(-value))),
            Expr {
                kind: ExprKind::FloatLit(value),
                ..
            } => Some(Expr::synthetic(ExprKind::FloatLit(
                FloatLiteral::from_value(-value.value()),
            ))),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn numeric_arg_value(args: &[Expr], index: usize) -> Option<f64> {
    numeric_literal_value(args.get(index)?)
}

pub(super) fn integer_arg_value(args: &[Expr], index: usize) -> Option<i64> {
    integer_literal_value(args.get(index)?)
}

pub(super) fn numeric_literal_value(expr: &Expr) -> Option<f64> {
    match expr.kind() {
        ExprKind::IntLit(value) => Some(*value as f64),
        ExprKind::UIntLit(value) => Some(*value as f64),
        ExprKind::FloatLit(value) => Some(value.value()),
        ExprKind::Neg(inner) => numeric_literal_value(inner).map(|value| -value),
        _ => None,
    }
}

pub(super) fn integer_literal_value(expr: &Expr) -> Option<i64> {
    match expr.kind() {
        ExprKind::IntLit(value) => i64::try_from(*value).ok(),
        ExprKind::UIntLit(value) => i64::try_from(*value).ok(),
        ExprKind::Neg(inner) => integer_literal_value(inner).map(|value| -value),
        _ => None,
    }
}

pub(super) fn array_literal_len(declared_ty: &Type, value: &Expr) -> Option<usize> {
    match value.kind() {
        ExprKind::ArrayLit(items)
            if matches!(transparent_type(declared_ty), Type::Array(_))
                && !items
                    .iter()
                    .any(|item| matches!(item.kind(), ExprKind::Spread(_))) =>
        {
            Some(items.len())
        }
        _ => None,
    }
}

pub(super) fn regex_match_result_len(object: &Expr, method: &str, args: &[Expr]) -> Option<usize> {
    match method {
        "match" => args.first().and_then(regex_literal_capture_len),
        "exec" => regex_literal_capture_len(object),
        _ => None,
    }
}

pub(super) fn regex_literal_capture_len(expr: &Expr) -> Option<usize> {
    let Expr {
        kind: ExprKind::RegexLit { pattern, flags },
        ..
    } = expr
    else {
        return None;
    };
    let pattern = crate::validators::regex_runtime_pattern(pattern, flags).ok()?;
    regex::Regex::new(&pattern)
        .ok()
        .map(|regex| regex.captures_len())
}

pub(super) const ARRAY_MUTATING_METHODS: &[&str] = &[
    "push",
    "pop",
    "shift",
    "unshift",
    "splice",
    "sort",
    "sortBy",
    "reverse",
    "rotate",
    "fill",
    "copyWithin",
];

pub fn is_array_mutating_method(method: &str) -> bool {
    ARRAY_MUTATING_METHODS.contains(&method)
}

pub fn is_mutating_method(method: &str) -> bool {
    is_array_mutating_method(method) || matches!(method, "set" | "add" | "delete" | "clear")
}

pub(super) const LENGTH_PRESERVING_RESULT_METHODS: &[&str] =
    &["toReversed", "toSorted", "with", "map", "shuffle"];

pub(super) fn array_method_len_delta(method: &str, args: &[Expr]) -> Option<i64> {
    match method {
        "push" | "unshift" => {
            if args
                .iter()
                .any(|arg| matches!(arg.kind(), ExprKind::Spread(_)))
            {
                None
            } else {
                Some(args.len() as i64)
            }
        }
        "pop" | "shift" => Some(-1),
        "sort" | "sortBy" | "reverse" | "rotate" | "fill" | "copyWithin" => Some(0),
        _ => None,
    }
}

pub(super) fn walk_block_mutations(body: &[Stmt], names: &mut HashSet<String>) {
    for statement in body {
        walk_stmt_mutations(statement, names);
    }
}

pub(super) fn walk_stmt_mutations(statement: &Stmt, names: &mut HashSet<String>) {
    match statement {
        Stmt {
            kind: StmtKind::Assign { name, value, .. },
            ..
        } => {
            names.insert(name.clone());
            walk_expr_mutations(value, names);
        }
        Stmt {
            kind: StmtKind::IndexAssign { index, value, .. },
            ..
        } => {
            walk_expr_mutations(index, names);
            walk_expr_mutations(value, names);
        }
        Stmt {
            kind: StmtKind::FieldAssign { object, value, .. },
            ..
        } => {
            walk_expr_mutations(object, names);
            walk_expr_mutations(value, names);
        }
        Stmt {
            kind: StmtKind::VarDecl { value, .. },
            ..
        } => {
            if let Some(value) = value {
                walk_expr_mutations(value, names);
            }
        }
        Stmt {
            kind: StmtKind::ArrayDestructure { value, .. },
            ..
        }
        | Stmt {
            kind: StmtKind::ObjectDestructure { value, .. },
            ..
        }
        | Stmt {
            kind: StmtKind::Return(Some(value)),
            ..
        }
        | Stmt {
            kind: StmtKind::Throw(value),
            ..
        }
        | Stmt {
            kind: StmtKind::Expr(value),
            ..
        } => walk_expr_mutations(value, names),
        Stmt {
            kind: StmtKind::While { cond, body },
            ..
        }
        | Stmt {
            kind: StmtKind::DoWhile { body, cond },
            ..
        } => {
            walk_expr_mutations(cond, names);
            walk_block_mutations(body, names);
        }
        Stmt {
            kind:
                StmtKind::For {
                    init,
                    cond,
                    update,
                    body,
                },
            ..
        } => {
            walk_stmt_mutations(init, names);
            walk_expr_mutations(cond, names);
            walk_stmt_mutations(update, names);
            walk_block_mutations(body, names);
        }
        Stmt {
            kind: StmtKind::ForOf { iterable, body, .. },
            ..
        } => {
            walk_expr_mutations(iterable, names);
            walk_block_mutations(body, names);
        }
        Stmt {
            kind: StmtKind::ForIn { object, body, .. },
            ..
        } => {
            walk_expr_mutations(object, names);
            walk_block_mutations(body, names);
        }
        Stmt {
            kind:
                StmtKind::TryCatch {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                },
            ..
        } => {
            walk_block_mutations(try_body, names);
            walk_block_mutations(catch_body, names);
            if let Some(finally_body) = finally_body {
                walk_block_mutations(finally_body, names);
            }
        }
        Stmt {
            kind:
                StmtKind::Switch {
                    expr,
                    cases,
                    default,
                    ..
                },
            ..
        } => {
            walk_expr_mutations(expr, names);
            for case in cases {
                walk_block_mutations(&case.body, names);
            }
            if let Some(default) = default {
                walk_block_mutations(default, names);
            }
        }
        Stmt {
            kind:
                StmtKind::Match {
                    expr,
                    arms,
                    default,
                },
            ..
        } => {
            walk_expr_mutations(expr, names);
            for arm in arms {
                walk_block_mutations(&arm.body, names);
            }
            if let Some(default) = default {
                walk_block_mutations(default, names);
            }
        }
        Stmt {
            kind: StmtKind::Defer { body, .. },
            ..
        } => walk_block_mutations(body, names),
        Stmt {
            kind: StmtKind::Return(None),
            ..
        }
        | Stmt {
            kind: StmtKind::Continue,
            ..
        } => {}
        Stmt {
            kind: StmtKind::Break(value),
            ..
        } => {
            if let Some(value) = value {
                walk_expr_mutations(value, names);
            }
        }
    }
}

pub(super) fn walk_expr_mutations(expr: &Expr, names: &mut HashSet<String>) {
    match expr.kind() {
        ExprKind::CallValue { callee, args, .. } => {
            walk_expr_mutations(callee, names);
            for arg in args {
                walk_expr_mutations(arg, names);
            }
        }
        ExprKind::MethodCall {
            object,
            method,
            args,
            ..
        }
        | ExprKind::OptionalMethodCall {
            object,
            method,
            args,
            ..
        } => {
            if ARRAY_MUTATING_METHODS.contains(&method.as_str())
                && let Some(name) = crate::ast::place_root_ident(object)
            {
                names.insert(name.to_string());
            }
            walk_expr_mutations(object, names);
            args.iter().for_each(|arg| walk_expr_mutations(arg, names));
        }
        ExprKind::Borrow {
            mutable: true,
            name,
            ..
        } => {
            names.insert(name.clone());
        }
        ExprKind::BinOp { left, right, .. } => {
            walk_expr_mutations(left, names);
            walk_expr_mutations(right, names);
        }
        ExprKind::Pipeline { left, right } => {
            walk_expr_mutations(left, names);
            walk_expr_mutations(right, names);
        }
        ExprKind::ChainedComparison { terms, .. } => {
            terms
                .iter()
                .for_each(|term| walk_expr_mutations(term, names));
        }
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
        | ExprKind::PostfixDecr(inner) => walk_expr_mutations(inner, names),
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            walk_expr_mutations(object, names);
        }
        ExprKind::IndexAccess { object, index } => {
            walk_expr_mutations(object, names);
            walk_expr_mutations(index, names);
        }
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            walk_expr_mutations(cond, names);
            walk_expr_mutations(then_expr, names);
            walk_expr_mutations(else_expr, names);
        }
        ExprKind::TypeAssert { value, .. } => walk_expr_mutations(value, names),
        ExprKind::Is { expr, .. } => walk_expr_mutations(expr, names),
        ExprKind::ArrayLit(values) => values
            .iter()
            .for_each(|value| walk_expr_mutations(value, names)),
        ExprKind::ArrayRepeat { value, count } => {
            walk_expr_mutations(value, names);
            walk_expr_mutations(count, names);
        }
        ExprKind::ObjectLit(fields) => fields
            .iter()
            .for_each(|(_, value)| walk_expr_mutations(value, names)),
        ExprKind::ObjectUpdate { base, fields } => {
            walk_expr_mutations(base, names);
            fields
                .iter()
                .for_each(|(_, value)| walk_expr_mutations(value, names));
        }
        ExprKind::FnCall { args, .. } => {
            args.iter().for_each(|arg| walk_expr_mutations(arg, names));
        }
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            if let Some(initial) = initial {
                walk_expr_mutations(initial, names);
            }
        }
        ExprKind::ArrowFn { body, .. } => walk_block_mutations(body, names),
        ExprKind::If {
            cond,
            then_block,
            else_block,
        } => {
            walk_expr_mutations(cond, names);
            walk_block_mutations(then_block, names);
            if let Some(else_block) = else_block {
                walk_block_mutations(else_block, names);
            }
        }
        ExprKind::Loop { body } => walk_block_mutations(body, names),
        ExprKind::In { key, object } => {
            walk_expr_mutations(key, names);
            walk_expr_mutations(object, names);
        }
        ExprKind::Range { start, end, .. } => {
            walk_expr_mutations(start, names);
            walk_expr_mutations(end, names);
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let TemplatePart::Expr(expr, _) = part {
                    walk_expr_mutations(expr, names);
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
        | ExprKind::Topic
        | ExprKind::Ident(_)
        | ExprKind::Borrow { .. } => {}
    }
}

pub(super) fn expression_subject_name(context: &str) -> Option<&str> {
    context.split_whitespace().last()
}

pub(super) fn optional_value_type(value: ValueType, optional: bool) -> ValueType {
    if !optional {
        return value;
    }
    match value {
        ValueType::Known(Type::Optional(inner)) => ValueType::Known(Type::Optional(inner)),
        ValueType::Known(ty) => ValueType::Known(Type::Optional(Box::new(ty))),
        other => other,
    }
}

pub(super) fn is_same_value_type(a: &ValueType, b: &ValueType) -> bool {
    match (a, b) {
        (ValueType::Error, _) | (_, ValueType::Error) => true,
        (ValueType::Unknown, _) | (_, ValueType::Unknown) => true,
        _ => a.display() == b.display(),
    }
}

pub(super) fn tuple_index_type(
    elements: &[Type],
    index_expr: &Expr,
    checker: &mut TypeChecker,
) -> ValueType {
    let Expr {
        kind: ExprKind::IntLit(index),
        ..
    } = index_expr
    else {
        checker.error(
            "E2493",
            "tuple index must be an integer literal".to_string(),
            expression_subject(index_expr),
        );
        return ValueType::Error;
    };
    if *index < 0 {
        checker.error(
            "E2493",
            format!("tuple index `{index}` is out of bounds"),
            expression_subject(index_expr),
        );
        return ValueType::Error;
    }
    elements
        .get(*index as usize)
        .cloned()
        .map(ValueType::Known)
        .unwrap_or_else(|| {
            checker.error(
                "E2493",
                format!(
                    "tuple type of length {} has no element at index {index}",
                    elements.len()
                ),
                expression_subject(index_expr),
            );
            ValueType::Error
        })
}

pub(super) fn null_narrowing(condition: &Expr) -> Option<(String, bool)> {
    if let Some(path) = expression_path(condition) {
        return Some((path, true));
    }
    let Expr {
        kind: ExprKind::BinOp {
            op, left, right, ..
        },
        ..
    } = condition
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
                kind: ExprKind::Null,
                ..
            },
        )
        | (
            Expr {
                kind: ExprKind::Null,
                ..
            },
            Expr {
                kind: ExprKind::Ident(name),
                ..
            },
        ) => name.clone(),
        _ => return None,
    };
    match op {
        BinOp::Neq => Some((name, true)),
        BinOp::Eq => Some((name, false)),
        _ => None,
    }
}

pub(super) fn result_narrowing(condition: &Expr) -> Option<ResultNarrowing> {
    match condition.kind() {
        ExprKind::Ident(name) => Some(ResultNarrowing {
            name: name.clone(),
            success_in_then: true,
        }),
        ExprKind::Not(inner) => match inner.kind() {
            ExprKind::Ident(name) => Some(ResultNarrowing {
                name: name.clone(),
                success_in_then: false,
            }),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn typeof_narrowing(condition: &Expr) -> Option<(String, String, bool)> {
    let Expr {
        kind: ExprKind::BinOp {
            op, left, right, ..
        },
        ..
    } = condition
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
        ) => (inner, name),
        (
            Expr {
                kind: ExprKind::StrLit(name),
                ..
            },
            Expr {
                kind: ExprKind::Typeof(inner),
                ..
            },
        ) => (inner, name),
        _ => return None,
    };
    let var_name = match typeof_expr.as_ref().kind() {
        ExprKind::Ident(name) => name.clone(),
        _ => return None,
    };
    match op {
        BinOp::Eq => Some((var_name, type_name.clone(), true)),
        BinOp::Neq => Some((var_name, type_name.clone(), false)),
        _ => None,
    }
}

pub(super) fn is_narrowing(condition: &Expr) -> Option<(String, Type, bool)> {
    match condition.kind() {
        ExprKind::Is { expr, ty } => {
            let var_name = match expr.kind() {
                ExprKind::Ident(name) => name.clone(),
                _ => return None,
            };
            Some((var_name, ty.clone(), true))
        }
        ExprKind::Not(inner) => {
            if let ExprKind::Is { expr, ty } = inner.kind() {
                let var_name = match expr.kind() {
                    ExprKind::Ident(name) => name.clone(),
                    _ => return None,
                };
                Some((var_name, ty.clone(), false))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(super) fn narrowed_is_type(
    current: &Type,
    target: &Type,
    in_then_branch: bool,
) -> Option<Type> {
    match current {
        Type::Union(variants) => {
            if in_then_branch {
                let narrowed = variants
                    .iter()
                    .filter(|v| types_intersect(v, target))
                    .cloned()
                    .collect::<Vec<_>>();
                type_from_narrowed_variants(narrowed)
            } else {
                let narrowed = variants
                    .iter()
                    .filter(|v| !types_intersect(v, target))
                    .cloned()
                    .collect::<Vec<_>>();
                type_from_narrowed_variants(narrowed)
            }
        }
        ty if in_then_branch && types_intersect(ty, target) => Some(ty.clone()),
        ty if !in_then_branch && !types_intersect(ty, target) => Some(ty.clone()),
        _ => None,
    }
}

pub(super) fn types_intersect(a: &Type, b: &Type) -> bool {
    match (transparent_type(a), transparent_type(b)) {
        (Type::Union(variants), _) => variants.iter().any(|v| types_intersect(v, b)),
        (_, Type::Union(variants)) => variants.iter().any(|v| types_intersect(a, v)),
        (Type::Never, _) | (_, Type::Never) => false,
        (Type::Unknown, _) | (_, Type::Unknown) => true,
        (Type::Object(_), Type::Object(_)) => true,
        (Type::Array(_), Type::Array(_)) => true,
        (Type::Map(_, _), Type::Map(_, _)) => true,
        (Type::Set(_), Type::Set(_)) => true,
        (Type::Result { .. }, Type::Result { .. }) => true,
        (Type::Named(a_name), Type::Named(b_name)) => a_name == b_name,
        _ => {
            std::mem::discriminant(transparent_type(a))
                == std::mem::discriminant(transparent_type(b))
        }
    }
}

pub(super) fn discriminant_narrowing(condition: &Expr) -> Option<DiscriminantNarrowing> {
    let Expr {
        kind: ExprKind::BinOp {
            op, left, right, ..
        },
        ..
    } = condition
    else {
        return None;
    };
    let (object, field, literal) = match (left.as_ref(), right.as_ref()) {
        (
            Expr {
                kind: ExprKind::FieldAccess { object, field },
                ..
            },
            literal,
        ) if is_narrowing_literal(literal) => (object, field, literal),
        (
            literal,
            Expr {
                kind: ExprKind::FieldAccess { object, field },
                ..
            },
        ) if is_narrowing_literal(literal) => (object, field, literal),
        _ => return None,
    };
    let name = match object.as_ref().kind() {
        ExprKind::Ident(name) => name.clone(),
        _ => return None,
    };
    match op {
        BinOp::Eq => Some(DiscriminantNarrowing {
            name,
            field: field.clone(),
            literal: literal.clone(),
            matches_in_then: true,
        }),
        BinOp::Neq => Some(DiscriminantNarrowing {
            name,
            field: field.clone(),
            literal: literal.clone(),
            matches_in_then: false,
        }),
        _ => None,
    }
}

pub(super) fn is_narrowing_literal(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr {
            kind: ExprKind::StrLit(_),
            ..
        } | Expr {
            kind: ExprKind::IntLit(_),
            ..
        } | Expr {
            kind: ExprKind::FloatLit(_),
            ..
        } | Expr {
            kind: ExprKind::BoolLit(_),
            ..
        } | Expr {
            kind: ExprKind::Null,
            ..
        } | Expr {
            kind: ExprKind::None,
            ..
        }
    )
}

pub(super) fn narrowed_typeof_type(
    current: &Type,
    type_name: &str,
    type_in_branch: bool,
) -> Option<Type> {
    match transparent_type(current) {
        Type::Union(variants) => {
            let narrowed = variants
                .iter()
                .filter(|variant| type_matches_typeof_name(variant, type_name) == type_in_branch)
                .cloned()
                .collect::<Vec<_>>();
            type_from_narrowed_variants(narrowed)
        }
        Type::Unknown if type_in_branch => crate::typeof_category::typeof_category_type(type_name),
        Type::Unknown => None,
        ty if type_in_branch && type_matches_typeof_name(ty, type_name) => Some(ty.clone()),
        _ => None,
    }
}

pub(super) fn variant_discriminant_matches(variant: &Type, field: &str, literal: &Expr) -> bool {
    let Type::Object(fields) = transparent_type(variant) else {
        return false;
    };
    let Some(field_ty) = fields
        .iter()
        .find(|candidate| candidate.name == field)
        .map(field_value_type)
    else {
        return false;
    };
    matches!(transparent_type(&field_ty), Type::Lit(expected) if literal_expr_matches(expected, literal))
}

pub(super) fn match_pattern_variant_indices(
    pattern: &MatchPattern,
    variants: &[Type],
) -> Vec<usize> {
    match pattern {
        MatchPattern::Wildcard | MatchPattern::Binding(_) => (0..variants.len()).collect(),
        MatchPattern::ResultVariant { success, .. } => vec![usize::from(!success)],
        MatchPattern::Lit(literal) => variants
            .iter()
            .enumerate()
            .filter_map(|(index, variant)| match_variant_literal(variant, literal).then_some(index))
            .collect(),
        MatchPattern::Object(fields) => variants
            .iter()
            .enumerate()
            .filter_map(|(index, variant)| {
                match_variant_object_fields(variant, fields).then_some(index)
            })
            .collect(),
        MatchPattern::Or(patterns) => {
            let mut indices = Vec::new();
            for pattern in patterns {
                for index in match_pattern_variant_indices(pattern, variants) {
                    if !indices.contains(&index) {
                        indices.push(index);
                    }
                }
            }
            indices
        }
        MatchPattern::Array { .. } | MatchPattern::Tuple(_) | MatchPattern::Range { .. } => {
            Vec::new()
        }
        MatchPattern::EnumVariant {
            enum_name, variant, ..
        } => {
            let full_name = format!("{enum_name}.{variant}");
            variants
                .iter()
                .enumerate()
                .filter_map(|(index, v)| {
                    if matches!(v, Type::Named(name) if name == &full_name) {
                        Some(index)
                    } else {
                        None
                    }
                })
                .collect()
        }
    }
}

pub(super) fn enum_payload_binding_list(count: usize) -> String {
    (0..count)
        .map(|index| {
            let letter = char::from(b'a' + (index % 26) as u8);
            let round = index / 26;
            if round == 0 {
                letter.to_string()
            } else {
                format!("{letter}{round}")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn match_variant_literal(variant: &Type, literal: &Expr) -> bool {
    match transparent_type(variant) {
        Type::Lit(expected) => literal_expr_matches(expected, literal),
        Type::Str => matches!(literal.kind(), ExprKind::StrLit(_)),
        Type::Regex => matches!(literal.kind(), ExprKind::RegexLit { .. }),
        Type::Bool => matches!(literal.kind(), ExprKind::BoolLit(_)),
        Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::Isize
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64 => {
            matches!(literal.kind(), ExprKind::IntLit(_))
        }
        Type::U128 | Type::Usize => {
            matches!(literal.kind(), ExprKind::IntLit(_) | ExprKind::UIntLit(_))
        }
        Type::F32 | Type::F64 | Type::F128 => matches!(literal.kind(), ExprKind::FloatLit(_)),
        Type::Never => matches!(literal.kind(), ExprKind::Null | ExprKind::None),
        Type::Optional(_) => matches!(literal.kind(), ExprKind::Null | ExprKind::None),
        _ => false,
    }
}

pub(super) fn match_variant_object_fields(
    variant: &Type,
    pattern_fields: &[crate::ast::ObjectMatchField],
) -> bool {
    let Type::Object(fields) = transparent_type(variant) else {
        return false;
    };
    pattern_fields.iter().all(|pattern_field| {
        let Some(field) = fields
            .iter()
            .find(|candidate| candidate.name == pattern_field.key)
        else {
            return false;
        };
        pattern_field.literal.as_ref().is_none_or(|literal| {
            matches!(transparent_type(&field_value_type(field)), Type::Lit(expected) if literal_expr_matches(expected, literal))
        })
    })
}

pub(super) fn match_pattern_subject(pattern: &MatchPattern) -> Option<String> {
    match pattern {
        MatchPattern::Binding(name) => Some(name.clone()),
        MatchPattern::ResultVariant { success, binding } => binding
            .clone()
            .or_else(|| Some(if *success { "Ok" } else { "Err" }.to_string())),
        MatchPattern::Object(fields) => fields
            .iter()
            .find_map(|field| field.binding.clone().or_else(|| Some(field.key.clone()))),
        MatchPattern::Lit(Expr {
            kind: ExprKind::StrLit(value),
            ..
        }) => Some(value.clone()),
        MatchPattern::Lit(Expr {
            kind: ExprKind::IntLit(value),
            ..
        }) => Some(value.to_string()),
        MatchPattern::Lit(Expr {
            kind: ExprKind::UIntLit(value),
            ..
        }) => Some(value.to_string()),
        MatchPattern::Lit(Expr {
            kind: ExprKind::FloatLit(value),
            ..
        }) => Some(value.text().to_string()),
        MatchPattern::Lit(Expr {
            kind: ExprKind::BoolLit(value),
            ..
        }) => Some(value.to_string()),
        MatchPattern::Lit(Expr {
            kind: ExprKind::Null,
            ..
        }) => Some("null".to_string()),
        MatchPattern::Lit(Expr {
            kind: ExprKind::None,
            ..
        }) => Some("None".to_string()),
        MatchPattern::Wildcard => Some("_".to_string()),
        MatchPattern::Array { .. } | MatchPattern::Or(_) | MatchPattern::Tuple(_) => None,
        MatchPattern::EnumVariant { variant, .. } => Some(variant.clone()),
        MatchPattern::Lit(_) => None,
        MatchPattern::Range {
            start,
            end,
            inclusive,
        } => {
            let start = start
                .as_ref()
                .map(crate::ast::expr_display)
                .unwrap_or_default();
            let op = if *inclusive { "..=" } else { ".." };
            let end = end
                .as_ref()
                .map(crate::ast::expr_display)
                .unwrap_or_default();
            Some(format!("{start}{op}{end}"))
        }
    }
}

pub(super) fn type_from_narrowed_variants(variants: Vec<Type>) -> Option<Type> {
    match variants.len() {
        0 => Some(Type::Never),
        1 => variants.into_iter().next(),
        _ => Some(Type::Union(variants)),
    }
}

pub(super) fn type_matches_typeof_name(ty: &Type, type_name: &str) -> bool {
    crate::typeof_name::matches_typeof_name(transparent_type(ty), type_name)
}

pub(super) fn block_always_returns(body: &[Stmt], exhaustive: &HashSet<Span>) -> bool {
    body.last()
        .is_some_and(|statement| stmt_always_returns(statement, exhaustive))
}

pub(super) fn stmt_always_returns(statement: &Stmt, exhaustive: &HashSet<Span>) -> bool {
    match statement.kind() {
        StmtKind::Return(_) | StmtKind::Throw(_) => true,
        StmtKind::Expr(Expr {
            kind: ExprKind::MethodCall { object, method, .. },
            ..
        }) => {
            method == "exit" && matches!(object.kind(), ExprKind::Ident(name) if name == "process")
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
            block_always_returns(then_block, exhaustive)
                && block_always_returns(else_block, exhaustive)
        }
        StmtKind::While { cond, body } => {
            matches!(cond.kind(), ExprKind::BoolLit(true)) && !block_has_loop_break(body)
        }
        StmtKind::Expr(Expr {
            kind: ExprKind::Loop { body },
            ..
        }) => !block_has_loop_break(body),
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            finally_body
                .as_deref()
                .is_some_and(|body| block_always_returns(body, exhaustive))
                || (block_always_returns(try_body, exhaustive)
                    && block_always_returns(catch_body, exhaustive))
        }
        StmtKind::Switch { cases, default, .. } => {
            default
                .as_deref()
                .is_some_and(|body| block_always_returns(body, exhaustive))
                && cases
                    .iter()
                    .all(|case| block_always_returns(&case.body, exhaustive))
        }
        StmtKind::Match { arms, default, .. } => {
            let catches_everything = default.is_some()
                || arms.iter().any(|arm| {
                    arm.guard.is_none()
                        && matches!(
                            arm.pattern,
                            MatchPattern::Wildcard | MatchPattern::Binding(_)
                        )
                });
            (catches_everything || exhaustive.contains(&statement.span))
                && default
                    .as_deref()
                    .is_none_or(|body| block_always_returns(body, exhaustive))
                && arms
                    .iter()
                    .all(|arm| block_always_returns(&arm.body, exhaustive))
        }
        _ => false,
    }
}

pub(super) fn block_has_loop_break(body: &[Stmt]) -> bool {
    body.iter().any(stmt_has_loop_break)
}

pub(super) fn stmt_has_loop_break(statement: &Stmt) -> bool {
    match statement.kind() {
        StmtKind::Break(_) => true,
        StmtKind::Expr(Expr {
            kind:
                ExprKind::If {
                    then_block,
                    else_block,
                    ..
                },
            ..
        }) => {
            block_has_loop_break(then_block)
                || else_block
                    .as_ref()
                    .is_some_and(|block| block_has_loop_break(block))
        }
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            block_has_loop_break(try_body)
                || block_has_loop_break(catch_body)
                || finally_body
                    .as_ref()
                    .is_some_and(|block| block_has_loop_break(block))
        }
        _ => false,
    }
}
