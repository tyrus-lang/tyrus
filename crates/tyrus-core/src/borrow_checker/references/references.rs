use super::*;

impl BorrowChecker {
    pub(super) fn collect_reference_origins(
        &self,
        expr: &Expr,
        origins: &mut Vec<ReferenceOrigin>,
    ) {
        match expr.kind() {
            ExprKind::Borrow { name, .. } => {
                if let Some(source) = self.lookup(name) {
                    origins.push(ReferenceOrigin {
                        source: name.clone(),
                        source_scope_level: source.scope_level,
                        source_is_parameter: source.is_parameter,
                        source_ty: source.ty.clone(),
                    });
                }
            }
            ExprKind::Ident(name) => {
                if let Some(var) = self.lookup(name)
                    && is_reference_type(&var.ty)
                {
                    let source_name = var
                        .active_borrow
                        .as_ref()
                        .map(|active| active.source.as_str())
                        .unwrap_or(name.as_str());
                    if let Some(source) = self.lookup(source_name) {
                        origins.push(ReferenceOrigin {
                            source: source_name.to_string(),
                            source_scope_level: source.scope_level,
                            source_is_parameter: source.is_parameter,
                            source_ty: source.ty.clone(),
                        });
                    }
                }
            }
            ExprKind::ObjectLit(fields) => {
                for (_, value) in fields {
                    self.collect_reference_origins(value, origins);
                }
            }
            ExprKind::ArrayLit(values) => {
                for value in values {
                    self.collect_reference_origins(value, origins);
                }
            }
            ExprKind::ArrayRepeat { value, count } => {
                self.collect_reference_origins(value, origins);
                self.collect_reference_origins(count, origins);
            }
            ExprKind::ObjectUpdate { base, fields } => {
                self.collect_reference_origins(base, origins);
                for (_, value) in fields {
                    self.collect_reference_origins(value, origins);
                }
            }
            ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
                self.collect_reference_origins(object, origins);
            }
            ExprKind::IndexAccess { object, index } => {
                self.collect_reference_origins(object, origins);
                self.collect_reference_origins(index, origins);
            }
            ExprKind::Unwrap(inner)
            | ExprKind::Not(inner)
            | ExprKind::Neg(inner)
            | ExprKind::BitNot(inner)
            | ExprKind::Typeof(inner)
            | ExprKind::Spread(inner)
            | ExprKind::TypeAssert { value: inner, .. }
            | ExprKind::Is { expr: inner, .. }
            | ExprKind::ConstAssert(inner)
            | ExprKind::Comptime(inner)
            | ExprKind::PrefixIncr(inner)
            | ExprKind::PrefixDecr(inner)
            | ExprKind::PostfixIncr(inner)
            | ExprKind::PostfixDecr(inner) => self.collect_reference_origins(inner, origins),
            ExprKind::BinOp { left, right, .. } => {
                self.collect_reference_origins(left, origins);
                self.collect_reference_origins(right, origins);
            }
            ExprKind::ChainedComparison { terms, .. } => {
                for term in terms {
                    self.collect_reference_origins(term, origins);
                }
            }
            ExprKind::Pipeline { left, right } => {
                if contains_outer_topic(right) {
                    self.collect_reference_origins(left, origins);
                    self.collect_reference_origins(right, origins);
                } else {
                    match desugar_pipeline(left, right) {
                        Some(call) => self.collect_reference_origins(&call, origins),
                        None => {
                            self.collect_reference_origins(left, origins);
                            self.collect_reference_origins(right, origins);
                        }
                    }
                }
            }
            ExprKind::TemplateLit(parts) => {
                for part in parts {
                    if let TemplatePart::Expr(expr, _) = part {
                        self.collect_reference_origins(expr, origins);
                    }
                }
            }
            ExprKind::FnCall { args, .. } => {
                for arg in args {
                    self.collect_reference_origins(arg, origins);
                }
            }
            ExprKind::CallValue { callee, args, .. } => {
                self.collect_reference_origins(callee, origins);
                for arg in args {
                    self.collect_reference_origins(arg, origins);
                }
            }
            ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
                if let Some(initial) = initial {
                    self.collect_reference_origins(initial, origins);
                }
            }
            ExprKind::MethodCall { object, args, .. }
            | ExprKind::OptionalMethodCall { object, args, .. } => {
                self.collect_reference_origins(object, origins);
                for arg in args {
                    self.collect_reference_origins(arg, origins);
                }
            }
            ExprKind::Ternary {
                cond,
                then_expr,
                else_expr,
            } => {
                self.collect_reference_origins(cond, origins);
                self.collect_reference_origins(then_expr, origins);
                self.collect_reference_origins(else_expr, origins);
            }
            ExprKind::In { key, object } => {
                self.collect_reference_origins(key, origins);
                self.collect_reference_origins(object, origins);
            }
            ExprKind::Range { start, end, .. } => {
                self.collect_reference_origins(start, origins);
                self.collect_reference_origins(end, origins);
            }
            ExprKind::Await(inner) => {
                self.collect_reference_origins(inner, origins);
            }
            ExprKind::Try(inner) => {
                self.collect_reference_origins(inner, origins);
            }
            ExprKind::If { .. } | ExprKind::Loop { .. } | ExprKind::ArrowFn { .. } => {}
            ExprKind::IntLit(_)
            | ExprKind::UIntLit(_)
            | ExprKind::FloatLit(_)
            | ExprKind::DecimalLit(_)
            | ExprKind::StrLit(_)
            | ExprKind::RegexLit { .. }
            | ExprKind::BoolLit(_)
            | ExprKind::Null
            | ExprKind::None
            | ExprKind::Topic => {}
        }
    }

    pub(super) fn active_borrow_for_expr(&self, expr: &Expr) -> Option<ActiveBorrow> {
        match expr.kind() {
            ExprKind::Borrow { mutable, name, .. } => Some(ActiveBorrow {
                source: name.clone(),
                mutable: *mutable,
            }),
            ExprKind::Ident(name) => self.lookup(name).and_then(|var| {
                if is_reference_type(&var.ty) {
                    Some(ActiveBorrow {
                        source: var
                            .active_borrow
                            .as_ref()
                            .map(|active| active.source.clone())
                            .unwrap_or_else(|| name.clone()),
                        mutable: matches!(transparent_borrow_type(&var.ty), Type::MutRef(_)),
                    })
                } else {
                    None
                }
            }),
            _ => None,
        }
    }

    pub(super) fn validate_reference_assignment(&mut self, target_scope_level: usize, expr: &Expr) {
        let mut origins = Vec::new();
        self.collect_reference_origins(expr, &mut origins);
        for origin in origins {
            if origin.source_scope_level > target_scope_level {
                self.error(format!(
                    "error: borrowed value `{}` does not live long enough",
                    origin.source
                ));
            }
        }
    }

    pub(super) fn validate_return_references(&mut self, expr: &Expr) {
        let mut origins = Vec::new();
        self.collect_reference_origins(expr, &mut origins);
        for origin in origins {
            if !self.reference_origin_can_escape_function(&origin) {
                self.error(format!(
                    "error: cannot return reference to local variable `{}`",
                    origin.source
                ));
            }
        }
    }

    pub(super) fn reference_origin_can_escape_function(&self, origin: &ReferenceOrigin) -> bool {
        origin.source_scope_level == 0
            || (origin.source_is_parameter && is_reference_type(&origin.source_ty))
    }

    pub(super) fn validate_no_borrow_across_await(&mut self) {
        if self.async_depth == 0 {
            return;
        }
        let mut crossing_borrows = Vec::new();
        for scope in &self.scopes {
            for (name, var) in scope {
                if var.scope_level == 0 || var.dead {
                    continue;
                }
                if let Some(active) = &var.active_borrow {
                    crossing_borrows.push((name.clone(), active.source.clone()));
                } else if is_reference_type(&var.ty) {
                    crossing_borrows.push((name.clone(), name.clone()));
                }
            }
        }
        crossing_borrows.sort();
        crossing_borrows.dedup();
        for (borrow, source) in crossing_borrows {
            self.error(format!(
                "error: borrow `{borrow}` of `{source}` crosses `await`"
            ));
        }
    }

    pub(super) fn is_external(&self, name: &str) -> bool {
        self.external_names.contains(name)
            || self
                .external_prefixes
                .iter()
                .any(|prefix| name.starts_with(prefix))
    }
}
