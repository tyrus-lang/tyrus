use super::*;

impl BorrowChecker {
    #[allow(clippy::too_many_lines)]
    pub(super) fn check_expr_read(&mut self, expr: &Expr) {
        match expr.kind() {
            ExprKind::ObjectLit(fields) => {
                for (_, value) in fields {
                    self.check_expr_read(value);
                }
            }
            ExprKind::ObjectUpdate { base, fields } => {
                self.check_expr_read(base);
                for (_, value) in fields {
                    self.check_expr_read(value);
                }
            }
            ExprKind::ArrayLit(values) => {
                for value in values {
                    self.check_expr_read(value);
                }
            }
            ExprKind::ArrayRepeat { value, count } => {
                self.check_expr_read(value);
                self.check_expr_read(count);
            }
            ExprKind::FieldAccess { object, field } => {
                if matches!(
                    object.as_ref(),
                    Expr { kind: ExprKind::Ident(name), .. }
                        if matches!(
                            name.as_str(),
                            "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "f32" | "f64" | "f128" | "decimal"
                        ) && matches!(field.as_str(), "MIN" | "MAX")
                ) {
                    return;
                }
                if matches!(
                    object.as_ref(),
                    Expr { kind: ExprKind::Ident(name), .. } if name == "process" && self.lookup(name).is_none()
                ) {
                    return;
                }
                let read = Expr::synthetic(ExprKind::FieldAccess {
                    object: object.clone(),
                    field: field.clone(),
                });
                if let Some((root, path)) = field_move_path(&read)
                    && let Some(var) = self.lookup(&root)
                {
                    let moved = var
                        .moved_fields
                        .iter()
                        .find(|moved| paths_overlap(moved, &path))
                        .cloned();
                    if let Some(moved) = moved {
                        self.error(format!("error: use of moved value: `{root}.{moved}`"));
                    } else {
                        self.check_binding_state(&root);
                    }
                    return;
                }
                self.check_expr_read(object);
            }
            ExprKind::OptionalFieldAccess { object, .. } => {
                self.check_expr_read(object);
            }
            ExprKind::IndexAccess { object, index } => {
                self.check_expr_read(object);
                self.check_expr_read(index);
            }
            ExprKind::Ident(name) => match self.lookup(name) {
                None if !self.is_external(name) => {
                    self.error(format!("error: use of undeclared variable `{name}`"));
                }
                Some(v) if v.availability == Availability::Moved => {
                    self.error(format!("error: use of moved value: `{name}`"));
                }
                Some(v) if v.availability == Availability::Dropped => {
                    self.error(format!("error: use of dropped value: `{name}`"));
                }
                Some(v) if !v.moved_fields.is_empty() => self.error(format!(
                    "error: use of moved value: `{name}` (its field `{}` was moved out)",
                    v.moved_fields[0]
                )),
                Some(v) if v.borrow == BorrowState::Mutable => self.error(format!(
                    "error: cannot use `{name}` while it is mutably borrowed"
                )),
                _ => {}
            },

            ExprKind::Borrow {
                mutable,
                name,
                place,
            } => {
                if let Some(var) = self.lookup(name) {
                    let borrowed = place
                        .as_deref()
                        .and_then(field_move_path)
                        .map(|(_, path)| path);
                    let moved = var
                        .moved_fields
                        .iter()
                        .find(|moved| match &borrowed {
                            Some(path) => paths_overlap(moved, path),
                            None => true,
                        })
                        .cloned();
                    if let Some(moved) = moved {
                        self.error(format!("error: use of moved value: `{name}.{moved}`"));
                        return;
                    }
                }
                let state = self.lookup(name).map(|v| {
                    (
                        v.availability == Availability::Available,
                        v.mutable,
                        v.borrow.clone(),
                    )
                });

                match state {
                    None if !self.is_external(name) => {
                        self.error(format!("error: cannot borrow undeclared variable `{name}`"));
                    }
                    None => {}
                    Some((owns, var_mutable, borrow)) => {
                        if !owns {
                            self.error(format!("error: cannot borrow moved value `{name}`"));
                            return;
                        }
                        if *mutable {
                            if !var_mutable {
                                self.error(format!(
                                    "error: cannot borrow `{name}` as mutable because it is declared `const`"
                                ));
                            }
                            if borrow != BorrowState::None {
                                self.error(format!(
                                    "error: cannot borrow `{name}` as mutable because it is already borrowed"
                                ));
                            } else if let Some(v) = self.lookup_mut(name) {
                                v.borrow = BorrowState::Mutable;
                            }
                        } else if borrow == BorrowState::Mutable {
                            self.error(format!(
                                "error: cannot borrow `{name}` as immutable because it is already mutably borrowed"
                            ));
                        } else {
                            let count = match &borrow {
                                BorrowState::Immutable(n) => n + 1,
                                _ => 1,
                            };
                            if let Some(v) = self.lookup_mut(name) {
                                v.borrow = BorrowState::Immutable(count);
                            }
                        }
                    }
                }
            }

            ExprKind::Unwrap(inner) => {
                self.check_expr_read(inner);
            }

            ExprKind::BinOp { left, right, .. } => {
                self.check_expr_read(left);
                self.check_expr_read(right);
            }

            ExprKind::ChainedComparison { terms, .. } => {
                for term in terms {
                    self.check_expr_read(term);
                }
            }

            ExprKind::Pipeline { left, right } => {
                if contains_outer_topic(right) {
                    self.check_expr_read(left);
                    self.check_expr_read(right);
                } else {
                    match desugar_pipeline(left, right) {
                        Some(call) => self.check_expr_read(&call),
                        None => {
                            self.check_expr_read(left);
                            self.check_expr_read(right);
                        }
                    }
                }
            }

            ExprKind::Not(inner)
            | ExprKind::Neg(inner)
            | ExprKind::BitNot(inner)
            | ExprKind::Typeof(inner)
            | ExprKind::Spread(inner)
            | ExprKind::Is { expr: inner, .. } => self.check_expr_read(inner),

            ExprKind::PrefixIncr(inner)
            | ExprKind::PrefixDecr(inner)
            | ExprKind::PostfixIncr(inner)
            | ExprKind::PostfixDecr(inner) => {
                if let ExprKind::Ident(name) = inner.as_ref().kind() {
                    match self.lookup(name) {
                        None => self.error(format!("error: use of undeclared variable `{name}`")),
                        Some(v) if !v.mutable => {
                            self.error(format!("error: cannot mutate immutable variable `{name}`"));
                        }
                        Some(v) if v.borrow != BorrowState::None => self.error(format!(
                            "error: cannot mutate `{name}` while it is borrowed"
                        )),
                        _ => {}
                    }
                } else {
                    self.check_expr_read(inner);
                }
            }

            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.check_expr_read(cond);
                self.check_expr_read(then_expr);
                self.check_expr_read(else_expr);
            }

            ExprKind::TemplateLit(parts) => {
                for part in parts {
                    if let TemplatePart::Expr(expr, _) = part {
                        self.check_expr_read(expr);
                    }
                }
            }

            ExprKind::FnCall { callee, args, .. } => {
                if let Some(var) = self.lookup(callee) {
                    if var.availability == Availability::Moved {
                        self.error(format!("error: use of moved value: `{callee}`"));
                    } else if var.availability == Availability::Dropped {
                        self.error(format!("error: use of dropped value: `{callee}`"));
                    } else if !matches!(var.ty, Type::Function { .. } | Type::Throws { .. }) {
                        self.error(format!("error: `{callee}` is not callable"));
                    }
                }
                for arg in args {
                    self.check_expr_read(arg);
                }
                if matches!(
                    callee.as_str(),
                    crate::memory::ZEROIZE_TARGET | crate::memory::DROP_TARGET
                ) || self.consuming_imports.contains(callee)
                {
                    for arg in args {
                        if let ExprKind::Ident(name) = arg.kind() {
                            if self.is_captured_by_pending_finalizer(name) {
                                self.error(format!(
                                    "error: cannot consume `{name}` because it is captured by a pending finalizer"
                                ));
                                continue;
                            }
                            if let Some(var) = self.lookup_mut(name) {
                                var.availability = Availability::Dropped;
                            }
                        }
                    }
                }
                if self.replacing_imports.contains(callee)
                    && let Some(Expr {
                        kind: ExprKind::Ident(name),
                        ..
                    }) = args.get(1)
                {
                    if self.is_captured_by_pending_finalizer(name) {
                        self.error(format!(
                                "error: cannot move `{name}` because it is captured by a pending finalizer"
                            ));
                        return;
                    }
                    if let Some(var) = self.lookup_mut(name)
                        && matches!(var.ty, Type::Str)
                    {
                        var.availability = Availability::Moved;
                    }
                }
            }
            ExprKind::CallValue { callee, args, .. } => {
                self.check_expr_read(callee);
                for arg in args {
                    self.check_expr_read(arg);
                }
            }

            ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
                if let Some(initial) = initial {
                    self.check_expr_read(initial);
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
                let builtin_namespace = matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "console")
                    || (matches!(method.as_str(), "isArray" | "of" | "from" | "withCapacity")
                        && matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "Array")
                        && self.lookup("Array").is_none())
                    || (matches!(
                        method.as_str(),
                        "resolve"
                            | "reject"
                            | "all"
                            | "allSettled"
                            | "any"
                            | "race"
                            | "withResolvers"
                    ) && matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "Promise")
                        && self.lookup("Promise").is_none())
                    || (matches!(
                        method.as_str(),
                        "exit" | "cwd" | "uptime" | "memoryUsage" | "cpuUsage"
                    ) && matches!(object.as_ref().kind(), ExprKind::Ident(name) if name == "process")
                        && self.lookup("process").is_none())
                    || (method == "fromJson"
                        && matches!(object.as_ref().kind(), ExprKind::Ident(name) if self.data_interface_names.contains(name)));
                if !builtin_namespace {
                    self.check_expr_read(object);
                }
                if matches!(
                    method.as_str(),
                    "count"
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
                ) {
                    if let Some(arg) = args.first() {
                        let free: Vec<String> = expr_referenced_identifiers(arg)
                            .into_iter()
                            .filter(|name| self.lookup(name).is_none())
                            .collect();
                        self.push_scope();
                        for name in &free {
                            self.declare(name, false, Type::Infer);
                        }
                        self.check_expr_read(arg);
                        self.pop_scope();
                    }
                    return;
                }
                if self.is_http_router_route_call(object, method) {
                    if let Some(path) = args.first() {
                        self.check_expr_read(path);
                    }
                    if let Some(Expr {
                        kind: ExprKind::ObjectLit(fields),
                        ..
                    }) = args.get(1)
                    {
                        if let Some((_, handler)) =
                            fields.iter().find(|(field, _)| field == "handler")
                        {
                            self.check_expr_read(handler);
                        }
                    } else {
                        for arg in args.iter().skip(1) {
                            self.check_expr_read(arg);
                        }
                    }
                    return;
                }
                let sync_callback = is_sync_array_method(method) && !args.is_empty();
                let prev_flag = self.in_non_escaping_callback;
                if sync_callback {
                    self.in_non_escaping_callback = true;
                }
                if let Some(first) = args.first() {
                    self.check_expr_read(first);
                }
                if sync_callback {
                    self.in_non_escaping_callback = prev_flag;
                }
                for arg in args.iter().skip(1) {
                    self.check_expr_read(arg);
                }
            }

            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => {
                self.check_expr_read(cond);
                let scopes_before_branches = self.scopes.clone();
                let null_narrow = Self::null_narrowing(cond);
                let typeof_narrow = Self::typeof_narrowing(cond);

                self.scopes.clone_from(&scopes_before_branches);
                self.push_scope();
                self.apply_null_narrowing(null_narrow, true);
                self.apply_typeof_narrowing(typeof_narrow.as_ref(), true);
                self.check_stmt_sequence(then_block);
                self.pop_scope();
                let then_scopes = self.scopes.clone();

                let else_scopes = if let Some(else_stmts) = else_block {
                    self.scopes.clone_from(&scopes_before_branches);
                    self.push_scope();
                    self.apply_null_narrowing(null_narrow, false);
                    self.apply_typeof_narrowing(typeof_narrow.as_ref(), false);
                    self.check_stmt_sequence(else_stmts);
                    self.pop_scope();
                    self.scopes.clone()
                } else {
                    scopes_before_branches.clone()
                };

                let then_reaches_next_statement = Self::block_reaches_next_statement(then_block);
                let else_reaches_next_statement = else_block
                    .as_ref()
                    .is_none_or(|stmts| Self::block_reaches_next_statement(stmts));
                self.scopes = Self::merge_reachable_branch_scopes(
                    &scopes_before_branches,
                    &[
                        (then_scopes, then_reaches_next_statement),
                        (else_scopes, else_reaches_next_statement),
                    ],
                );
                self.refresh_persistent_borrows();
            }

            ExprKind::Loop { body } => {
                let before = self.outer_move_state();
                self.push_scope();
                self.check_stmt_sequence(body);
                self.pop_scope();
                self.report_moves_repeated_by_loop(&before, body);
            }

            ExprKind::ArrowFn {
                is_async,
                params,
                body,
                ..
            } => {
                let is_escaping = !self.in_non_escaping_callback;
                self.in_non_escaping_callback = false;
                let previous_async_depth = self.async_depth;
                self.async_depth = if *is_async { 1 } else { 0 };

                let capture_names = closure_capture_names(params, body);
                let captures: Vec<(String, Type)> = capture_names
                    .iter()
                    .filter_map(|name| self.lookup(name).map(|v| (name.clone(), v.ty.clone())))
                    .collect();

                if is_escaping {
                    for (name, ty) in &captures {
                        if let Some(v) = self.lookup(name) {
                            if v.availability != Availability::Available
                                || !v.moved_fields.is_empty()
                            {
                                self.error(format!(
                                    "error: use of moved value: `{name}` (captured by closure)"
                                ));
                            } else if v.borrow == BorrowState::Mutable && !ty.is_copy() {
                                self.error(format!(
                                    "error: cannot move `{name}` into closure while it is mutably borrowed"
                                ));
                            }
                        }
                    }
                }

                self.push_scope();

                if is_escaping {
                    for (name, ty) in &captures {
                        if !ty.is_copy() {
                            self.declare(name, false, ty.clone());
                        }
                    }
                }

                for p in params {
                    match &p.pattern {
                        ParamPattern::Name(name) => {
                            self.declare_parameter(name, p.mutable, p.ty.clone());
                        }
                        ParamPattern::Array(names) => {
                            let element_ty = if let Type::Array(element) = &p.ty {
                                element.as_ref().clone()
                            } else {
                                p.ty.clone()
                            };
                            for name in names {
                                self.declare_parameter(name, p.mutable, element_ty.clone());
                            }
                        }
                        ParamPattern::Object(names) => {
                            let fields = if let Type::Object(fields) = &p.ty {
                                fields.clone()
                            } else {
                                vec![]
                            };
                            for name in names {
                                let field_ty = fields
                                    .iter()
                                    .find(|f| f.name == *name)
                                    .map(field_borrow_type)
                                    .unwrap_or_else(|| p.ty.clone());
                                self.declare_parameter(name, p.mutable, field_ty);
                            }
                        }
                    }
                }
                let cfg = Cfg::from_stmts(body);
                let liveness = cfg.compute_liveness();
                let prev_liveness = self.current_liveness.replace(liveness);
                self.check_stmt_sequence(body);
                self.pop_scope();
                self.async_depth = previous_async_depth;
                self.current_liveness = prev_liveness;

                if is_escaping {
                    for (name, ty) in &captures {
                        if !ty.is_copy() {
                            if self.is_captured_by_pending_finalizer(name) {
                                self.error(format!(
                                    "error: cannot move `{name}` because it is captured by a pending finalizer"
                                ));
                                continue;
                            }
                            if let Some(v) = self.lookup_mut(name) {
                                v.availability = Availability::Moved;
                            }
                        }
                    }
                }
            }
            ExprKind::TypeAssert { value, .. } => {
                self.check_expr_read(value);
            }
            ExprKind::ConstAssert(inner) | ExprKind::Comptime(inner) => {
                self.check_expr_read(inner);
            }
            ExprKind::Await(inner) => {
                self.check_expr_read(inner);
                self.validate_no_borrow_across_await();
            }
            ExprKind::Try(inner) => {
                self.check_expr_read(inner);
                if let ExprKind::Ident(name) = inner.kind() {
                    self.maybe_move(name, &Type::Infer);
                }
            }

            ExprKind::In { key, object } => {
                self.check_expr_read(key);
                self.check_expr_read(object);
            }

            ExprKind::Range { start, end, .. } => {
                self.check_expr_read(start);
                self.check_expr_read(end);
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
