use super::*;

#[path = "archive_literals/archive_literals.rs"]
mod archive_literals;
#[path = "compress_literals/compress_literals.rs"]
mod compress_literals;
#[path = "container_receivers/container_receivers.rs"]
mod container_receivers;
#[path = "native_receivers/native_receivers.rs"]
mod native_receivers;
#[path = "text_receivers/text_receivers.rs"]
mod text_receivers;
pub(super) struct MethodReceiver<'a> {
    pub(super) ty: &'a ValueType,
    pub(super) optional: bool,
}

#[path = "algorithm_constructors/algorithm_constructors.rs"]
mod algorithm_constructors;
#[path = "db_calls/db_calls.rs"]
mod db_calls;
impl TypeChecker {
    pub(super) fn callee_has_native_signature(&self, callee: &str) -> bool {
        self.lookup(callee)
            .is_some_and(|symbol| matches!(symbol.kind, SymbolKind::Import))
            && !self
                .import_targets
                .get(callee)
                .is_some_and(|target| target.starts_with("__tyrus_module_"))
    }

    pub(super) fn require_unknown_representable_argument(
        &mut self,
        expected: &Type,
        actual: &ValueType,
        arg: &Expr,
        label: &str,
    ) {
        let Some(found) = self.unknown_unrepresentable_value(expected, actual) else {
            return;
        };
        self.error_on_expr(
            "E0794",
            format!("cannot store a value of type `{found}` in `unknown`"),
            arg,
        );
        if let Some(last) = self.diagnostics.last_mut() {
            last.help = vec![
                format!(
                    "`unknown` holds only `string`, `boolean`, `char`, numeric and `decimal` values, and `{found}` is none of them"
                ),
                format!(
                    "pass a scalar value as {label}, or give that parameter a type that describes the value"
                ),
            ];
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn check_call(
        &mut self,
        callee: &str,
        type_args: &[Type],
        args: &[Expr],
        span: Span,
    ) -> ValueType {
        let callee_short = crate::module_loader::namespace_member_display(callee)
            .and_then(|display| {
                display
                    .rsplit_once('.')
                    .map(|(_, member)| member.to_owned())
            })
            .unwrap_or_else(|| callee.to_owned());
        self.check_call_effects(callee, args, span);
        self.check_file_encoding_argument(callee, args);
        if type_args.is_empty() && !self.has_symbol(callee) {
            if let Some(result) = self.check_conversion_builtin(callee, args) {
                return result;
            }
            if let Some(result) = self.check_float_predicate_builtin(callee, args) {
                return result;
            }
            if let Some(result) = self.check_assert_builtin(callee, args) {
                return result;
            }
            if let Some(result) = self.check_print_builtin(callee, args) {
                return result;
            }
        }
        let callee_ty = match self.read_symbol(callee) {
            ValueType::Known(Type::Throws { function, .. }) => ValueType::Known(*function),
            other => other,
        };
        if let ValueType::Known(Type::GenericFunction {
            type_params,
            params,
            ret,
        }) = callee_ty.clone()
        {
            if !type_args.is_empty() && type_args.len() != type_params.len() {
                self.error(
                    "E0107",
                    format!(
                        "function `{callee}` expects {} type argument(s), but {} were provided",
                        type_params.len(),
                        type_args.len()
                    ),
                    Some(callee),
                );
            }
            if matches!(
                self.import_targets.get(callee).map(String::as_str),
                Some(crate::memory::SIZE_OF_TARGET | crate::memory::ALIGN_OF_TARGET)
            ) && let Some(ty) = type_args.first()
                && !crate::memory::has_stable_layout(ty)
            {
                self.error(
                            "E3244",
                            format!(
                                "`{callee_short}` requires a primitive type with stable layout, found `{ty}`"
                            ),
                            Some(callee),
                        );
            }
            if is_spawn_shaped(&type_params, &params, &ret)
                && let Some(task) = args.first()
                && let ExprKind::ArrowFn {
                    is_async: false, ..
                } = task.kind()
            {
                // Checked without the raw generic parameter type: it
                // still contains the unsubstituted `Type::Generic("T")`
                // placeholder at this point (substitution happens
                // further down, after this early return), and letting
                // that leak into `check_expr_with_expected` produces
                // confusing cascading errors ("cannot find type `T`")
                // instead of just this one clear diagnostic.
                self.check_expr_with_expected(task, None);
                self.error_at_with_help(
                    "E3030",
                    "the task passed to `spawn` must be an `async` function".to_string(),
                    task.span,
                    vec!["mark the function `async`, e.g. `async () => { ... }`".to_string()],
                );
                return ValueType::Error;
            }
            let actuals = args
                .iter()
                .enumerate()
                .map(|(index, arg)| {
                    if matches!(arg.kind(), ExprKind::ArrowFn { .. }) {
                        self.check_expr_with_expected(arg, params.get(index))
                    } else {
                        self.check_expr(arg)
                    }
                })
                .collect::<Vec<_>>();
            let is_simd_call = matches!(
                self.import_targets.get(callee).map(String::as_str),
                Some(
                    crate::simd::ADD_TARGET
                        | crate::simd::SUB_TARGET
                        | crate::simd::MUL_TARGET
                        | crate::simd::DIV_TARGET
                )
            );
            let mut substitutions = HashMap::default();
            for (parameter, explicit) in type_params.iter().zip(type_args) {
                substitutions.insert(parameter.name.clone(), explicit.clone());
            }
            for (expected, actual) in params.iter().zip(&actuals) {
                infer_generic_types(expected, actual, &mut substitutions);
            }
            let is_db_query = self
                .import_targets
                .get(callee)
                .is_some_and(|target| target == crate::db::QUERY_TARGET);
            if is_db_query {
                substitutions.entry("T".to_string()).or_insert(Type::Void);
            }
            for parameter in &type_params {
                if !substitutions.contains_key(&parameter.name) {
                    self.error(
                        "E0282",
                        format!(
                            "cannot infer type parameter `{}` for `{callee}`",
                            parameter.name
                        ),
                        Some(callee),
                    );
                }
                if let (Some(constraint), Some(actual)) =
                    (&parameter.constraint, substitutions.get(&parameter.name))
                    && !self.is_assignable(constraint, &ValueType::Known(actual.clone()))
                {
                    self.error(
                        "E0277",
                        format!("type `{actual}` does not satisfy constraint `{constraint}`"),
                        Some(callee),
                    );
                }
            }
            if is_simd_call && actuals.len() >= 2 {
                let fixed_lengths = actuals
                    .iter()
                    .take(2)
                    .map(|actual| match actual {
                        ValueType::Known(ty) => match transparent_type(ty) {
                            Type::Tuple(elements) => Some(elements.len()),
                            _ => None,
                        },
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if let ([Some(left_length), Some(right_length)], Some(argument)) =
                    (fixed_lengths.as_slice(), args.get(1))
                    && left_length != right_length
                {
                    self.error_at_with_help(
                            "E0308",
                            format!(
                                "argument 2 of `{callee}` has fixed length {right_length}, but argument 1 has fixed length {left_length}"
                            ),
                            argument.span,
                            vec![
                                "use fixed-size arrays with equal lengths, or use dynamic arrays when lengths are only known at runtime".to_string(),
                            ],
                        );
                }
            }
            for (index, (expected, actual)) in params.iter().zip(&actuals).enumerate() {
                let expected = substitute_generics(expected, &substitutions);
                if !self.is_assignable(&expected, actual) {
                    self.error("E0308", format!("type mismatch for argument {} of `{callee}`: expected `{expected}`, found `{}`", index + 1, actual.display()), Some(callee));
                }
            }
            if let (Some(target), Some(argument), Some(actual)) = (
                self.import_targets.get(callee).cloned(),
                args.first(),
                actuals.first(),
            ) {
                let accepted = match target.as_str() {
                    crate::memory::DROP_TARGET => {
                        matches!(actual, ValueType::Known(ty) if crate::memory::supports_drop(transparent_type(ty)))
                            || matches!(actual, ValueType::Unknown | ValueType::Error)
                    }
                    crate::memory::ZEROIZE_TARGET => {
                        matches!(actual, ValueType::Known(Type::Str))
                            || matches!(
                                actual,
                                ValueType::Known(Type::Array(element))
                                    if matches!(transparent_type(element), Type::U8)
                            )
                            || matches!(actual, ValueType::Unknown | ValueType::Error)
                    }
                    _ => true,
                };
                if !accepted {
                    let expected = if target == crate::memory::DROP_TARGET {
                        "an owned string, array, collection, or native handle"
                    } else {
                        "`string` or `u8[]`"
                    };
                    self.error_at_with_help(
                        "E3243",
                        format!(
                            "`{callee_short}` does not support {}; expected {expected}",
                            actual.display()
                        ),
                        argument.span,
                        vec![format!("pass {expected} to `{callee_short}`")],
                    );
                }
                if matches!(
                    target.as_str(),
                    crate::memory::SWAP_TARGET | crate::memory::REPLACE_TARGET
                ) {
                    if target == crate::memory::SWAP_TARGET
                        && matches!(
                            args,
                            [left, right]
                                if matches!((left.kind(), right.kind()), (ExprKind::Ident(left), ExprKind::Ident(right)) if left == right)
                        )
                    {
                        self.error_at_with_help(
                            "E3247",
                            format!("`{callee_short}` requires two distinct mutable bindings"),
                            args[1].span,
                            vec![format!("pass a different mutable binding as the second argument to `{callee_short}`")],
                        );
                    }
                    let binding_arguments = if target == crate::memory::SWAP_TARGET {
                        args
                    } else {
                        &args[..1]
                    };
                    for binding in binding_arguments {
                        let ExprKind::Ident(name) = binding.kind() else {
                            self.error_at_with_help(
                                "E3245",
                                format!("`{callee_short}` requires a mutable local binding"),
                                binding.span,
                                vec![format!("pass a mutable `let` binding to `{callee_short}`")],
                            );
                            continue;
                        };
                        let supported = self.lookup(name).is_some_and(|symbol| {
                            symbol.kind == SymbolKind::Variable
                                && symbol.mutable
                                && matches!(
                                    &symbol.ty,
                                    ValueType::Known(ty)
                                        if ty.is_copy()
                                            || matches!(transparent_type(ty), Type::Str)
                                )
                        });
                        if !supported {
                            let expected = "a mutable binding of a `Copy` type or `string`";
                            self.error_at_with_help(
                                "E3245",
                                format!("`{callee_short}` requires `{name}` to be {expected}"),
                                binding.span,
                                vec![format!("declare `{name}` with `let` and {expected}")],
                            );
                        }
                    }
                    if target == crate::memory::REPLACE_TARGET
                        && matches!(
                            actuals.first(),
                            Some(ValueType::Known(ty)) if matches!(transparent_type(ty), Type::Str)
                        )
                    {
                        let source = args.get(1);
                        let supported_source = source.is_some_and(|source| match source.kind() {
                            ExprKind::Ident(name) => self.lookup(name).is_some_and(|symbol| {
                                symbol.kind == SymbolKind::Variable
                                    && symbol.mutable
                                    && matches!(&symbol.ty, ValueType::Known(ty) if matches!(transparent_type(ty), Type::Str))
                            }),
                            _ => false,
                        });
                        if !supported_source {
                            let span = source.map(|source| source.span).unwrap_or(span);
                            self.error_at_with_help(
                                "E3246",
                                format!("`{callee_short}` requires its `string` source to be a mutable local binding"),
                                span,
                                vec![format!("pass a mutable `string` binding as the source to `{callee_short}`")],
                            );
                        }
                    }
                }
            }
            self.validate_decorated_call_arguments(callee, args);
            if is_channel_shaped(&type_params, &params, &ret) {
                if let Some(payload) = substitutions.get(&type_params[0].name)
                    && !is_supported_channel_payload(payload)
                {
                    self.error(
                            "E3036",
                            format!(
                                "`channel<{payload}>` is not supported yet: channel payloads are limited to `string`, `Copy` scalars, and arrays of `Copy` elements"
                            ),
                            Some(callee),
                        );
                }
                self.check_positive_capacity(
                    args.first(),
                    "channel",
                    "a capacity of at least 1",
                    "a channel with no room holds nothing: every `send` would wait for a `recv` that has nowhere to take the value from",
                    "pass a capacity of at least 1, or omit the argument to use the default",
                );
            }
            if is_mutex_shaped(&type_params, &params, &ret)
                && let Some(payload) = substitutions.get(&type_params[0].name)
                && !is_supported_mutex_payload(payload)
            {
                self.error(
                            "E3037",
                            format!(
                                "`Mutex<{payload}>` is not supported yet: mutex payloads are limited to `string`, `Copy` scalars, and arrays of `Copy` elements"
                            ),
                            Some(callee),
                        );
            }
            if is_rwlock_shaped(&type_params, &params, &ret)
                && let Some(payload) = substitutions.get(&type_params[0].name)
                && !is_supported_rwlock_payload(payload)
            {
                self.error(
                            "E3038",
                            format!(
                                "`RwLock<{payload}>` is not supported yet: RwLock payloads are limited to `string`, `Copy` scalars, and arrays of `Copy` elements"
                            ),
                            Some(callee),
                        );
            }
            if is_join_handle_array_consuming_shaped(&type_params, &params)
                && let Some(ExprKind::ArrayLit(elements)) = args.first().map(Expr::kind)
            {
                for element in elements {
                    let ExprKind::Ident(name) = element.kind() else {
                        continue;
                    };
                    if self.is_join_handle_consumed(name, element.span) {
                        self.error_at(
                                "E3033",
                                format!(
                                    "`{name}` was already consumed by an earlier `.join()`, `joinAll`, or `race`; a `JoinHandle<T>` can only be consumed once"
                                ),
                                element.span,
                            );
                    } else {
                        self.mark_join_handle_consumed(name, element.span);
                    }
                }
            }
            if is_db_query {
                return ValueType::Known(self.check_db_query_call(type_args, args, span));
            }
            let return_type = substitute_generics(&ret, &substitutions);
            return ValueType::Known(self.resolve_type_annotation(&return_type));
        }

        let ValueType::Known(Type::Function {
            params: sig_params,
            ret,
        }) = callee_ty
        else {
            for arg in args {
                self.check_expr(arg);
            }
            if !matches!(callee_ty, ValueType::Unknown | ValueType::Error) {
                self.error("E0618", format!("`{callee}` is not callable"), Some(callee));
            }
            return if matches!(callee_ty, ValueType::Unknown) {
                ValueType::Unknown
            } else {
                ValueType::Error
            };
        };

        if matches!(sig_params.as_slice(), [Type::Usize])
            && matches!(ret.as_ref(), Type::Named(name) if name == "Semaphore")
        {
            self.check_positive_capacity(
                args.first(),
                callee,
                "at least 1 permit",
                "a semaphore with no permits can never be acquired, so every `acquire` would wait forever",
                "pass a permit count of at least 1",
            );
        }

        self.check_archive_pack_literals(args, &sig_params, &ret);
        self.check_compress_literals(args, &sig_params, &ret);

        let has_rest = self
            .fn_param_metadata
            .get(callee)
            .and_then(|p| p.last())
            .is_some_and(|p| p.rest);
        let mandatory_count = self
            .fn_param_metadata
            .get(callee)
            .map(|meta| {
                meta.iter()
                    .filter(|p| !p.optional && p.default_value.is_none() && !p.rest)
                    .count()
            })
            .unwrap_or_else(|| {
                sig_params.len().saturating_sub(
                    native_optional_trailing_param_count(callee).max(
                        sig_params
                            .iter()
                            .rev()
                            .take_while(|parameter| matches!(parameter, Type::Optional(_)))
                            .count(),
                    ),
                )
            });

        let max_args = if has_rest {
            usize::MAX
        } else {
            sig_params.len()
        };
        if args.len() < mandatory_count {
            self.error(
                "E0061",
                format!(
                    "function `{callee}` expects at least {mandatory_count} argument(s), but {} were provided",
                    args.len()
                ),
                Some(callee),
            );
        } else if args.len() > max_args {
            self.error(
                "E0061",
                format!(
                    "function `{callee}` expects at most {max_args} argument(s), but {} were provided",
                    args.len()
                ),
                Some(callee),
            );
        }
        let fixed_count = if has_rest {
            sig_params.len().saturating_sub(1)
        } else {
            sig_params.len()
        };
        for (index, arg) in args.iter().enumerate().take(fixed_count) {
            let expected = sig_params.get(index);
            let before = self.diagnostics.len();
            let actual = expected
                .map(|expected| self.check_expr_as(arg, expected))
                .unwrap_or_else(|| self.check_expr(arg));
            let reported_specifically = self.diagnostics.len() > before;
            if let Some(expected) = expected {
                let label = format!("argument {} of `{callee}`", index + 1);
                if !reported_specifically && !self.is_assignable(expected, &actual) {
                    if let Some((name, module)) = self.unimported_native_interface(expected) {
                        self.error_at_with_help(
                            "E0412",
                            format!("cannot find type `{name}` in this scope"),
                            arg.span,
                            vec![format!(
                                "{label} expects `{name}`, which is exported by `{module}`; add `import {{ {name} }} from \"{module}\";`"
                            )],
                        );
                    } else {
                        self.error(
                            "E0308",
                            format!(
                                "type mismatch for `{label}`: expected `{expected}`, found `{}`",
                                actual.display()
                            ),
                            expression_subject(arg).or(Some(callee)),
                        );
                    }
                }
                let expected = expected.clone();
                self.require_integer_literal_fits(&expected, arg);
                if !self.callee_has_native_signature(callee) {
                    self.require_unknown_representable_argument(&expected, &actual, arg, &label);
                }
            }
        }
        self.validate_decorated_call_arguments(callee, args);
        if has_rest {
            let rest_array_type = sig_params.last().cloned();
            let rest_element = rest_array_type.as_ref().and_then(|ty| match ty {
                Type::Array(element) => Some(element.as_ref().clone()),
                _ => None,
            });
            for (index, arg) in args.iter().enumerate().skip(fixed_count) {
                let label = format!("argument {} of `{callee}`", index + 1);
                if let ExprKind::Spread(inner) = arg.kind() {
                    let actual = self.check_expr(inner);
                    if let Some(expected) = &rest_array_type
                        && !self.is_assignable(expected, &actual)
                    {
                        self.error(
                                "E0308",
                                format!(
                                    "type mismatch for spread `{label}`: expected `{expected}`, found `{}`",
                                    actual.display()
                                ),
                                expression_subject(inner).or(Some(callee)),
                            );
                    }
                } else if let Some(rest_element) = &rest_element {
                    let actual = self.check_expr_as(arg, rest_element);
                    if !self.callee_has_native_signature(callee) {
                        self.require_unknown_representable_argument(
                            rest_element,
                            &actual,
                            arg,
                            &label,
                        );
                    }
                    if !self.is_assignable(rest_element, &actual) {
                        self.error(
                            "E0308",
                            format!(
                                "type mismatch for `{label}`: expected `{rest_element}`, found `{}`",
                                actual.display()
                            ),
                            expression_subject(arg).or(Some(callee)),
                        );
                    }
                }
            }
        }
        if let Some(result) =
            self.check_algorithm_constructor_call(callee, &callee_short, type_args, args)
        {
            return result;
        }
        let return_type = self.resolve_type_annotation(&ret);
        ValueType::Known(return_type)
    }

    pub(super) fn check_file_encoding_argument(&mut self, callee: &str, args: &[Expr]) {
        let target = self
            .import_targets
            .get(callee)
            .map(String::as_str)
            .unwrap_or(callee);
        let encoding_index = match target {
            "__tyrus_fs_readFileSync" => 1,
            "__tyrus_fs_writeFileSync" => 2,
            _ => return,
        };
        let Some(argument) = args.get(encoding_index) else {
            return;
        };
        match argument.kind() {
            ExprKind::StrLit(encoding) if encoding == "utf8" || encoding == "utf-8" => {}
            ExprKind::StrLit(encoding) => {
                self.error_at_with_help(
                    "E0790",
                    format!("unsupported file encoding `{encoding}`"),
                    argument.span,
                    vec![
                        "use `\"utf8\"` or `\"utf-8\"`; other encodings are not supported yet"
                            .to_string(),
                    ],
                );
            }
            _ => {
                self.error_at_with_help(
                    "E0790",
                    "file encoding must be a string literal".to_string(),
                    argument.span,
                    vec![
                        "write the encoding inline as `\"utf8\"`; it is resolved at compile time, so a variable cannot be used"
                            .to_string(),
                    ],
                );
            }
        }
    }

    pub(super) fn check_call_effects(&mut self, callee: &str, args: &[Expr], span: Span) {
        let mut effects = self.fn_throws.get(callee).cloned().unwrap_or_default();
        if effects.is_empty()
            && let Some(ValueType::Known(Type::Throws {
                effects: declared, ..
            })) = self.lookup(callee).map(|symbol| symbol.ty.clone())
        {
            effects = declared;
        }
        for effect in effects {
            if effect == Type::Named("ValidationError".to_string())
                && self.decorated_call_is_compile_time_validated(callee, args)
            {
                continue;
            }
            let synchronous = self.effect_reaches_the_caller_as_a_jump(callee);
            self.check_unhandled_effect(&effect, &format!("call to `{callee}`"), span, synchronous);
        }
    }

    pub(super) fn effect_reaches_the_caller_as_a_jump(&self, callee: &str) -> bool {
        if self.async_fns.contains(callee) {
            return false;
        }
        let returns_promise = self
            .lookup(callee)
            .and_then(|symbol| value_type_to_type(&symbol.ty))
            .is_some_and(|ty| {
                matches!(transparent_type(&ty), Type::Function { ret, .. } if promise_payload(ret).is_some())
            });
        !returns_promise
    }

    pub(super) fn decorated_call_is_compile_time_validated(
        &self,
        callee: &str,
        args: &[Expr],
    ) -> bool {
        let Some(params) = self.fn_param_metadata.get(callee) else {
            return false;
        };
        let mut has_decorators = false;
        for (param, arg) in params.iter().zip(args) {
            if param.decorators.is_empty() {
                continue;
            }
            has_decorators = true;
            if !is_validator_literal(arg) {
                return false;
            }
        }
        has_decorators
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn check_method_call(
        &mut self,
        object: &Expr,
        method: &str,
        method_span: Option<Span>,
        type_args: &[Type],
        args: &[Expr],
        optional: bool,
    ) -> ValueType {
        if !optional && let ExprKind::Ident(model_name) = object.kind() {
            if self.db_models.contains_key(model_name) && method == "findMany" {
                self.mark_used(model_name);
                return ValueType::Known(self.check_db_find_many_call(
                    &[Type::Named(model_name.clone())],
                    args,
                    method_span.unwrap_or(object.span),
                ));
            }
            if self.db_models.contains_key(model_name) && method == "findOne" {
                self.mark_used(model_name);
                return ValueType::Known(self.check_db_find_one_call(
                    &[Type::Named(model_name.clone())],
                    args,
                    method_span.unwrap_or(object.span),
                ));
            }
            if self.db_models.contains_key(model_name) && method == "count" {
                self.mark_used(model_name);
                return ValueType::Known(self.check_db_count_call(
                    &[Type::Named(model_name.clone())],
                    args,
                    method_span.unwrap_or(object.span),
                ));
            }
            if self.db_models.contains_key(model_name)
                && crate::db::model_method_target(method).is_some()
            {
                self.mark_used(model_name);
                return ValueType::Known(self.check_db_model_mutation_call(
                    method,
                    model_name,
                    args,
                    method_span.unwrap_or(object.span),
                ));
            }
        }
        if !optional && let ExprKind::Ident(name) = object.kind() {
            let target = self
                .import_targets
                .get(name)
                .map(String::as_str)
                .unwrap_or(name);
            if crate::test::is_registrar_target(target) {
                let target = target.to_string();
                self.mark_used(name);
                return self.check_test_modifier_call(&target, method, method_span, args);
            }
        }
        if !optional
            && let ExprKind::Ident(type_name) = object.kind()
            && self.data_interfaces.contains(type_name)
            && let Some(result) = self.check_automatic_data_static_method(
                type_name,
                method,
                method_span.unwrap_or(object.span),
                args,
            )
        {
            return result;
        }
        if !optional
            && let ExprKind::Ident(name) = object.kind()
            && name == "fetch"
            && self
                .lookup(name)
                .is_some_and(|symbol| symbol.kind == SymbolKind::Import)
        {
            if crate::http_body::fetch_verb_method(method).is_some() {
                return self.check_fetch_verb(method, args);
            }
            if method == "stream" {
                return self.check_fetch_stream(args);
            }
        }
        if !optional && method == "copy" && args.is_empty() {
            let receiver = self.check_expr(object);
            if let ValueType::Known(ty) = &receiver {
                let resolved = transparent_type(ty).clone();
                if !resolved.is_copy()
                    && matches!(
                        resolved,
                        Type::Str | Type::Array(_) | Type::Tuple(_) | Type::Optional(_)
                    )
                {
                    return ValueType::Known(resolved);
                }
            }
        }
        if !optional
            && let ExprKind::Ident(type_name) = object.kind()
            && let Some(enum_info) = self.enums.get(type_name).cloned()
            && let Some((_, _)) = enum_info.members.iter().find(|(m, _)| m == method)
        {
            let member_index = enum_info
                .members
                .iter()
                .position(|(m, _)| m == method)
                .unwrap();
            match &enum_info.member_kinds[member_index] {
                EnumPayload::Unit => {
                    if !args.is_empty() {
                        self.error(
                            "E0061",
                            format!(
                                "`{type_name}.{method}` takes no arguments, but {} were provided",
                                args.len()
                            ),
                            Some(method),
                        );
                    }
                }
                EnumPayload::Tuple(types) => {
                    if args.len() != types.len() {
                        self.error(
                                        "E0061",
                                        format!(
                                            "`{type_name}.{method}` expects {} argument(s), but {} were provided",
                                            types.len(),
                                            args.len()
                                        ),
                                        Some(method),
                                    );
                    }
                    for (arg, expected) in args.iter().zip(types) {
                        let actual = self.check_expr(arg);
                        if !self.is_assignable(expected, &actual)
                            && let ValueType::Known(actual_ty) = &actual
                        {
                            self.error(
                                "E0308",
                                format!("expected `{expected}`, found `{actual_ty}`",),
                                expression_subject(arg),
                            );
                        }
                    }
                }
                EnumPayload::Struct(fields) => {
                    if args.len() != 1 {
                        self.error(
                                        "E0061",
                                        format!(
                                            "`{type_name}.{method}` expects 1 argument (an object), but {} were provided",
                                            args.len()
                                        ),
                                        Some(method),
                                    );
                    } else if let Some(arg) = args.first() {
                        let actual = self.check_expr(arg);
                        if !self.is_assignable(&Type::Object(fields.clone()), &actual)
                            && let ValueType::Known(actual_ty) = &actual
                        {
                            self.error(
                                "E0308",
                                format!(
                                    "expected `{{ {} }}`, found `{actual_ty}`",
                                    fields
                                        .iter()
                                        .map(|f| format!("{}: {}", f.name, f.ty))
                                        .collect::<Vec<_>>()
                                        .join(", "),
                                ),
                                expression_subject(arg),
                            );
                        }
                    }
                }
            }
            self.mark_used(type_name);
            return ValueType::Known(Type::Named(type_name.clone()));
        }
        if !optional
            && matches!(object.kind(), ExprKind::Ident(name) if name == "Array")
            && !self.has_symbol("Array")
        {
            return self.check_array_static_method(method, args);
        }
        if !optional
            && matches!(object.kind(), ExprKind::Ident(name) if name == "Promise")
            && !self.has_symbol("Promise")
        {
            return self.check_promise_static_method(method, type_args, args);
        }
        if !optional
            && let ExprKind::Ident(name) = object.kind()
            && matches!(name.as_str(), "BitSet" | "LinkedList" | "DoubleLinkedList")
        {
            return self.check_algorithm_static_method(name, method, args);
        }
        if matches!(object.kind(), ExprKind::Ident(name) if name == "process")
            && !self.has_symbol("process")
        {
            let ordinal = self.expressions.len() as u32;
            self.expressions.push(InferredExpression {
                ordinal,
                ty: "process".to_string(),
            });
            if !matches!(
                method,
                "exit" | "cwd" | "uptime" | "memoryUsage" | "cpuUsage"
            ) {
                self.error(
                    "E3145",
                    format!("`process` has no method `{method}`; only `process.exit`, `process.cwd`, `process.uptime`, `process.memoryUsage`, and `process.cpuUsage` are available"),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
                return ValueType::Error;
            }
            if method != "exit" {
                if !args.is_empty() {
                    self.error(
                        "E0061",
                        format!(
                            "`process.{method}` expects 0 arguments, but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                }
                return ValueType::Known(match method {
                    "cwd" => Type::Str,
                    "uptime" => Type::U64,
                    "memoryUsage" => Type::Named("MemoryInfo".to_string()),
                    _ => Type::Named("CpuUsage".to_string()),
                });
            }
            if args.len() != 1 {
                self.error(
                    "E0061",
                    format!(
                        "`process.exit` expects 1 argument, but {} were provided",
                        args.len()
                    ),
                    Some(method),
                );
            }
            for arg in args {
                let actual = self.check_expr(arg);
                self.require_assignable(&Type::I32, &actual, "exit", "E0308");
            }
            return ValueType::Known(Type::Never);
        }
        if matches!(object.kind(), ExprKind::Ident(name) if name == "console") {
            let ordinal = self.expressions.len() as u32;
            self.expressions.push(InferredExpression {
                ordinal,
                ty: "console".to_string(),
            });
            if !matches!(method, "log" | "error" | "warn") {
                self.error(
                    "E0599",
                    format!("unknown console method `{method}`"),
                    Some(method),
                );
            }
            for arg in args {
                let ty = self.check_expr(arg);
                if !is_console_value(&ty) {
                    self.error(
                        "E0277",
                        format!("console output does not support type `{}`", ty.display()),
                        expression_subject(arg),
                    );
                }
            }
            return ValueType::Known(Type::Void);
        }

        let object_ty = self.check_expr(object);
        if !optional
            && matches!(
                &object_ty,
                ValueType::Known(Type::Named(name)) if name == crate::test::ASSERTION_TYPE
            )
        {
            let expectation = crate::test::expectation_actual(object, |callee| {
                self.import_targets
                    .get(callee)
                    .map(String::as_str)
                    .unwrap_or(callee)
                    == crate::test::EXPECT_TARGET
            });
            return self.check_test_assertion_method(method, method_span, args, expectation);
        }
        let object_ty = match &object_ty {
            ValueType::Known(Type::Union(variants)) => common_underlying_type(variants)
                .map(ValueType::Known)
                .unwrap_or(object_ty),
            _ => object_ty,
        };
        let (object_ty, optional_receiver) = if optional {
            match object_ty {
                ValueType::Known(Type::Optional(inner)) => (ValueType::Known(*inner), true),
                other @ (ValueType::Unknown | ValueType::Error) => (other, true),
                other => {
                    self.error(
                        "E0277",
                        format!(
                            "optional chaining requires an optional value, found `{}`",
                            other.display()
                        ),
                        expression_subject(object),
                    );
                    (other, false)
                }
            }
        } else {
            (object_ty, false)
        };
        let object_ty = match &object_ty {
            ValueType::Array(_) => match concrete_value_type(&object_ty) {
                Some(settled) => ValueType::Known(settled),
                None => object_ty,
            },
            _ => object_ty,
        };
        if let ValueType::Known(receiver_ty) = &object_ty {
            let matching_extensions = self
                .extension_methods
                .iter()
                .filter(|candidate| {
                    candidate.name == method
                        && self.is_assignable(
                            &candidate.target,
                            &ValueType::Known(receiver_ty.clone()),
                        )
                })
                .cloned()
                .collect::<Vec<_>>();
            let mut owners = matching_extensions
                .iter()
                .map(|extension| extension.owner.clone())
                .collect::<Vec<_>>();
            owners.sort();
            owners.dedup();
            if owners.len() > 1 {
                for arg in args {
                    self.check_expr(arg);
                }
                self.error_at_with_help(
                    "E0034",
                    format!(
                        "multiple extension methods named `{method}` apply to `{receiver_ty}` through {}",
                        owners
                            .iter()
                            .map(|owner| format!("`{owner}`"))
                            .collect::<Vec<_>>()
                            .join(" and ")
                    ),
                    method_span.unwrap_or(object.span),
                    vec![format!(
                        "remove one behavior interface, narrow its target, or rename `{method}` so dispatch is unambiguous"
                    )],
                );
                return ValueType::Error;
            }
            if let Some(extension) = matching_extensions.last().cloned() {
                let expected_args = extension.params.len().saturating_sub(1);
                if args.len() != expected_args {
                    self.error(
                        "E0061",
                        format!(
                            "extension method `{method}` expects {expected_args} argument(s), but {} were provided",
                            args.len()
                        ),
                        Some(method),
                    );
                }
                for (index, arg) in args.iter().enumerate() {
                    let actual = self.check_expr(arg);
                    if let Some(parameter) = extension.params.get(index + 1) {
                        self.require_assignable(&parameter.ty, &actual, method, "E0308");
                    }
                }
                return optional_value_type(
                    ValueType::Known(extension.ret_type),
                    optional_receiver,
                );
            }
        }
        if let Some(result) = self.check_automatic_data_method(
            &object_ty,
            method,
            method_span.unwrap_or(object.span),
            args,
        ) {
            return optional_value_type(result, optional_receiver);
        }
        if matches!(method, "pick" | "omit") {
            let result = match &object_ty {
                ValueType::Known(receiver_ty) => {
                    self.check_object_projection_method(object, receiver_ty, method, args)
                }
                _ => {
                    for arg in args {
                        self.check_expr(arg);
                    }
                    ValueType::Error
                }
            };
            return optional_value_type(result, optional_receiver);
        }
        if let ValueType::Known(ty) = &object_ty {
            if let Some(payload) = promise_payload(ty) {
                let result = self.check_promise_instance_method(payload, method, args);
                return optional_value_type(result, optional_receiver);
            }
            if let Some(payload) = join_handle_payload(ty) {
                if matches!(method, "join" | "abort")
                    && let ExprKind::Ident(name) = object.kind()
                {
                    let call_span = method_span.unwrap_or(object.span);
                    if self.is_join_handle_consumed(name, call_span) {
                        self.error_at(
                                "E3033",
                                format!(
                                    "`{name}` was already consumed by an earlier `.join()`; a `JoinHandle<T>` can only be joined once"
                                ),
                                call_span,
                            );
                    } else if method == "join" {
                        self.mark_join_handle_consumed(name, call_span);
                    }
                }
                let result = self.check_join_handle_instance_method(payload, method, args);
                return optional_value_type(result, optional_receiver);
            }
            if let Some(payload) = sender_payload(ty) {
                let result = self.check_sender_instance_method(payload, method, args);
                return optional_value_type(result, optional_receiver);
            }
            if let Some(payload) = readable_stream_payload(ty) {
                let result = self.check_readable_stream_instance_method(payload, method, args);
                return optional_value_type(result, optional_receiver);
            }
            if let Some(payload) = writable_stream_payload(ty) {
                let result = self.check_writable_stream_instance_method(payload, method, args);
                return optional_value_type(result, optional_receiver);
            }
            if let Type::Apply {
                name,
                args: type_args,
            } = transparent_type(ty)
                && matches!(name.as_str(), "DuplexStream" | "TransformStream")
                && type_args.len() == 2
            {
                let result = self.check_composite_stream_instance_method(name, method, args);
                return optional_value_type(result, optional_receiver);
            }
            if let Some(payload) = receiver_payload(ty) {
                let result = self.check_receiver_instance_method(payload, method, args);
                return optional_value_type(result, optional_receiver);
            }
            if let Some(payload) = mutex_payload(ty) {
                let result = self.check_mutex_instance_method(payload, method, args);
                return optional_value_type(result, optional_receiver);
            }
            if let Some(payload) = rwlock_payload(ty) {
                let result = self.check_rwlock_instance_method(payload, method, args);
                return optional_value_type(result, optional_receiver);
            }
            if matches!(ty, Type::Named(name) if name == "Semaphore") {
                let result = self.check_semaphore_instance_method(method, args);
                return optional_value_type(result, optional_receiver);
            }
        }
        if method == "toString" && is_stringifiable_scalar(&object_ty) {
            if !args.is_empty() {
                self.error(
                    "E0061",
                    format!(
                        "`toString` expects 0 arguments, but {} were provided",
                        args.len()
                    ),
                    Some(method),
                );
                for arg in args {
                    self.check_expr(arg);
                }
            }
            return optional_value_type(ValueType::Known(Type::Str), optional_receiver);
        }
        if matches!(
            method,
            "algebraicAdd" | "algebraicSub" | "algebraicMul" | "algebraicDiv" | "algebraicRem"
        ) && matches!(&object_ty, ValueType::Known(Type::F32 | Type::F64))
        {
            if args.len() != 1 {
                self.error(
                    "E0061",
                    format!(
                        "`{method}` expects 1 argument, but {} were provided",
                        args.len()
                    ),
                    Some(method),
                );
            }
            let receiver = match &object_ty {
                ValueType::Known(ty) => ty.clone(),
                _ => unreachable!(),
            };
            for arg in args {
                let actual = self.check_expr(arg);
                self.require_assignable(&receiver, &actual, method, "E0308");
            }
            return optional_value_type(ValueType::Known(receiver), optional_receiver);
        }
        if let Some(result) = self.check_container_receiver_method(
            object,
            method,
            args,
            &object_ty,
            optional_receiver,
        ) {
            return result;
        }
        if let Some(result) =
            self.check_text_receiver_method(method, args, &object_ty, optional_receiver)
        {
            return result;
        }
        if let Some(result) = self.check_native_receiver_method(
            object,
            method,
            method_span,
            type_args,
            args,
            MethodReceiver {
                ty: &object_ty,
                optional: optional_receiver,
            },
        ) {
            return result;
        }
        let function_field = match &object_ty {
            ValueType::Known(Type::Named(name)) => self
                .interfaces
                .get(name)
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::BinaryHeap(_)) => self
                .interfaces
                .get("BinaryHeap")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::BinomialHeap(_)) => self
                .interfaces
                .get("BinomialHeap")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::UnionFind(_)) => self
                .interfaces
                .get("UnionFind")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::Counter(_)) => self
                .interfaces
                .get("Counter")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::Deque(_)) => self
                .interfaces
                .get("Deque")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::FenwickTree) => self
                .interfaces
                .get("FenwickTree")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::BubbleSort) => self
                .interfaces
                .get("BubbleSort")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::SegmentTree) => self
                .interfaces
                .get("SegmentTree")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::SparseTable) => self
                .interfaces
                .get("SparseTable")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::SegmentTreeLazy) => self
                .interfaces
                .get("SegmentTreeLazy")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::BitSet) => self
                .interfaces
                .get("BitSet")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::LinkedList(_)) => self
                .interfaces
                .get("LinkedList")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::DoubleLinkedList(_)) => self
                .interfaces
                .get("DoubleLinkedList")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::AvlTree(_)) => self
                .interfaces
                .get("AvlTree")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::RedBlackTree(_)) => self
                .interfaces
                .get("RedBlackTree")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::SplayTree(_)) => self
                .interfaces
                .get("SplayTree")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::Treap(_)) => self
                .interfaces
                .get("Treap")
                .and_then(|fields| fields.iter().find(|field| field.name == method))
                .map(|field| field.ty.clone()),
            ValueType::Known(Type::Object(fields)) => fields
                .iter()
                .find(|field| field.name == method)
                .map(|field| field.ty.clone()),
            _ => None,
        };
        let generic_function_field =
            matches!(&object_ty, ValueType::Known(Type::Named(name)) if name == "JwtNamespace")
                .then_some(function_field.clone())
                .flatten();
        if let Some(Type::GenericFunction {
            type_params,
            params,
            ret,
        }) = generic_function_field
        {
            if !type_args.is_empty() && type_args.len() != type_params.len() {
                self.error(
                    "E0107",
                    format!(
                        "method `{method}` expects {} type argument(s), but {} were provided",
                        type_params.len(),
                        type_args.len()
                    ),
                    Some(method),
                );
            }
            let actuals = args
                .iter()
                .map(|argument| self.check_expr(argument))
                .collect::<Vec<_>>();
            let minimum = params
                .iter()
                .rposition(|parameter| !matches!(parameter, Type::Optional(_)))
                .map_or(0, |index| index + 1);
            if args.len() < minimum || args.len() > params.len() {
                let expectation = if minimum == params.len() {
                    params.len().to_string()
                } else {
                    format!("{minimum} to {}", params.len())
                };
                self.error(
                    "E0061",
                    format!(
                        "`{method}` expects {expectation} argument(s), but {} were provided",
                        args.len()
                    ),
                    Some(method),
                );
            }
            let mut substitutions = HashMap::default();
            for (parameter, explicit) in type_params.iter().zip(type_args) {
                substitutions.insert(parameter.name.clone(), explicit.clone());
            }
            for (expected, actual) in params.iter().zip(&actuals) {
                infer_generic_types(expected, actual, &mut substitutions);
            }
            for parameter in &type_params {
                if !substitutions.contains_key(&parameter.name) {
                    self.error(
                        "E0282",
                        format!(
                            "cannot infer type parameter `{}` for method `{method}`",
                            parameter.name
                        ),
                        Some(method),
                    );
                }
                if let (Some(constraint), Some(actual)) =
                    (&parameter.constraint, substitutions.get(&parameter.name))
                    && !self.is_assignable(constraint, &ValueType::Known(actual.clone()))
                {
                    self.error(
                        "E0277",
                        format!("type `{actual}` does not satisfy constraint `{constraint}`"),
                        Some(method),
                    );
                }
            }
            for (index, (expected, actual)) in params.iter().zip(&actuals).enumerate() {
                let expected = substitute_generics(expected, &substitutions);
                if !self.is_assignable(&expected, actual) {
                    self.span_stack.push(args[index].span);
                    self.error(
                        "E0308",
                        format!(
                            "type mismatch for argument {} of `{method}`: expected `{expected}`, found `{}`",
                            index + 1,
                            actual.display()
                        ),
                        Some(method),
                    );
                    self.span_stack.pop();
                }
            }
            let return_type = substitute_generics(&ret, &substitutions);
            if matches!(&object_ty, ValueType::Known(Type::Named(name)) if name == "JwtNamespace")
                && matches!(
                    method,
                    "sign" | "verify" | "verifyWithJwks" | "decodeUnverified"
                )
            {
                let claim_type = substitutions.get("T").cloned();
                if let Some(claim_type) = claim_type {
                    self.check_jwt_claim_type(&claim_type, method);
                }
            }
            if let ValueType::Known(Type::Named(namespace)) = &object_ty
                && crate::crypto::crypto_method_throws(namespace, method)
            {
                self.check_unhandled_effect(
                    &Type::Named("Error".to_string()),
                    &format!("call to `{method}`"),
                    method_span.unwrap_or(object.span),
                    true,
                );
            }
            return optional_value_type(ValueType::Known(return_type), optional_receiver);
        }
        if !type_args.is_empty() {
            self.error(
                "E0107",
                format!("method `{method}` does not accept explicit type arguments"),
                Some(method),
            );
        }
        if let Some(Type::Function { params, ret }) = function_field {
            let minimum = params
                .iter()
                .rposition(|parameter| !matches!(parameter, Type::Optional(_)))
                .map_or(0, |index| index + 1);
            if args.len() < minimum || args.len() > params.len() {
                let expectation = if minimum == params.len() {
                    format!("{}", params.len())
                } else {
                    format!("{minimum} to {}", params.len())
                };
                self.error(
                    "E0061",
                    format!(
                        "`{method}` expects {expectation} argument(s), but {} were provided",
                        args.len()
                    ),
                    Some(method),
                );
            }
            for (index, arg) in args.iter().enumerate() {
                match params.get(index) {
                    Some(expected) => {
                        let actual = self.check_expr_as(arg, expected);
                        self.span_stack.push(arg.span);
                        self.require_assignable(expected, &actual, method, "E0308");
                        self.span_stack.pop();
                    }
                    None => {
                        self.check_expr(arg);
                    }
                }
            }
            if let ValueType::Known(Type::Named(namespace)) = &object_ty
                && crate::crypto::crypto_method_throws(namespace, method)
            {
                self.check_unhandled_effect(
                    &Type::Named("Error".to_string()),
                    &format!("call to `{method}`"),
                    method_span.unwrap_or(object.span),
                    true,
                );
            }
            return optional_value_type(ValueType::Known(*ret), optional_receiver);
        }
        for arg in args {
            self.check_expr(arg);
        }
        self.error(
            "E0599",
            format!(
                "no method named `{method}` for type `{}`",
                object_ty.display()
            ),
            Some(method),
        );
        ValueType::Error
    }

    pub(super) fn check_test_modifier_call(
        &mut self,
        target: &str,
        method: &str,
        method_span: Option<Span>,
        args: &[Expr],
    ) -> ValueType {
        use crate::test::{
            TestModifier, UNIMPLEMENTED_MODIFIERS, registrar_display, test_modifier,
        };
        let owner = registrar_display(target);
        let available = if owner == "describe" {
            "`describe.skip` or `describe.only`".to_string()
        } else {
            format!("`{owner}.skip`, `{owner}.only`, `{owner}.todo` or `{owner}.each`")
        };
        let Some(modifier) = test_modifier(target, method) else {
            let message = if UNIMPLEMENTED_MODIFIERS.contains(&method) {
                format!("`{owner}.{method}` is not implemented yet")
            } else {
                format!("no modifier named `{method}` on `{owner}`")
            };
            self.error_at_member_with_help(
                "E0599",
                message,
                method_span,
                method,
                vec![format!("the available modifiers are {available}")],
            );
            for arg in args {
                self.check_expr(arg);
            }
            return ValueType::Error;
        };
        let expected = match modifier {
            TestModifier::Todo => 1,
            TestModifier::Each => 3,
            _ => 2,
        };
        if args.len() != expected {
            let plural = if expected == 1 {
                "argument"
            } else {
                "arguments"
            };
            let shape = if modifier == TestModifier::Todo {
                format!("pass only the test name, as in `{owner}.{method}(\"name\")`")
            } else {
                format!(
                    "pass a name and a callback, as in `{owner}.{method}(\"name\", () => {{ ... }})`"
                )
            };
            self.error_at_member_with_help(
                "E0061",
                format!(
                    "`{owner}.{method}` expects {expected} {plural}, but {} were provided",
                    args.len()
                ),
                method_span,
                method,
                vec![shape],
            );
        }
        if let Some(name) = args.first() {
            let actual = self.check_expr(name);
            if !matches!(actual, ValueType::Unknown | ValueType::Error)
                && !self.is_assignable(&Type::Str, &actual)
            {
                let message = format!(
                    "the name passed to `{owner}.{method}` must be a `string`, found `{}`",
                    actual.display()
                );
                let example = if modifier == TestModifier::Todo {
                    format!("`{owner}.{method}(\"name\")`")
                } else {
                    format!("`{owner}.{method}(\"name\", () => {{ ... }})`")
                };
                let help = vec![format!("write the name as a string, as in {example}")];
                if name.span.is_dummy() {
                    self.error("E0308", message, expression_subject(name));
                } else {
                    self.error_at_with_help("E0308", message, name.span, help);
                }
            }
        }
        if modifier == TestModifier::Each {
            let cases = args.get(1).map(|cases| self.check_expr(cases));
            let element = match cases.as_ref().and_then(value_type_to_type) {
                Some(Type::Array(element)) => Some(element.as_ref().clone()),
                Some(other) => {
                    self.error_at_member(
                        "E0308",
                        format!("`{owner}.each` expects an array of cases, found `{other}`"),
                        method_span,
                        method,
                    );
                    None
                }
                None => None,
            };
            if let Some(cases) = args.get(1)
                && !matches!(cases.kind(), ExprKind::ArrayLit(_))
            {
                self.error_on_expr(
                    "E0658",
                    format!("`{owner}.each` needs its cases written inline, as an array literal"),
                    cases,
                );
                if let Some(last) = self.diagnostics.last_mut() {
                    last.help = vec![format!(
                        "write the cases in the call, as in `{owner}.each(\"name\", [1, 2, 3], callback)`"
                    )];
                }
            }
            if let Some(callback) = args.get(2) {
                if let ExprKind::ArrowFn { params, .. } = callback.kind()
                    && params.len() != 1
                {
                    self.error_on_expr(
                            "E0057",
                            format!(
                                "the callback of `{owner}.each` takes exactly one case, but it declares {}",
                                params.len()
                            ),
                            callback,
                        );
                }
                let expected_callback = element.map(|element| Type::Function {
                    params: vec![element],
                    ret: Box::new(Type::Void),
                });
                self.check_expr_with_expected(callback, expected_callback.as_ref());
            }
            return ValueType::Known(Type::Void);
        }
        for arg in args.iter().skip(1) {
            self.check_expr(arg);
        }
        ValueType::Known(Type::Void)
    }

    pub(super) fn is_spawn_task_call(&self, expr: &Expr) -> bool {
        let ExprKind::FnCall { callee, .. } = expr.kind() else {
            return false;
        };
        self.lookup(callee).is_some_and(|symbol| {
            matches!(
                &symbol.ty,
                ValueType::Known(Type::GenericFunction { type_params, params, ret })
                    if is_spawn_shaped(type_params, params, ret)
            )
        })
    }

    pub(super) fn validate_decorated_call_arguments(&mut self, callee: &str, args: &[Expr]) {
        let Some(params) = self.fn_param_metadata.get(callee).cloned() else {
            return;
        };
        for (param, arg) in params.iter().zip(args) {
            if param.decorators.is_empty() {
                continue;
            }
            let param_name = param
                .pattern
                .binding_names()
                .first()
                .copied()
                .unwrap_or("parameter")
                .to_string();
            for decorator in &param.decorators {
                if let Some(message) = validate_literal_value(decorator, arg, &param.ty) {
                    let validator = validator_display(decorator);
                    self.error_at(
                        "E3165",
                        format!("`{validator}` failed for `{param_name}`: {message}"),
                        arg.span,
                    );
                }
            }
        }
    }
}

impl TypeChecker {
    /// A parameter typed by a native interface the file never imported: the
    /// interface is unknown here, so any argument, an object literal above
    /// all, can only mismatch. Naming the missing import is the actionable
    /// report; a bare type mismatch against `object` is not.
    pub(super) fn unimported_native_interface(
        &self,
        expected: &Type,
    ) -> Option<(String, &'static str)> {
        let inner = match expected {
            Type::Optional(inner) => inner.as_ref(),
            other => other,
        };
        let Type::Named(name) = inner else {
            return None;
        };
        if self.interfaces.contains_key(name) || self.enums.contains_key(name) {
            return None;
        }
        crate::module_loader::native_module_exporting(name).map(|module| (name.clone(), module))
    }
}
