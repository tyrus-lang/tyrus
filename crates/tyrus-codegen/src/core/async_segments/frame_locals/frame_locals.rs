use super::*;

impl Codegen<'_> {
    /// Walks the body and records every local that has to live in the async frame.
    pub(super) fn collect_async_frame_locals(
        &mut self,
        body: &[Stmt],
        last_suspending_outer: Option<usize>,
        frame_locals: &mut Vec<(String, Type, AsyncLocalStorage)>,
    ) -> Result<(), String> {
        let mut frame_candidates: Vec<(&Stmt, bool)> = Vec::new();
        let preserved_end = if let Some(last_suspending_outer) = last_suspending_outer {
            for statement in &body[..=last_suspending_outer] {
                frame_candidates.push((statement, false));
                let mut nested = Vec::new();
                collect_nested_async_statements(statement, &mut nested);
                frame_candidates.extend(nested.into_iter().map(|statement| (statement, false)));
            }
            last_suspending_outer + 1
        } else {
            0
        };
        for statement in &body[preserved_end..] {
            frame_candidates.push((statement, true));
            let mut nested = Vec::new();
            collect_nested_async_statements(statement, &mut nested);
            frame_candidates.extend(nested.into_iter().map(|statement| (statement, true)));
        }
        for (stmt, after_last_suspension) in frame_candidates {
            if let StmtKind::VarDecl {
                mutable,
                name,
                ty,
                value,
                onfail,
                ..
            } = stmt.kind()
            {
                let resolved = if let Some(value) = value {
                    self.resolve_onfail_binding_type(ty, value, onfail.is_some())?
                } else {
                    self.resolve_binding_type(ty, None)?
                };
                if matches!(resolved, Type::Void) {
                    continue;
                }
                if after_last_suspension && resolved.is_copy() {
                    continue;
                }
                let static_string = !*mutable
                    && matches!(resolved, Type::Str)
                    && value
                        .as_ref()
                        .is_some_and(|value| matches!(value.kind(), ExprKind::StrLit(_)));
                let promise_handle = matches!(
                    transparent_codegen_type(&resolved),
                    Type::Apply { name, args }
                        if matches!(name.as_str(), "Promise" | "DbPromise" | "JoinHandle") && args.len() == 1
                );
                let channel_endpoint =
                    tyrus_core::native_handles::is_handle_type(transparent_codegen_type(&resolved));
                let borrowed_local = matches!(resolved, Type::Ref(_) | Type::MutRef(_));
                let local_storage = if borrowed_local {
                    AsyncLocalStorage::None
                } else if channel_endpoint
                    || value.as_ref().is_some_and(|value| {
                        (first_await_expression(value).is_some() || onfail.is_some())
                            && self.async_owned_value_supported(&resolved)
                    })
                {
                    AsyncLocalStorage::OwnedValue
                } else if matches!(transparent_codegen_type(&resolved), Type::Str) {
                    if *mutable {
                        AsyncLocalStorage::TrackedString
                    } else if value
                        .as_ref()
                        .is_some_and(|value| self.async_owned_string_expression(value))
                    {
                        AsyncLocalStorage::OwnedString
                    } else if value
                        .as_ref()
                        .is_some_and(|value| !matches!(value.kind(), ExprKind::StrLit(_)))
                    {
                        // A string read from somewhere the frame does not own,
                        // such as `const name = user.name;`: the declaration
                        // stores a clone, and the frame releases that clone.
                        AsyncLocalStorage::OwnedValue
                    } else {
                        AsyncLocalStorage::None
                    }
                } else if async_copy_array(&resolved)
                    && value
                        .as_ref()
                        .is_some_and(|value| self.async_owned_copy_array_expression(value))
                {
                    if *mutable {
                        AsyncLocalStorage::TrackedCopyArray
                    } else {
                        AsyncLocalStorage::CopyArray
                    }
                } else if async_string_array(&resolved) {
                    match value
                        .as_ref()
                        .and_then(|value| self.async_string_array_ownership_mode(value))
                    {
                        Some(_) if *mutable => AsyncLocalStorage::TrackedStringArray,
                        Some(1) => AsyncLocalStorage::StaticStringArray,
                        Some(2) => AsyncLocalStorage::OwnedStringArray,
                        _ => AsyncLocalStorage::None,
                    }
                } else if !*mutable
                    && value
                        .as_ref()
                        .is_some_and(|value| self.async_static_object_expression(&resolved, value))
                {
                    AsyncLocalStorage::StaticObject
                } else if *mutable
                    && value
                        .as_ref()
                        .is_some_and(|value| self.async_copy_object_expression(&resolved, value))
                {
                    AsyncLocalStorage::None
                } else if ((!*mutable
                    || async_frame_ownable_shape(transparent_codegen_type(&resolved)))
                    && value
                        .as_ref()
                        .is_some_and(|value| self.async_owned_local_expression(&resolved, value)))
                    || (async_frame_ownable_shape(transparent_codegen_type(&resolved))
                        && self.async_owned_value_supported(&resolved))
                {
                    AsyncLocalStorage::OwnedValue
                } else {
                    AsyncLocalStorage::None
                };
                let supported_string = static_string
                    || matches!(
                        local_storage,
                        AsyncLocalStorage::OwnedString | AsyncLocalStorage::TrackedString
                    );
                let supported_array = matches!(
                    local_storage,
                    AsyncLocalStorage::CopyArray
                        | AsyncLocalStorage::TrackedCopyArray
                        | AsyncLocalStorage::StaticStringArray
                        | AsyncLocalStorage::OwnedStringArray
                        | AsyncLocalStorage::TrackedStringArray
                );
                let supported_object = matches!(
                    local_storage,
                    AsyncLocalStorage::StaticObject | AsyncLocalStorage::OwnedValue
                ) || *mutable
                    && value
                        .as_ref()
                        .is_some_and(|value| self.async_copy_object_expression(&resolved, value));
                if !resolved.is_copy()
                    && !borrowed_local
                    && !promise_handle
                    && !channel_endpoint
                    && !supported_string
                    && !supported_array
                    && !supported_object
                {
                    return Err(format!(
                        "error: async frame storage for non-Copy local `{name}` of type `{resolved}` is not implemented yet"
                    ));
                }
                if matches!(local_storage, AsyncLocalStorage::TrackedString)
                    && value.as_ref().is_some_and(|value| {
                        !matches!(value.kind(), ExprKind::StrLit(_))
                            && !self.async_owned_string_expression(value)
                    })
                {
                    return Err(format!(
                        "error: mutable async string local `{name}` requires an initializer with known ownership"
                    ));
                }
                if let Some((_, existing, _)) = frame_locals
                    .iter()
                    .find(|(local_name, _, _)| local_name == name)
                {
                    if existing != &resolved {
                        return Err(format!(
                            "error: async frame locals named `{name}` have incompatible shadowed types `{existing}` and `{resolved}`"
                        ));
                    }
                } else {
                    frame_locals.push((name.clone(), resolved, local_storage));
                }
            }
            if let StmtKind::ArrayDestructure {
                bindings, value, ..
            } = stmt.kind()
                && let Some(Type::Tuple(elements)) = self
                    .expr_type_hint(value)
                    .map(|ty| self.resolve_type(&ty))
                    .map(|ty| transparent_codegen_type(&ty).clone())
            {
                for (binding, element_ty) in bindings.iter().zip(elements.iter()) {
                    let resolved = self.resolve_type(element_ty);
                    let channel_endpoint = matches!(
                        transparent_codegen_type(&resolved),
                        Type::Apply { name, args }
                            if matches!(name.as_str(), "Sender" | "Receiver") && args.len() == 1
                    );
                    if !resolved.is_copy() && !channel_endpoint {
                        return Err(format!(
                            "error: async frame storage for tuple-destructured local `{}` of type `{resolved}` is not implemented yet",
                            binding.name
                        ));
                    }
                    if !frame_locals
                        .iter()
                        .any(|(local_name, _, _)| local_name == &binding.name)
                    {
                        let storage = if channel_endpoint {
                            AsyncLocalStorage::OwnedValue
                        } else {
                            AsyncLocalStorage::None
                        };
                        frame_locals.push((binding.name.clone(), resolved, storage));
                    }
                }
            }
            if let StmtKind::For { init, .. } = stmt.kind()
                && let StmtKind::VarDecl {
                    name,
                    ty,
                    value,
                    onfail,
                    ..
                } = init.kind()
            {
                let resolved = if let Some(value) = value {
                    self.resolve_onfail_binding_type(ty, value, onfail.is_some())?
                } else {
                    self.resolve_binding_type(ty, None)?
                };
                if !resolved.is_copy() {
                    return Err(format!(
                        "error: async for initializer `{name}` of type `{resolved}` requires owned frame storage"
                    ));
                }
                if !frame_locals
                    .iter()
                    .any(|(local_name, _, _)| local_name == name)
                {
                    frame_locals.push((name.clone(), resolved, AsyncLocalStorage::None));
                }
            }
            if let StmtKind::ForOf {
                name,
                iterable,
                body: loop_body,
                ..
            } = stmt.kind()
            {
                // A loop whose body never suspends runs inside one segment, so
                // its binding lives on the stack of that segment like any
                // synchronous loop's; only a binding that has to survive an
                // `await` inside the loop needs a frame slot. Either way the
                // binding is a view of the element, as in a synchronous loop:
                // the frame slot is never released, the array still owns it.
                if first_await_in_block(loop_body).is_none()
                    && !self
                        .for_of_element_type(iterable, frame_locals)
                        .is_some_and(|element| element.is_copy())
                {
                    continue;
                }
                let element_type = self
                    .expr_type_hint(iterable)
                    .or_else(|| {
                        let ExprKind::Ident(iterable_name) = iterable.kind() else {
                            return None;
                        };
                        frame_locals
                            .iter()
                            .find(|(local_name, _, _)| local_name == iterable_name)
                            .map(|(_, ty, _)| ty.clone())
                    })
                    .and_then(|ty| match transparent_codegen_type(&ty) {
                        Type::Array(element) => Some(self.resolve_type(element)),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        format!("error: async for...of binding `{name}` requires an array iterable")
                    })?;
                if !frame_locals
                    .iter()
                    .any(|(local_name, _, _)| local_name == name)
                {
                    frame_locals.push((name.clone(), element_type, AsyncLocalStorage::None));
                }
            }
        }
        Ok(())
    }

    fn for_of_element_type(
        &self,
        iterable: &Expr,
        frame_locals: &[(String, Type, AsyncLocalStorage)],
    ) -> Option<Type> {
        self.expr_type_hint(iterable)
            .or_else(|| {
                let ExprKind::Ident(iterable_name) = iterable.kind() else {
                    return None;
                };
                frame_locals
                    .iter()
                    .find(|(local_name, _, _)| local_name == iterable_name)
                    .map(|(_, ty, _)| ty.clone())
            })
            .and_then(|ty| match transparent_codegen_type(&ty) {
                Type::Array(element) => Some(self.resolve_type(element)),
                _ => None,
            })
    }
}
