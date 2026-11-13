use super::*;

impl TypeChecker {
    pub(super) fn validate_native_promise_name_conflicts(&mut self, program: &Program) {
        for import in &program.imports {
            for specifier in &import.specifiers {
                if specifier.local == "Promise" {
                    self.error_on_declaration(
                        "E3049",
                        "`Promise<T>` is a native async result type and cannot be imported or aliased by user code".to_string(),
                        Some("Promise"),
                    );
                }
                let from_task =
                    |name: &str| import.source == "tyrus:task" && specifier.imported == name;
                if specifier.local == "JoinHandle" && !from_task("JoinHandle") {
                    self.error_on_declaration(
                        "E3042",
                        "`JoinHandle<T>` is a native task handle type returned by `spawn`; import it only from `tyrus:task` and without an alias".to_string(),
                        Some("JoinHandle"),
                    );
                }
                if matches!(specifier.local.as_str(), "Sender" | "Receiver")
                    && !from_task(&specifier.local)
                {
                    self.error_on_declaration(
                        "E3040",
                        format!(
                            "`{0}<T>` is a native channel endpoint type returned by `channel`; import it only from `tyrus:task` and without an alias",
                            specifier.local
                        ),
                        Some(&specifier.local),
                    );
                }
                if specifier.local == "Mutex"
                    && !(import.source == "tyrus:task" && specifier.imported == "Mutex")
                {
                    self.error_on_declaration(
                        "E3044",
                        "`Mutex<T>` is a native mutual-exclusion type and cannot be imported or aliased by user code".to_string(),
                        Some("Mutex"),
                    );
                }
                if specifier.local == "MutexGuard" {
                    self.error_on_declaration(
                        "E3044",
                        "`MutexGuard<T>` is a native lock guard type returned by `Mutex<T>.lock()`/`.tryLock()` and cannot be imported or aliased by user code".to_string(),
                        Some("MutexGuard"),
                    );
                }
                if specifier.local == "RwLock"
                    && !(import.source == "tyrus:task" && specifier.imported == "RwLock")
                {
                    self.error_on_declaration(
                        "E3046",
                        "`RwLock<T>` is a native reader-writer lock type and cannot be imported or aliased by user code".to_string(),
                        Some("RwLock"),
                    );
                }
                if matches!(specifier.local.as_str(), "RwReadGuard" | "RwWriteGuard") {
                    self.error_on_declaration(
                        "E3046",
                        format!(
                            "`{}<T>` is a native lock guard type returned by `RwLock<T>.read()`/`.write()`/`.tryRead()`/`.tryWrite()` and cannot be imported or aliased by user code",
                            specifier.local
                        ),
                        Some(&specifier.local),
                    );
                }
                if specifier.local == "Semaphore"
                    && !(import.source == "tyrus:task" && specifier.imported == "Semaphore")
                {
                    self.error_on_declaration(
                        "E3047",
                        "`Semaphore` is a native concurrency limiter type and cannot be imported or aliased by user code".to_string(),
                        Some("Semaphore"),
                    );
                }
                if specifier.local == "SemaphorePermit" {
                    self.error_on_declaration(
                        "E3047",
                        "`SemaphorePermit` is a native permit type returned by `Semaphore.acquire()`/`.tryAcquire()` and cannot be imported or aliased by user code".to_string(),
                        Some("SemaphorePermit"),
                    );
                }
                if matches!(specifier.local.as_str(), "Error" | "AggregateError") {
                    self.error_on_declaration(
                        "E3009",
                        format!(
                            "`{}` is a native asynchronous error type and cannot be imported or aliased by user code",
                            specifier.local
                        ),
                        Some(&specifier.local),
                    );
                }
            }
        }
        for item in &program.items {
            self.item_span = Some(item.span());
            let name = match item {
                Item::InterfaceDecl { name, .. }
                | Item::TypeAlias { name, .. }
                | Item::EnumDecl { name, .. }
                | Item::FnDecl { name, .. } => Some(name),
                Item::Stmt(Stmt {
                    kind: StmtKind::VarDecl { name, .. },
                    ..
                }) => Some(name),
                _ => None,
            };
            if name.is_some_and(|name| name == "Promise") {
                self.error_on_declaration(
                    "E3049",
                    "`Promise<T>` is a native async result type and cannot be redeclared by user code".to_string(),
                    Some("Promise"),
                );
            }
            if name.is_some_and(|name| name == "JoinHandle") {
                self.error_on_declaration(
                    "E3042",
                    "`JoinHandle<T>` is a native task handle type returned by `spawn` and cannot be redeclared by user code".to_string(),
                    Some("JoinHandle"),
                );
            }
            if let Some(name @ ("Sender" | "Receiver")) = name.map(String::as_str) {
                self.error_on_declaration(
                    "E3040",
                    format!(
                        "`{name}<T>` is a native channel endpoint type returned by `channel` and cannot be redeclared by user code"
                    ),
                    Some(name),
                );
            }
            if name.is_some_and(|name| name == "Mutex") {
                self.error_on_declaration(
                    "E3044",
                    "`Mutex<T>` is a native mutual-exclusion type and cannot be redeclared by user code".to_string(),
                    Some("Mutex"),
                );
            }
            if name.is_some_and(|name| name == "MutexGuard") {
                self.error_on_declaration(
                    "E3044",
                    "`MutexGuard<T>` is a native lock guard type returned by `Mutex<T>.lock()`/`.tryLock()` and cannot be redeclared by user code".to_string(),
                    Some("MutexGuard"),
                );
            }
            if name.is_some_and(|name| name == "RwLock") {
                self.error_on_declaration(
                    "E3046",
                    "`RwLock<T>` is a native reader-writer lock type and cannot be redeclared by user code".to_string(),
                    Some("RwLock"),
                );
            }
            if let Some(name @ ("RwReadGuard" | "RwWriteGuard")) = name.map(String::as_str) {
                self.error_on_declaration(
                    "E3046",
                    format!(
                        "`{name}<T>` is a native lock guard type returned by `RwLock<T>.read()`/`.write()`/`.tryRead()`/`.tryWrite()` and cannot be redeclared by user code"
                    ),
                    Some(name),
                );
            }
            if name.is_some_and(|name| name == "Semaphore") {
                self.error_on_declaration(
                    "E3047",
                    "`Semaphore` is a native concurrency limiter type and cannot be redeclared by user code".to_string(),
                    Some("Semaphore"),
                );
            }
            if name.is_some_and(|name| name == "SemaphorePermit") {
                self.error_on_declaration(
                    "E3047",
                    "`SemaphorePermit` is a native permit type returned by `Semaphore.acquire()`/`.tryAcquire()` and cannot be redeclared by user code".to_string(),
                    Some("SemaphorePermit"),
                );
            }
            if let Some(name @ ("Error" | "AggregateError")) = name.map(String::as_str) {
                self.error_on_declaration(
                    "E3009",
                    format!(
                        "`{name}` is a native asynchronous error type and cannot be redeclared by user code"
                    ),
                    Some(name),
                );
            }
        }
    }

    pub(super) fn check_async_codegen_boundaries(
        &mut self,
        params: &[Param],
        ret_type: &Type,
        body: &[Stmt],
    ) {
        if let Some(payload) = promise_payload(ret_type) {
            let payload = self.resolve_type_annotation(payload);
            if !self.async_codegen_payload_supported(&payload) {
                self.error(
                    "E3007",
                    format!(
                        "async return payload `{payload}` does not have ownership-safe cleanup yet"
                    ),
                    Some(&payload.to_string()),
                );
            }
        }
        for param in params {
            let name = param
                .pattern
                .binding_names()
                .first()
                .copied()
                .unwrap_or("parameter");
            let ty = self.resolve_type_annotation(&param.ty);
            let supported_form = match &param.pattern {
                ParamPattern::Name(_) if param.rest => {
                    matches!(&ty, Type::Array(element) if element.is_copy() || self.async_owned_value_supported(element))
                }
                ParamPattern::Name(_) => {
                    !param.optional
                        || param.default_value.is_some()
                        || ty.is_copy()
                        || async_http_reply_handle(&ty)
                        || self.async_owned_value_supported(&ty)
                }
                ParamPattern::Array(_) => {
                    matches!(&ty, Type::Array(element) if element.is_copy() || self.async_owned_value_supported(element))
                }
                ParamPattern::Object(_) => self.async_frame_object(&ty),
            };
            if !supported_form {
                self.error_at_with_subject(
                    "E3086",
                    format!(
                        "async parameter `{name}` requires owned argument-frame lowering that is not implemented yet"
                    ),
                    param.span,
                    name.to_string(),
                );
            } else if !ty.is_copy()
                && !async_promise_handle(&ty)
                && !async_http_reply_handle(&ty)
                && !self.async_owned_value_supported(&ty)
            {
                self.error_at_with_subject(
                    "E3087",
                    format!(
                        "async parameter `{name}` of type `{ty}` does not have ownership-safe frame cleanup yet"
                    ),
                    param.span,
                    name.to_string(),
                );
            }
        }
        for statement in body {
            if let Some(span) = first_await_in_onfail_handler(statement) {
                self.error_at_with_help(
                    "E3002",
                    "this `await` inside an `onfail` handler requires continuation lowering that is not implemented yet"
                        .to_string(),
                    span,
                    vec!["keep the handler synchronous, and return the error so the caller can run the asynchronous recovery".to_string()],
                );
                continue;
            }
            if async_statement_await_is_lowerable(statement) {
                continue;
            }
            if self.async_for_of_await_is_lowerable(statement) {
                continue;
            }
            if self.async_nested_copy_await_is_lowerable(statement) {
                continue;
            }
            if self.async_loop_condition_await_is_lowerable(statement) {
                continue;
            }
            if let Some(span) = async_loop_guard_before_await(statement) {
                self.error_at(
                    "E3010",
                    "this `await` runs after a short-circuiting guard, so the loop condition cannot be lowered as a single suspension"
                        .to_string(),
                    span,
                );
                continue;
            }
            if let Some(span) = first_await_in_statement(statement) {
                let help = if statement_awaits_inside_a_loop(statement) {
                    vec![
                        "move the `try` inside the loop, around the `await` itself; note that this changes the behaviour, since a failure then ends one iteration instead of the loop".to_string(),
                        "or move the loop into a function of its own and `try` around the call to it, which keeps a failure ending the whole loop".to_string(),
                    ]
                } else {
                    Vec::new()
                };
                self.error_at_with_help(
                    "E3002",
                    "this `await` position requires continuation lowering that is not implemented yet"
                        .to_string(),
                    span,
                    help,
                );
            }
        }
        let Some(last_await_index) = body.iter().rposition(|statement| {
            async_statement_await_is_lowerable(statement)
                && first_await_in_statement(statement).is_some()
        }) else {
            return;
        };
        for statement in &body[..=last_await_index] {
            let StmtKind::VarDecl {
                mutable,
                name,
                value,
                ..
            } = statement.kind()
            else {
                continue;
            };
            let Some(ty) = self.lookup(name).and_then(|symbol| match &symbol.ty {
                ValueType::Known(ty) => Some(ty.clone()),
                _ => None,
            }) else {
                continue;
            };
            let static_string = !*mutable
                && matches!(ty, Type::Str)
                && value
                    .as_ref()
                    .is_some_and(|value| matches!(value.kind(), ExprKind::StrLit(_)));
            let owned_string = !*mutable
                && matches!(ty, Type::Str)
                && value
                    .as_ref()
                    .is_some_and(|value| self.async_owned_string_initializer(value));
            let tracked_string = *mutable
                && matches!(ty, Type::Str)
                && value.as_ref().is_none_or(|value| {
                    matches!(value.kind(), ExprKind::StrLit(_))
                        || self.async_owned_string_initializer(value)
                        || matches!(value.kind(), ExprKind::Await(_))
                })
                && self.async_mutable_string_assignments_supported(body, name);
            let owned_array = value.as_ref().is_some_and(|value| {
                matches!(value.kind(), ExprKind::Await(_))
                    && (async_copy_array(&ty) || async_string_array(&ty))
                    || async_copy_array(&ty) && self.async_owned_copy_array_initializer(value)
                    || async_string_array(&ty) && self.async_owned_string_array_initializer(value)
            }) && (!*mutable
                || async_copy_array(&ty)
                    && self.async_mutable_copy_array_assignments_supported(body, name)
                || async_string_array(&ty)
                    && self.async_mutable_string_array_assignments_supported(body, name));
            let static_object = !*mutable
                && value
                    .as_ref()
                    .is_some_and(|value| self.async_static_object_initializer(&ty, value));
            let mutable_copy_object = *mutable
                && value.as_ref().is_some_and(|value| {
                    self.async_copy_object_initializer(&ty, value)
                        && self.async_mutable_copy_object_assignments_supported(body, name, &ty)
                });
            let mutable_owned_object = *mutable
                && matches!(ty, Type::Named(_) | Type::Object(_))
                && value.as_ref().is_some_and(|value| {
                    self.async_owned_local_initializer(&ty, value)
                        && self.async_mutable_owned_object_assignments_supported(body, name, &ty)
                });
            let owned_value = !*mutable
                && value.as_ref().is_some_and(|value| {
                    self.async_owned_local_initializer(&ty, value)
                        || first_await_in_expr(value).is_some()
                            && self.async_owned_value_supported(&ty)
                });
            if !ty.is_copy()
                && !static_string
                && !async_promise_handle(&ty)
                && !crate::native_handles::is_handle_type(&ty)
                && !owned_string
                && !tracked_string
                && !owned_array
                && !static_object
                && !mutable_copy_object
                && !mutable_owned_object
                && !owned_value
            {
                // A hidden local only carries an awaited value to the next
                // statement; code generation knows the layout of every native
                // interface, so it needs no import of its own.
                if crate::await_hoist::is_hoisted_await_name(name)
                    && self.unimported_native_element(&ty).is_some()
                {
                    continue;
                }
                if let Some((type_name, module)) = self.unimported_native_element(&ty) {
                    self.error_at_with_subject(
                        "E0412",
                        format!("cannot find type `{type_name}` in this scope"),
                        statement.span,
                        name.clone(),
                    );
                    if let Some(last) = self.diagnostics.last_mut() {
                        last.help = vec![format!(
                            "`{name}` has type `{ty}`, and `{type_name}` is exported by `{module}`; add `import {{ {type_name} }} from \"{module}\";`"
                        )];
                    }
                    continue;
                }
                let local_kind = if matches!(ty, Type::Object(_))
                    || matches!(&ty, Type::Named(type_name) if self.interfaces.contains_key(type_name))
                {
                    "object local"
                } else {
                    "local"
                };
                // A type that did not resolve comes from an error already
                // reported on the initializer; blaming the frame would point
                // at the wrong cause.
                if matches!(ty, Type::Unknown) {
                    continue;
                }
                if crate::await_hoist::is_hoisted_await_name(name) {
                    self.error_at(
                        "E3088",
                        format!(
                            "the result of this `await` has type `{ty}`, which requires owned frame storage that is not implemented yet"
                        ),
                        statement.span,
                    );
                    continue;
                }
                self.error_at_with_subject(
                    "E3088",
                    format!(
                        "async {local_kind} `{name}` of type `{ty}` requires owned frame storage that is not implemented yet"
                    ),
                    statement.span,
                    name.clone(),
                );
                if let Some(help) = value
                    .as_ref()
                    .and_then(|value| self.adoptable_initializer_help(name, &ty, value))
                    && let Some(last) = self.diagnostics.last_mut()
                {
                    last.help = vec![help];
                }
            }
        }
    }

    /// A native or method call whose result the frame cannot adopt yet has a
    /// working rewrite: the same call returned from a module-level function,
    /// whose result the frame does adopt.
    fn adoptable_initializer_help(
        &mut self,
        name: &str,
        ty: &Type,
        value: &Expr,
    ) -> Option<String> {
        let call = match value.kind() {
            ExprKind::FnCall { callee, .. } if self.callee_has_native_signature(callee) => {
                format!("{callee}(...)")
            }
            ExprKind::MethodCall { method, .. } => format!(".{method}(...)"),
            _ => return None,
        };
        if !self.async_owned_value_supported(ty) {
            return None;
        }
        Some(format!(
            "return the `{call}` result from a module-level function, such as `const load = (): {ty} => ...;`, and initialize `{name}` with `load()`"
        ))
    }

    pub(super) fn async_loop_condition_await_is_lowerable(&mut self, statement: &Stmt) -> bool {
        let (condition, body) = match statement.kind() {
            StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => (cond, body),
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                if first_await_in_statement(init).is_some()
                    || first_await_in_statement(update).is_some()
                {
                    return false;
                }
                (cond, body)
            }
            _ => return false,
        };
        if first_await_in_block(body).is_some() {
            return false;
        }
        let awaited = crate::async_shapes::nested_condition_await(condition).or_else(|| {
            crate::async_shapes::guarded_nested_condition_await(condition)
                .map(|(_, _, awaited)| awaited)
        });
        let Some(awaited) = awaited else {
            return false;
        };
        let ExprKind::Await(dependency) = awaited.kind() else {
            return false;
        };
        let ValueType::Known(Type::Apply { name, args }) = self.probe_expr(dependency) else {
            return false;
        };
        name == "Promise" && args.len() == 1 && args[0].is_copy()
    }

    pub(super) fn async_nested_copy_await_is_lowerable(&mut self, statement: &Stmt) -> bool {
        let dependency = match statement.kind() {
            StmtKind::VarDecl { value, .. } => {
                value.as_ref().and_then(first_await_dependency_in_expr)
            }
            StmtKind::Assign { value, .. }
            | StmtKind::Throw(value)
            | StmtKind::Expr(value)
            | StmtKind::ArrayDestructure { value, .. }
            | StmtKind::ObjectDestructure { value, .. } => first_await_dependency_in_expr(value),
            StmtKind::IndexAssign { index, value, .. } => first_await_dependency_in_expr(index)
                .or_else(|| first_await_dependency_in_expr(value)),
            StmtKind::Return(value) => value.as_ref().and_then(first_await_dependency_in_expr),
            _ => None,
        };
        let Some(dependency) = dependency else {
            return false;
        };
        let ValueType::Known(Type::Apply { name, args }) = self.check_expr(dependency) else {
            return false;
        };
        if name != "Promise" || args.len() != 1 {
            return false;
        }
        args[0].is_copy()
            || matches!(
                statement.kind(),
                StmtKind::VarDecl { .. }
                    | StmtKind::Assign {
                        op: AssignOp::Assign,
                        ..
                    }
                    | StmtKind::Expr(_)
                    | StmtKind::Throw(_)
            ) && self.async_owned_value_supported(&args[0])
    }

    pub(super) fn async_for_of_await_is_lowerable(&mut self, statement: &Stmt) -> bool {
        let StmtKind::ForOf { iterable, body, .. } = statement.kind() else {
            return false;
        };
        let ExprKind::Ident(name) = iterable.kind() else {
            return false;
        };
        let copy_array = self.lookup(name).is_some_and(|symbol| {
            matches!(&symbol.ty, ValueType::Known(Type::Array(element)) if element.is_copy())
        });
        copy_array
            && first_await_in_expr(iterable).is_none()
            && async_direct_void_await_block_is_lowerable(body)
    }

    pub(super) fn async_owned_string_initializer(&self, expression: &Expr) -> bool {
        if let ExprKind::FnCall { callee, .. } = expression.kind() {
            return self.lookup(callee).is_some_and(|symbol| {
                matches!(
                    &symbol.ty,
                    ValueType::Known(Type::Function { ret, .. })
                        if matches!(ret.as_ref(), Type::Str)
                )
            });
        }
        let ExprKind::MethodCall { object, method, .. } = expression.kind() else {
            return false;
        };
        if !owned_string_frame_method(method) {
            return false;
        }
        match object.kind() {
            ExprKind::StrLit(_) => true,
            ExprKind::Ident(name) => self
                .lookup(name)
                .is_some_and(|symbol| matches!(&symbol.ty, ValueType::Known(Type::Str))),
            ExprKind::MethodCall { .. } => self.async_owned_string_initializer(object),
            _ => false,
        }
    }

    pub(super) fn async_static_object_initializer(&mut self, ty: &Type, expression: &Expr) -> bool {
        let ExprKind::ObjectLit(values) = expression.kind() else {
            return false;
        };
        let fields = match ty {
            Type::Named(name) => self.interfaces.get(name).cloned(),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        };
        fields.is_some_and(|fields| {
            fields.iter().all(|field| {
                if field.optional {
                    return false;
                }
                let Some((_, value)) = values.iter().find(|(name, _)| name == &field.name) else {
                    return false;
                };
                let field_type = self.resolve_type_annotation(&field_value_type(field));
                field_type.is_copy()
                    || matches!(field_type, Type::Str)
                        && matches!(value.kind(), ExprKind::StrLit(_))
                    || async_string_array(&field_type) && static_string_array_frame_literal(value)
            })
        })
    }

    pub(super) fn async_copy_object_initializer(&mut self, ty: &Type, expression: &Expr) -> bool {
        let ExprKind::ObjectLit(values) = expression.kind() else {
            return false;
        };
        let fields = match ty {
            Type::Named(name) => self.interfaces.get(name).cloned(),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        };
        fields.is_some_and(|fields| {
            fields.iter().all(|field| {
                !field.optional
                    && self
                        .resolve_type_annotation(&field_value_type(field))
                        .is_copy()
                    && values.iter().any(|(name, _)| name == &field.name)
            })
        })
    }

    pub(super) fn async_mutable_copy_object_assignments_supported(
        &mut self,
        body: &[Stmt],
        name: &str,
        ty: &Type,
    ) -> bool {
        body.iter().all(|statement| match statement.kind() {
            StmtKind::Assign {
                name: target,
                op,
                value,
            } if target == name => {
                matches!(op, AssignOp::Assign) && self.async_copy_object_initializer(ty, value)
            }
            StmtKind::FieldAssign { object, field, .. }
                if crate::ast::place_root_ident(object).is_some_and(|root| root == name) =>
            {
                let fields = match ty {
                    Type::Named(type_name) => self.interfaces.get(type_name).cloned(),
                    Type::Object(fields) => Some(fields.clone()),
                    _ => None,
                };
                fields
                    .and_then(|fields| {
                        fields
                            .into_iter()
                            .find(|candidate| candidate.name == *field)
                    })
                    .is_some_and(|field| {
                        self.resolve_type_annotation(&field_value_type(&field))
                            .is_copy()
                    })
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                self.async_mutable_copy_object_assignments_supported(try_body, name, ty)
                    && self.async_mutable_copy_object_assignments_supported(catch_body, name, ty)
                    && finally_body.as_ref().is_none_or(|body| {
                        self.async_mutable_copy_object_assignments_supported(body, name, ty)
                    })
            }
            StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::ForIn { body, .. } => {
                self.async_mutable_copy_object_assignments_supported(body, name, ty)
            }
            StmtKind::For {
                init, update, body, ..
            } => {
                self.async_mutable_copy_object_assignments_supported(
                    std::slice::from_ref(init.as_ref()),
                    name,
                    ty,
                ) && self.async_mutable_copy_object_assignments_supported(
                    std::slice::from_ref(update.as_ref()),
                    name,
                    ty,
                ) && self.async_mutable_copy_object_assignments_supported(body, name, ty)
            }
            StmtKind::Switch { cases, default, .. } => {
                cases.iter().all(|case| {
                    self.async_mutable_copy_object_assignments_supported(&case.body, name, ty)
                }) && default.as_ref().is_none_or(|body| {
                    self.async_mutable_copy_object_assignments_supported(body, name, ty)
                })
            }
            StmtKind::Match { arms, default, .. } => {
                arms.iter().all(|arm| {
                    self.async_mutable_copy_object_assignments_supported(&arm.body, name, ty)
                }) && default.as_ref().is_none_or(|body| {
                    self.async_mutable_copy_object_assignments_supported(body, name, ty)
                })
            }
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        then_block,
                        else_block,
                        ..
                    },
                ..
            }) => {
                self.async_mutable_copy_object_assignments_supported(then_block, name, ty)
                    && else_block.as_ref().is_none_or(|body| {
                        self.async_mutable_copy_object_assignments_supported(body, name, ty)
                    })
            }
            _ => true,
        })
    }

    pub(super) fn async_mutable_owned_object_assignments_supported(
        &mut self,
        body: &[Stmt],
        name: &str,
        ty: &Type,
    ) -> bool {
        body.iter().all(|statement| match statement.kind() {
            StmtKind::Assign {
                name: target, op, ..
            } if target == name => {
                matches!(op, AssignOp::Assign) && self.async_owned_value_supported(ty)
            }
            StmtKind::FieldAssign {
                object, field, op, ..
            } if crate::ast::place_root_ident(object).is_some_and(|root| root == name) => {
                let fields = match ty {
                    Type::Named(type_name) => self.interfaces.get(type_name).cloned(),
                    Type::Object(fields) => Some(fields.clone()),
                    _ => None,
                };
                fields
                    .and_then(|fields| {
                        fields
                            .into_iter()
                            .find(|candidate| candidate.name == *field)
                    })
                    .is_some_and(|field| {
                        let field_type = self.resolve_type_annotation(&field_value_type(&field));
                        field_type.is_copy()
                            || matches!(op, AssignOp::Assign)
                                && self.async_owned_value_supported(&field_type)
                    })
            }
            StmtKind::Expr(Expr {
                kind: ExprKind::MethodCall { object, method, .. },
                ..
            }) if crate::ast::place_root_ident(object).is_some_and(|root| root == name) => {
                matches!(
                    method.as_str(),
                    "push"
                        | "unshift"
                        | "pop"
                        | "shift"
                        | "reverse"
                        | "rotate"
                        | "fill"
                        | "copyWithin"
                        | "sort"
                        | "splice"
                )
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                self.async_mutable_owned_object_assignments_supported(try_body, name, ty)
                    && self.async_mutable_owned_object_assignments_supported(catch_body, name, ty)
                    && finally_body.as_ref().is_none_or(|body| {
                        self.async_mutable_owned_object_assignments_supported(body, name, ty)
                    })
            }
            StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::ForIn { body, .. } => {
                self.async_mutable_owned_object_assignments_supported(body, name, ty)
            }
            StmtKind::For {
                init, update, body, ..
            } => {
                self.async_mutable_owned_object_assignments_supported(
                    std::slice::from_ref(init.as_ref()),
                    name,
                    ty,
                ) && self.async_mutable_owned_object_assignments_supported(
                    std::slice::from_ref(update.as_ref()),
                    name,
                    ty,
                ) && self.async_mutable_owned_object_assignments_supported(body, name, ty)
            }
            StmtKind::Switch { cases, default, .. } => {
                cases.iter().all(|case| {
                    self.async_mutable_owned_object_assignments_supported(&case.body, name, ty)
                }) && default.as_ref().is_none_or(|body| {
                    self.async_mutable_owned_object_assignments_supported(body, name, ty)
                })
            }
            StmtKind::Match { arms, default, .. } => {
                arms.iter().all(|arm| {
                    self.async_mutable_owned_object_assignments_supported(&arm.body, name, ty)
                }) && default.as_ref().is_none_or(|body| {
                    self.async_mutable_owned_object_assignments_supported(body, name, ty)
                })
            }
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        then_block,
                        else_block,
                        ..
                    },
                ..
            }) => {
                self.async_mutable_owned_object_assignments_supported(then_block, name, ty)
                    && else_block.as_ref().is_none_or(|body| {
                        self.async_mutable_owned_object_assignments_supported(body, name, ty)
                    })
            }
            _ => {
                let mut mutations = HashSet::default();
                walk_stmt_mutations(statement, &mut mutations);
                !mutations.contains(name)
            }
        })
    }

    pub(super) fn async_owned_local_initializer(&mut self, ty: &Type, expression: &Expr) -> bool {
        if matches!(ty, Type::Named(name) if name == "DigestStream")
            && matches!(expression.kind(), ExprKind::MethodCall { .. })
        {
            return true;
        }
        if !self.async_owned_value_supported(ty) {
            return false;
        }
        // Mirrors the one gap in the code generator's frame storage: an array
        // whose elements are neither Copy nor strings (an array of interfaces,
        // say) is adopted only from a literal, a repetition or a call to a
        // module-level function of the program, or cloned from a value read out
        // of other storage (`tyrus_core::ast::is_borrowed_place`). Strings, Copy arrays, string
        // arrays and every other shape have storage of their own, whatever
        // produced them. A native call, a method call or a local closure
        // producing such an array used to pass here and fail in code
        // generation, without a source range.
        let resolved = self.resolve_type_annotation(ty);
        let shape = match &resolved {
            Type::Ref(inner) | Type::MutRef(inner) | Type::Readonly(inner) => inner.as_ref(),
            other => other,
        };
        let Type::Array(element) = shape else {
            return true;
        };
        let element = self.resolve_type_annotation(element);
        if element.is_copy() || matches!(element, Type::Str) || self.is_unit_enum(&element) {
            return true;
        }
        if crate::ast::is_borrowed_place(expression) {
            return true;
        }
        match expression.kind() {
            ExprKind::FnCall { callee, .. } => self.program_function_call(callee),
            ExprKind::ArrayLit(values) => values
                .iter()
                .all(|value| !matches!(value.kind(), ExprKind::Spread(_))),
            ExprKind::ArrayRepeat { .. } => true,
            _ => false,
        }
    }

    fn is_unit_enum(&self, ty: &Type) -> bool {
        let Type::Named(name) = ty else {
            return false;
        };
        self.enums.get(name).map_or_else(
            || crate::builtin_interfaces::is_builtin_unit_enum(name),
            |resolved| !resolved.is_algebraic,
        )
    }

    /// A call to a module-level function of the program. A closure declared
    /// inside a function is looked up first and is not one: the code generator
    /// only knows the signatures of module-level functions.
    fn program_function_call(&self, callee: &str) -> bool {
        let declared_at_module_level = self
            .scopes
            .iter()
            .rposition(|scope| scope.contains_key(callee))
            == Some(0);
        declared_at_module_level
            && !self.callee_has_native_signature(callee)
            && self.lookup(callee).is_some_and(|symbol| {
                matches!(&symbol.ty, ValueType::Known(Type::Function { ret, .. })
                    if !matches!(ret.as_ref(), Type::Apply { name, .. } if name == "Promise"))
            })
    }

    pub(super) fn async_owned_copy_array_initializer(&self, expression: &Expr) -> bool {
        match expression.kind() {
            ExprKind::FnCall { callee, .. } => self.lookup(callee).is_some_and(|symbol| {
                matches!(
                    &symbol.ty,
                    ValueType::Known(Type::Function { ret, .. })
                        if matches!(ret.as_ref(), Type::Array(element) if element.is_copy())
                )
            }),
            ExprKind::ArrayLit(values) => values
                .iter()
                .all(|value| !matches!(value.kind(), ExprKind::Spread(_))),
            ExprKind::MethodCall { method, .. } => owned_copy_array_frame_method(method),
            _ => false,
        }
    }

    pub(super) fn async_owned_string_array_initializer(&self, expression: &Expr) -> bool {
        match expression.kind() {
            ExprKind::FnCall { callee, .. } => self.lookup(callee).is_some_and(|symbol| {
                matches!(
                    &symbol.ty,
                    ValueType::Known(Type::Function { ret, .. })
                        if matches!(ret.as_ref(), Type::Array(element) if matches!(element.as_ref(), Type::Str))
                )
            }),
            ExprKind::ArrayLit(values) => values
                .iter()
                .all(|value| matches!(value.kind(), ExprKind::StrLit(_))),
            ExprKind::MethodCall { method, .. } => owned_string_array_frame_method(method),
            _ => false,
        }
    }

    pub(super) fn async_mutable_string_assignments_supported(
        &self,
        body: &[Stmt],
        name: &str,
    ) -> bool {
        body.iter().all(|statement| match statement.kind() {
            StmtKind::Assign {
                name: target,
                op,
                value,
            } if target == name => {
                matches!(op, AssignOp::AddAssign)
                    || matches!(op, AssignOp::Assign)
                        && (matches!(value.kind(), ExprKind::StrLit(_))
                            || self.async_owned_string_initializer(value)
                            || matches!(value.kind(), ExprKind::Await(_)))
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                self.async_mutable_string_assignments_supported(try_body, name)
                    && self.async_mutable_string_assignments_supported(catch_body, name)
                    && finally_body.as_ref().is_none_or(|body| {
                        self.async_mutable_string_assignments_supported(body, name)
                    })
            }
            StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::ForIn { body, .. } => {
                self.async_mutable_string_assignments_supported(body, name)
            }
            StmtKind::For {
                init, update, body, ..
            } => {
                self.async_mutable_string_assignments_supported(
                    std::slice::from_ref(init.as_ref()),
                    name,
                ) && self.async_mutable_string_assignments_supported(
                    std::slice::from_ref(update.as_ref()),
                    name,
                ) && self.async_mutable_string_assignments_supported(body, name)
            }
            StmtKind::Switch { cases, default, .. } => {
                cases
                    .iter()
                    .all(|case| self.async_mutable_string_assignments_supported(&case.body, name))
                    && default.as_ref().is_none_or(|body| {
                        self.async_mutable_string_assignments_supported(body, name)
                    })
            }
            StmtKind::Match { arms, default, .. } => {
                arms.iter()
                    .all(|arm| self.async_mutable_string_assignments_supported(&arm.body, name))
                    && default.as_ref().is_none_or(|body| {
                        self.async_mutable_string_assignments_supported(body, name)
                    })
            }
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        then_block,
                        else_block,
                        ..
                    },
                ..
            }) => {
                self.async_mutable_string_assignments_supported(then_block, name)
                    && else_block.as_ref().is_none_or(|body| {
                        self.async_mutable_string_assignments_supported(body, name)
                    })
            }
            _ => true,
        })
    }

    pub(super) fn async_mutable_copy_array_assignments_supported(
        &self,
        body: &[Stmt],
        name: &str,
    ) -> bool {
        body.iter().all(|statement| match statement.kind() {
            StmtKind::Assign {
                name: target,
                op,
                value,
            } if target == name => {
                matches!(op, AssignOp::Assign)
                    && (self.async_owned_copy_array_initializer(value)
                        || matches!(value.kind(), ExprKind::Await(_)))
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                self.async_mutable_copy_array_assignments_supported(try_body, name)
                    && self.async_mutable_copy_array_assignments_supported(catch_body, name)
                    && finally_body.as_ref().is_none_or(|body| {
                        self.async_mutable_copy_array_assignments_supported(body, name)
                    })
            }
            StmtKind::While { body, .. }
            | StmtKind::DoWhile { body, .. }
            | StmtKind::ForOf { body, .. }
            | StmtKind::ForIn { body, .. } => {
                self.async_mutable_copy_array_assignments_supported(body, name)
            }
            StmtKind::For {
                init, update, body, ..
            } => {
                self.async_mutable_copy_array_assignments_supported(
                    std::slice::from_ref(init.as_ref()),
                    name,
                ) && self.async_mutable_copy_array_assignments_supported(
                    std::slice::from_ref(update.as_ref()),
                    name,
                ) && self.async_mutable_copy_array_assignments_supported(body, name)
            }
            StmtKind::Switch { cases, default, .. } => {
                cases.iter().all(|case| {
                    self.async_mutable_copy_array_assignments_supported(&case.body, name)
                }) && default.as_ref().is_none_or(|body| {
                    self.async_mutable_copy_array_assignments_supported(body, name)
                })
            }
            StmtKind::Match { arms, default, .. } => {
                arms.iter()
                    .all(|arm| self.async_mutable_copy_array_assignments_supported(&arm.body, name))
                    && default.as_ref().is_none_or(|body| {
                        self.async_mutable_copy_array_assignments_supported(body, name)
                    })
            }
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        then_block,
                        else_block,
                        ..
                    },
                ..
            }) => {
                self.async_mutable_copy_array_assignments_supported(then_block, name)
                    && else_block.as_ref().is_none_or(|body| {
                        self.async_mutable_copy_array_assignments_supported(body, name)
                    })
            }
            _ => true,
        })
    }

    pub(super) fn async_mutable_string_array_assignments_supported(
        &self,
        body: &[Stmt],
        name: &str,
    ) -> bool {
        body.iter().all(|statement| match statement.kind() {
            StmtKind::Assign {
                name: target,
                op,
                value,
            } if target == name => {
                matches!(op, AssignOp::Assign)
                    && (self.async_owned_string_array_initializer(value)
                        || matches!(value.kind(), ExprKind::Await(_)))
            }
            StmtKind::IndexAssign { name: target, .. } if target == name => true,
            StmtKind::VarDecl {
                value:
                    Some(Expr {
                        kind: ExprKind::MethodCall { object, method, .. },
                        ..
                    }),
                ..
            } if matches!(object.kind(), ExprKind::Ident(target) if target == name)
                && matches!(method.as_str(), "pop" | "shift" | "splice") =>
            {
                true
            }
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::MethodCall {
                        object,
                        method,
                        args,
                        ..
                    },
                ..
            }) if matches!(object.kind(), ExprKind::Ident(target) if target == name)
                && matches!(
                    method.as_str(),
                    "push"
                        | "unshift"
                        | "pop"
                        | "shift"
                        | "reverse"
                        | "rotate"
                        | "fill"
                        | "copyWithin"
                        | "sort"
                ) =>
            {
                args.iter().all(|argument| {
                    self.async_expression_preserves_string_array_ownership(argument, name)
                })
            }
            StmtKind::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                self.async_mutable_string_array_assignments_supported(try_body, name)
                    && self.async_mutable_string_array_assignments_supported(catch_body, name)
                    && finally_body.as_ref().is_none_or(|body| {
                        self.async_mutable_string_array_assignments_supported(body, name)
                    })
            }
            StmtKind::While { cond, body } | StmtKind::DoWhile { body, cond } => {
                self.async_expression_preserves_string_array_ownership(cond, name)
                    && self.async_mutable_string_array_assignments_supported(body, name)
            }
            StmtKind::ForOf { iterable, body, .. } => {
                self.async_expression_preserves_string_array_ownership(iterable, name)
                    && self.async_mutable_string_array_assignments_supported(body, name)
            }
            StmtKind::ForIn { object, body, .. } => {
                self.async_expression_preserves_string_array_ownership(object, name)
                    && self.async_mutable_string_array_assignments_supported(body, name)
            }
            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => {
                self.async_mutable_string_array_assignments_supported(
                    std::slice::from_ref(init.as_ref()),
                    name,
                ) && self.async_expression_preserves_string_array_ownership(cond, name)
                    && self.async_mutable_string_array_assignments_supported(
                        std::slice::from_ref(update.as_ref()),
                        name,
                    )
                    && self.async_mutable_string_array_assignments_supported(body, name)
            }
            StmtKind::Switch {
                expr,
                cases,
                default,
                ..
            } => {
                self.async_expression_preserves_string_array_ownership(expr, name)
                    && cases.iter().all(|case| {
                        self.async_mutable_string_array_assignments_supported(&case.body, name)
                    })
                    && default.as_ref().is_none_or(|body| {
                        self.async_mutable_string_array_assignments_supported(body, name)
                    })
            }
            StmtKind::Match {
                expr,
                arms,
                default,
            } => {
                self.async_expression_preserves_string_array_ownership(expr, name)
                    && arms.iter().all(|arm| {
                        arm.guard.as_ref().is_none_or(|guard| {
                            self.async_expression_preserves_string_array_ownership(guard, name)
                        }) && self.async_mutable_string_array_assignments_supported(&arm.body, name)
                    })
                    && default.as_ref().is_none_or(|body| {
                        self.async_mutable_string_array_assignments_supported(body, name)
                    })
            }
            StmtKind::Expr(Expr {
                kind:
                    ExprKind::If {
                        cond,
                        then_block,
                        else_block,
                    },
                ..
            }) => {
                self.async_expression_preserves_string_array_ownership(cond, name)
                    && self.async_mutable_string_array_assignments_supported(then_block, name)
                    && else_block.as_ref().is_none_or(|body| {
                        self.async_mutable_string_array_assignments_supported(body, name)
                    })
            }
            _ => {
                let mut mutations = HashSet::default();
                walk_stmt_mutations(statement, &mut mutations);
                !mutations.contains(name)
            }
        })
    }

    pub(super) fn async_expression_preserves_string_array_ownership(
        &self,
        expression: &Expr,
        name: &str,
    ) -> bool {
        let mut mutations = HashSet::default();
        walk_expr_mutations(expression, &mut mutations);
        !mutations.contains(name)
    }

    pub(super) fn async_frame_object(&mut self, ty: &Type) -> bool {
        let fields = match ty {
            Type::Named(name) => self.interfaces.get(name).cloned(),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        };
        fields.is_some_and(|fields| {
            fields.iter().all(|field| {
                let field_type = self.resolve_type_annotation(&field_value_type(field));
                field_type.is_copy() || self.async_owned_value_supported(&field_type)
            })
        })
    }

    /// An unimported native interface has no field list here, so the frame
    /// check cannot tell whether it is storable and would blame the frame;
    /// the missing import is the real cause, arrays, optionals and both sides
    /// of a `Result` included.
    fn unimported_native_element(&self, ty: &Type) -> Option<(String, &'static str)> {
        match ty {
            Type::Array(inner) | Type::Optional(inner) => self.unimported_native_element(inner),
            Type::Result { success, error } => self
                .unimported_native_element(success)
                .or_else(|| self.unimported_native_element(error)),
            other => self.unimported_native_interface(other),
        }
    }

    pub(super) fn async_owned_value_supported(&mut self, ty: &Type) -> bool {
        let ty = self.resolve_type_annotation(ty);
        match &ty {
            Type::Void => true,
            Type::Named(name) if name == "DigestStream" => false,
            Type::Named(name)
                if self.enums.get(name).map_or_else(
                    || crate::builtin_interfaces::is_builtin_unit_enum(name),
                    |resolved| !resolved.is_algebraic,
                ) =>
            {
                true
            }
            Type::Named(name)
                if matches!(name.as_str(), "TcpStream" | "TcpListener" | "UdpSocket") =>
            {
                true
            }
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
                ) && args.len() == 1 =>
            {
                true
            }
            Type::Apply { name, args }
                if matches!(name.as_str(), "DuplexStream" | "TransformStream")
                    && args.len() == 2 =>
            {
                true
            }
            Type::Lit(_) => true,
            Type::Str => true,
            Type::Array(element) => element.is_copy() || self.async_owned_value_supported(element),
            Type::Optional(inner) => inner.is_copy() || self.async_owned_value_supported(inner),
            Type::Result { success, error } => {
                (matches!(success.as_ref(), Type::Void)
                    || success.is_copy()
                    || self.async_owned_value_supported(success))
                    && (error.is_copy() || self.async_owned_value_supported(error))
            }
            Type::Union(variants) => variants
                .iter()
                .all(|variant| variant.is_copy() || self.async_owned_value_supported(variant)),
            Type::Tuple(elements) => elements
                .iter()
                .all(|element| element.is_copy() || self.async_owned_value_supported(element)),
            Type::Named(_) | Type::Object(_) => self.async_frame_object(&ty),
            _ => false,
        }
    }

    pub(super) fn async_codegen_payload_supported(&mut self, ty: &Type) -> bool {
        let ty = self.resolve_type_annotation(ty);
        if matches!(ty, Type::Void | Type::Str) || ty.is_copy() {
            return true;
        }
        match &ty {
            Type::Array(_)
            | Type::Optional(_)
            | Type::Result { .. }
            | Type::Union(_)
            | Type::Named(_)
            | Type::Object(_) => self.async_owned_value_supported(&ty),
            _ => false,
        }
    }

    pub(super) fn refuse_throw_past_awaiting_finally(
        &mut self,
        effect: &Type,
        operation: &str,
        span: Span,
        catching_try: Option<usize>,
    ) {
        let passed_from = catching_try.map_or(0, |index| index + 1);
        let skips_awaiting_finally = self
            .try_finally_suspends
            .iter()
            .zip(self.try_body_suspends.iter())
            .skip(passed_from)
            .any(|(finally_suspends, body_suspends)| *finally_suspends && !*body_suspends);
        if skips_awaiting_finally {
            self.error_at_with_help(
                "E3299",
                format!(
                    "{operation} may throw `{effect}` past a `finally` that awaits in a `try` whose body does not, which would not run"
                ),
                span,
                vec![
                    format!("catch `{effect}` in the `try` whose `finally` awaits, so that `finally` runs after the `catch`"),
                    "or move the `await` out of that `finally`".to_string(),
                ],
            );
        }
    }

    pub(super) fn check_promise_static_method(
        &mut self,
        method: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> ValueType {
        if type_args.len() > 1 {
            self.error(
                "E0107",
                format!(
                    "`Promise.{method}` expects at most 1 type argument, but {} were provided",
                    type_args.len()
                ),
                Some(method),
            );
        }
        let explicit_payload = type_args.first().cloned();
        match method {
            "resolve" => {
                if args.len() > 1 {
                    self.error(
                        "E0061",
                        format!(
                            "`Promise.resolve` expects at most 1 argument, but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                }
                let inferred = match (args.first(), type_args.first()) {
                    (Some(argument), Some(expected)) => {
                        let actual = self.check_expr_as(argument, expected);
                        self.require_assignable(
                            expected,
                            &actual,
                            "Promise.resolve value",
                            "E0308",
                        );
                        expected.clone()
                    }
                    (Some(argument), None) => {
                        value_type_to_type(&self.check_expr(argument)).unwrap_or(Type::Unknown)
                    }
                    (None, _) => Type::Void,
                };
                let payload = explicit_payload
                    .unwrap_or_else(|| promise_payload(&inferred).cloned().unwrap_or(inferred));
                ValueType::Known(promise_of(
                    promise_payload(&payload).cloned().unwrap_or(payload),
                ))
            }
            "reject" => {
                if args.len() != 1 {
                    self.error(
                        "E0061",
                        format!(
                            "`Promise.reject` expects 1 argument, but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                }
                let reason = Type::Union(vec![
                    Type::Named("Error".to_string()),
                    Type::Named("AggregateError".to_string()),
                ]);
                for argument in args {
                    let actual = self.check_expr_as(argument, &reason);
                    self.require_assignable(&reason, &actual, "Promise.reject reason", "E0308");
                }
                ValueType::Known(promise_of(explicit_payload.unwrap_or(Type::Unknown)))
            }
            "all" | "race" | "allSettled" | "any" => {
                if args.len() != 1 {
                    self.error(
                        "E0061",
                        format!(
                            "`Promise.{method}` expects 1 argument, but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                }
                let inferred_payload = args
                    .first()
                    .map(|argument| self.check_expr(argument))
                    .and_then(|value| value_type_to_type(&value))
                    .and_then(|value| match value {
                        Type::Array(element) => promise_payload(&element).cloned(),
                        _ => None,
                    })
                    .unwrap_or(Type::Unknown);
                let payload = explicit_payload.unwrap_or(inferred_payload);
                let result = match method {
                    "all" => Type::Array(Box::new(payload)),
                    "allSettled" => Type::Array(Box::new(promise_settled_result(payload))),
                    "race" | "any" => payload,
                    _ => unreachable!(),
                };
                ValueType::Known(promise_of(result))
            }
            "withResolvers" => {
                if !args.is_empty() {
                    self.error(
                        "E0061",
                        format!(
                            "`Promise.withResolvers` expects 0 arguments, but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                }
                let payload = explicit_payload.unwrap_or(Type::Unknown);
                ValueType::Known(Type::Object(vec![
                    native_field("promise", promise_of(payload.clone())),
                    native_field(
                        "resolve",
                        Type::Function {
                            params: vec![payload],
                            ret: Box::new(Type::Void),
                        },
                    ),
                    native_field(
                        "reject",
                        Type::Function {
                            params: vec![Type::Named("Error".to_string())],
                            ret: Box::new(Type::Void),
                        },
                    ),
                ]))
            }
            _ => {
                self.error(
                    "E0599",
                    format!("no static Promise method named `{method}`"),
                    Some(method),
                );
                for argument in args {
                    self.check_expr(argument);
                }
                ValueType::Error
            }
        }
    }

    pub(super) fn check_promise_instance_method(
        &mut self,
        payload: &Type,
        method: &str,
        args: &[Expr],
    ) -> ValueType {
        if args.len() != 1 {
            self.error(
                "E0061",
                format!(
                    "`Promise.{method}` expects 1 argument, but {} were provided",
                    args.len()
                ),
                Some(method),
            );
        }
        match method {
            "then" => {
                let callback_return = args
                    .first()
                    .map(|callback| {
                        self.check_expr_as(
                            callback,
                            &Type::Function {
                                params: if matches!(payload, Type::Void) {
                                    Vec::new()
                                } else {
                                    vec![payload.clone()]
                                },
                                ret: Box::new(Type::Infer),
                            },
                        )
                    })
                    .and_then(function_return_type)
                    .unwrap_or(Type::Unknown);
                ValueType::Known(promise_of(
                    promise_payload(&callback_return)
                        .cloned()
                        .unwrap_or(callback_return),
                ))
            }
            "catch" => {
                let callback_return = args
                    .first()
                    .map(|callback| {
                        self.check_expr_as(
                            callback,
                            &Type::Function {
                                params: vec![Type::Named("Error".to_string())],
                                ret: Box::new(Type::Infer),
                            },
                        )
                    })
                    .and_then(function_return_type)
                    .unwrap_or(Type::Unknown);
                let callback_payload = promise_payload(&callback_return)
                    .cloned()
                    .unwrap_or(callback_return);
                ValueType::Known(promise_of(if callback_payload == *payload {
                    payload.clone()
                } else {
                    Type::Union(vec![payload.clone(), callback_payload])
                }))
            }
            "finally" => {
                for callback in args {
                    self.check_expr_as(
                        callback,
                        &Type::Function {
                            params: Vec::new(),
                            ret: Box::new(Type::Infer),
                        },
                    );
                }
                ValueType::Known(promise_of(payload.clone()))
            }
            _ => {
                self.error(
                    "E0599",
                    format!("no method named `{method}` for `Promise<{payload}>`"),
                    Some(method),
                );
                for argument in args {
                    self.check_expr(argument);
                }
                ValueType::Error
            }
        }
    }

    pub(super) fn is_promise_awaited(&self, name: &str, span: Span) -> bool {
        self.awaited_promise_names
            .iter()
            .rev()
            .any(|scope| scope.get(name).is_some_and(|consumed| *consumed != span))
    }

    pub(super) fn mark_promise_awaited(&mut self, name: &str, span: Span) {
        if let Some(scope) = self.awaited_promise_names.last_mut() {
            scope.insert(name.to_string(), span);
        }
    }
}
