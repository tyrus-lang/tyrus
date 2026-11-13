use super::*;

impl BorrowChecker {
    /// Refuses to take ownership of a non-Copy value out of a field.
    ///
    /// Moving a whole binding is tracked: the source is marked moved and using
    /// it again is an error. A field had no such tracking, so
    /// `let alias: i32[] = basket.items;` produced a second owner of the same
    /// buffer with nothing recording it. Mutating through either one then
    /// reallocated a buffer the other still pointed at, and scope exit freed it
    /// twice. That is a use-after-free reachable from ordinary code with no
    /// `unsafe` anywhere, and it did not crash: it left the allocator spinning
    /// on a corrupted free list, which is a hang with no stack and no exit code.
    ///
    /// Refused rather than tracked because tracking it properly means partial
    /// moves: `basket` would have to stay usable for its other fields while
    /// `items` alone became unavailable. Until that exists, a borrow is the
    /// operation the program actually wants, and it is what the help suggests.
    pub(super) fn move_out_of_field(&mut self, value: &Expr, declared: &Type) {
        let ExprKind::FieldAccess { object, field } = value.kind() else {
            return;
        };
        let field_type = field_move_path(value).and_then(|(root, path)| {
            self.lookup(&root)
                .map(|var| var.ty.clone())
                .and_then(|ty| self.field_type_on_path(&ty, &path))
        });
        let moved_type = match (declared, &field_type) {
            (Type::Infer, Some(field_type)) => field_type.clone(),
            (Type::Infer, None) => return,
            (declared, _) => declared.clone(),
        };
        if moved_type.is_copy() {
            return;
        }
        if matches!(field.as_str(), "stdin" | "stdout" | "stderr")
            && matches!(
                object.kind(),
                ExprKind::Ident(name)
                    if matches!(self.lookup(name).map(|binding| &binding.ty), Some(Type::Named(ty)) if ty == "ChildProcess")
            )
        {
            return;
        }
        if let ExprKind::FieldAccess {
            object: env_object,
            field: env_field,
        } = object.kind()
            && env_field == "env"
            && matches!(env_object.kind(), ExprKind::Ident(name) if name == "process")
        {
            return;
        }
        if let ExprKind::Ident(name) = object.kind()
            && (self.lookup(name).is_none() || self.enum_namespaces.contains(name))
        {
            return;
        }
        if matches!(object.kind(), ExprKind::Borrow { .. }) {
            return;
        }
        if let Some((root, path)) = field_move_path(value)
            && self.lookup(&root).is_some()
        {
            if let Some(field_type) = field_type
                .as_ref()
                .filter(|field_type| !matches!(declared, Type::Infer) && *field_type != declared)
            {
                let owner = crate::ast::place_path(object).unwrap_or_else(|| root.clone());
                self.error(format!(
                        "error: cannot move out of `{owner}`: field `{field}` holds `{field_type}` and the binding is declared `{declared}`, so the value would have to change shape where it stands; use `{owner}.{field}` directly to work on it in place, `&{owner}.{field}` to borrow it, or `{owner}.{field}.copy()` for an independent value"
                    ));
                return;
            }
            self.record_field_move(&root, &path, object, field);
            return;
        }
        let owner = crate::ast::place_path(object)
            .or_else(|| crate::ast::place_root_ident(object).map(str::to_string))
            .unwrap_or_else(|| "the object".to_string());
        self.error(format!(
            "error: cannot move out of `{owner}`: taking ownership of field `{field}` would leave `{owner}` holding a value it no longer owns; use `{owner}.{field}` directly to work on it in place, `&{owner}.{field}` to borrow it, or `{owner}.{field}.copy()` for an independent value"
        ));
    }

    pub(super) fn record_field_move(&mut self, root: &str, path: &str, object: &Expr, field: &str) {
        let owner = crate::ast::place_path(object).unwrap_or_else(|| root.to_string());
        let Some(var) = self.lookup(root) else {
            return;
        };
        if is_reference_type(&var.ty) {
            self.error(format!(
                "error: cannot move out of `{owner}`: taking ownership of field `{field}` through a reference would take it from the value the reference borrows; use `{owner}.{field}` directly to work on it in place, or `{owner}.{field}.copy()` for an independent value"
            ));
            return;
        }
        if var.availability == Availability::Moved {
            self.error(format!("error: use of moved value: `{root}`"));
            return;
        }
        if var.availability == Availability::Dropped {
            self.error(format!("error: use of dropped value: `{root}`"));
            return;
        }
        if var.borrow != BorrowState::None {
            self.error(format!(
                "error: cannot move out of `{root}` while it is borrowed"
            ));
            return;
        }
        let already_moved = var
            .moved_fields
            .iter()
            .find(|moved| paths_overlap(moved, path))
            .cloned();
        if let Some(moved) = already_moved {
            self.error(format!("error: use of moved value: `{root}.{moved}`"));
            return;
        }
        if self.is_captured_by_pending_finalizer(root) {
            self.error(format!(
                "error: cannot move `{root}` because it is captured by a pending finalizer"
            ));
            return;
        }
        let declared = var.ty.clone();
        if let Some(decorated) = self.validated_owner_on_path(&declared, path) {
            self.error(format!(
                "error: cannot move out of `{owner}`: taking ownership of field `{field}` would leave `{decorated}`, whose fields carry validation rules, holding a value those rules no longer describe; use `{owner}.{field}` directly to work on it in place, `&{owner}.{field}` to borrow it, or `{owner}.{field}.copy()` for an independent value"
            ));
            return;
        }
        if let Some(var) = self.lookup_mut(root) {
            var.moved_fields.push(path.to_string());
        }
    }

    pub(super) fn validated_owner_on_path(&self, ty: &Type, path: &str) -> Option<String> {
        let segments = path.split('.').collect::<Vec<_>>();
        let mut current = ty.clone();
        for (index, segment) in segments.iter().enumerate() {
            let (name, fields) = self.fields_of(&current)?;
            if fields.iter().any(|field| !field.decorators.is_empty()) {
                return Some(name.unwrap_or_else(|| current.to_string()));
            }
            if index + 1 == segments.len() {
                break;
            }
            current = fields
                .iter()
                .find(|field| field.name == *segment)?
                .ty
                .clone();
        }
        None
    }

    pub(super) fn field_type_on_path(&self, ty: &Type, path: &str) -> Option<Type> {
        let mut current = ty.clone();
        for segment in path.split('.') {
            let (_, fields) = self.fields_of(&current)?;
            let field = fields.iter().find(|field| field.name == segment)?;
            current = if field.optional && !matches!(field.ty, Type::Optional(_)) {
                Type::Optional(Box::new(field.ty.clone()))
            } else {
                field.ty.clone()
            };
        }
        Some(current)
    }

    pub(super) fn fields_of(&self, ty: &Type) -> Option<(Option<String>, Vec<Field>)> {
        match ty {
            Type::Named(name) => self
                .interface_fields
                .get(name)
                .map(|fields| (Some(name.clone()), fields.clone())),
            Type::Object(fields) => Some((None, fields.clone())),
            Type::Readonly(inner) | Type::Optional(inner) => self.fields_of(inner),
            _ => None,
        }
    }

    pub(super) fn maybe_move(&mut self, src: &str, _target_ty: &Type) {
        if self.is_captured_by_pending_finalizer(src) {
            self.error(format!(
                "error: cannot move `{src}` because it is captured by a pending finalizer"
            ));
            return;
        }
        if let Some(v) = self.lookup(src) {
            if v.borrow != BorrowState::None {
                self.error(format!("error: cannot move `{src}` while it is borrowed"));
                return;
            }
            if !v.ty.is_copy()
                && let Some(v) = self.lookup_mut(src)
            {
                v.availability = Availability::Moved;
            }
        }
    }

    pub(super) fn mutex_guard_return_type(&self, expr: &Expr) -> Option<Type> {
        let ExprKind::MethodCall { object, method, .. } = expr.kind() else {
            return None;
        };
        if !matches!(method.as_str(), "lock" | "tryLock") {
            return None;
        }
        let ExprKind::Ident(name) = object.as_ref().kind() else {
            return None;
        };
        let Type::Apply {
            name: type_name,
            args,
        } = &self.lookup(name)?.ty
        else {
            return None;
        };
        if type_name != "Mutex" || args.len() != 1 {
            return None;
        }
        let guard = Type::Apply {
            name: "MutexGuard".to_string(),
            args: vec![args[0].clone()],
        };
        Some(if method == "tryLock" {
            Type::Optional(Box::new(guard))
        } else {
            guard
        })
    }

    pub(super) fn rwlock_guard_return_type(&self, expr: &Expr) -> Option<Type> {
        let ExprKind::MethodCall { object, method, .. } = expr.kind() else {
            return None;
        };
        if !matches!(method.as_str(), "read" | "write" | "tryRead" | "tryWrite") {
            return None;
        }
        let ExprKind::Ident(name) = object.as_ref().kind() else {
            return None;
        };
        let Type::Apply {
            name: type_name,
            args,
        } = &self.lookup(name)?.ty
        else {
            return None;
        };
        if type_name != "RwLock" || args.len() != 1 {
            return None;
        }
        let guard_name = if matches!(method.as_str(), "read" | "tryRead") {
            "RwReadGuard"
        } else {
            "RwWriteGuard"
        };
        let guard = Type::Apply {
            name: guard_name.to_string(),
            args: vec![args[0].clone()],
        };
        Some(if matches!(method.as_str(), "tryRead" | "tryWrite") {
            Type::Optional(Box::new(guard))
        } else {
            guard
        })
    }

    pub(super) fn semaphore_permit_return_type(&self, expr: &Expr) -> Option<Type> {
        let ExprKind::MethodCall { object, method, .. } = expr.kind() else {
            return None;
        };
        if !matches!(method.as_str(), "acquire" | "tryAcquire") {
            return None;
        }
        let ExprKind::Ident(name) = object.as_ref().kind() else {
            return None;
        };
        let Type::Named(type_name) = &self.lookup(name)?.ty else {
            return None;
        };
        if type_name != "Semaphore" {
            return None;
        }
        let permit = Type::Named("SemaphorePermit".to_string());
        Some(if method == "tryAcquire" {
            Type::Optional(Box::new(permit))
        } else {
            permit
        })
    }

    pub(super) fn is_http_router_route_call(&self, object: &Expr, method: &str) -> bool {
        if !matches!(
            method,
            "get"
                | "post"
                | "put"
                | "patch"
                | "delete"
                | "head"
                | "options"
                | "connect"
                | "query"
                | "trace"
        ) {
            return false;
        }
        let Expr {
            kind: ExprKind::Ident(name),
            ..
        } = object
        else {
            return false;
        };
        matches!(
            self.lookup(name).map(|var| transparent_borrow_type(&var.ty)),
            Some(Type::Named(name)) if name == "HttpRouter"
        )
    }
}
