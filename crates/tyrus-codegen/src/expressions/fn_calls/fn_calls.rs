use super::*;

impl<'ctx> Codegen<'ctx> {
    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_fn_call_expr(
        &mut self,
        expr: &Expr,
        callee: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match callee {
            callee_str if tyrus_core::test::is_test_target(callee_str) => {
                if let Some(result) = self.emit_test_fn_call(callee_str, args)? {
                    return Ok(result);
                }
                return Err(format!(
                    "internal: `{callee_str}` is a test target but generated nothing"
                ));
            }
            tyrus_core::memory::ZEROIZE_TARGET => return self.emit_memory_zeroize(args),
            tyrus_core::memory::DROP_TARGET => return self.emit_memory_drop(args),
            tyrus_core::memory::SWAP_TARGET => return self.emit_memory_swap(args),
            tyrus_core::memory::REPLACE_TARGET => return self.emit_memory_replace(args),
            tyrus_core::memory::SIZE_OF_TARGET => {
                return self.emit_memory_layout_query(type_args, args, false);
            }
            tyrus_core::memory::ALIGN_OF_TARGET => {
                return self.emit_memory_layout_query(type_args, args, true);
            }
            "__tyrus_process_exec" => return self.emit_process_exec(args),
            _ => {}
        }
        if let Some(result) = self.emit_db_call(callee, type_args, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_simd_call(callee, type_args, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_algorithm_constructor_call(callee, type_args, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_math_fn_call(callee, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_temporal_plain_fn_call(callee, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_temporal_fn_call(callee, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_os_fn_call(callee, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_net_fn_call(callee, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_archive_fn_call(callee, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_compress_fn_call(callee, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_task_fn_call(callee, type_args, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_fs_fn_call(callee, args)? {
            return Ok(result);
        }
        if let Some(result) = self.emit_cli_prompt(callee, args) {
            return result;
        }
        let is_user_defined = self.fn_signatures.contains_key(callee)
            || self.generic_fns.contains_key(callee)
            || self.load_var(callee).is_some();
        if !is_user_defined && type_args.is_empty() {
            if let Some(result) = self.emit_conversion_builtin(callee, args)? {
                return Ok(result);
            }
            if let Some(result) = self.emit_float_predicate_builtin(callee, args)? {
                return Ok(result);
            }
            if let Some(result) = self.emit_assert_builtin(callee, args, expr.span)? {
                return Ok(result);
            }
            if let Some(result) = self.emit_print_builtin(callee, args)? {
                return Ok(result);
            }
        }
        let generic_call = if self.generic_fns.contains_key(callee) {
            Some(self.specialize_generic_function(callee, type_args, args)?)
        } else {
            None
        };
        let expected_params = generic_call
            .as_ref()
            .map(|(_, params, _)| params.clone())
            .or_else(|| {
                self.fn_signatures
                    .get(callee)
                    .map(|(params, _)| params.clone())
                    .or_else(|| {
                        self.load_var(callee).and_then(|(_, ty)| match ty {
                            Type::Function { params, .. } => Some(params),
                            _ => None,
                        })
                    })
            });
        let param_meta_clone = self.fn_param_metadata.get(callee).cloned().or_else(|| {
            self.generic_fns
                .get(callee)
                .map(|definition| definition.params.clone())
        });
        let has_param_meta = param_meta_clone.is_some();
        let has_rest = param_meta_clone
            .as_ref()
            .and_then(|m| m.last())
            .is_some_and(|p| p.rest);
        let mut compiled_args = Vec::with_capacity(
            expected_params
                .as_ref()
                .map_or(args.len(), std::vec::Vec::len),
        );
        let mut compiled_arg_values = Vec::with_capacity(
            expected_params
                .as_ref()
                .map_or(args.len(), std::vec::Vec::len),
        );
        let mut async_argument_cleanups = Vec::new();
        let mut borrowed_optional_boxes = Vec::new();
        let mut closure_arg_temporaries: Vec<BasicValueEnum<'ctx>> = Vec::new();
        let async_callee = self.async_functions.contains(callee);
        if let Some(expected) = &expected_params {
            let fixed_count = if has_rest {
                expected.len().saturating_sub(1)
            } else {
                expected.len()
            };
            for (index, param_type) in expected.iter().take(fixed_count).enumerate() {
                let value = if let Some(arg) = args.get(index) {
                    self.emit_expr_as(arg, param_type)?
                } else if has_param_meta {
                    if let Some(meta) = param_meta_clone.as_ref() {
                        if let Some(param) = meta.get(index) {
                            if let Some(default) = &param.default_value {
                                self.emit_expr_as(default, param_type)?
                            } else if param.optional || param.rest {
                                let ptr_ty =
                                    self.context.ptr_type(inkwell::AddressSpace::default());
                                ptr_ty.const_null().into()
                            } else {
                                let pattern_display = match &param.pattern {
                                    ParamPattern::Name(n) => n.clone(),
                                    ParamPattern::Array(names) => {
                                        format!("[{}]", names.join(", "))
                                    }
                                    ParamPattern::Object(names) => {
                                        format!("{{{}}}", names.join(", "))
                                    }
                                };
                                return Err(format!(
                                    "error: missing required argument for `{callee}` param `{pattern_display}`"
                                ));
                            }
                        } else {
                            self.emit_expr_as(
                                &Expr::synthetic(ExprKind::Null),
                                &Type::Optional(Box::new(param_type.clone())),
                            )?
                        }
                    } else {
                        self.emit_expr_as(
                            &Expr::synthetic(ExprKind::Null),
                            &Type::Optional(Box::new(param_type.clone())),
                        )?
                    }
                } else {
                    self.emit_expr_as(
                        &Expr::synthetic(ExprKind::Null),
                        &Type::Optional(Box::new(param_type.clone())),
                    )?
                };
                if let Some(argument) = args.get(index) {
                    let source_is_optional = self
                        .expr_type_hint(argument)
                        .map(|hint| self.resolve_type(&hint))
                        .is_some_and(|hint| {
                            matches!(transparent_codegen_type(&hint), Type::Optional(_))
                        });
                    if matches!(transparent_codegen_type(param_type), Type::Optional(_))
                        && !source_is_optional
                        && !matches!(argument.kind(), ExprKind::Null | ExprKind::None)
                    {
                        let tracked = (self.async_results.is_empty()
                            || self.async_temporaries_tracked > 0)
                            && self.optional_chain_depth == 0;
                        if tracked {
                            self.record_owned_native_result_with(
                                value,
                                param_type.clone(),
                                crate::ReleasePlan::Value,
                            );
                        } else if !async_callee {
                            borrowed_optional_boxes.push((argument.clone(), value));
                        }
                    }
                }
                if async_callee {
                    if let Some(argument) = args.get(index) {
                        async_argument_cleanups.push((value, param_type.clone(), argument.clone()));
                    } else if let Some(default) = param_meta_clone
                        .as_ref()
                        .and_then(|metadata| metadata.get(index))
                        .and_then(|parameter| parameter.default_value.as_ref())
                    {
                        async_argument_cleanups.push((
                            value,
                            param_type.clone(),
                            default.as_ref().clone(),
                        ));
                    }
                }
                if let Some(argument) = args.get(index)
                    && self.is_temporary_closure_argument(argument, param_type)
                {
                    closure_arg_temporaries.push(value);
                }
                compiled_args.push(value.into());
                compiled_arg_values.push(value);
            }
            if has_rest {
                let rest_type = &expected[fixed_count];
                let rest_elements: Vec<Expr> = args.iter().skip(fixed_count).cloned().collect();
                let rest_array = Expr::synthetic(ExprKind::ArrayLit(rest_elements));
                let value = self.emit_expr_as(&rest_array, rest_type)?;
                if async_callee {
                    async_argument_cleanups.push((value, rest_type.clone(), rest_array.clone()));
                }
                compiled_args.push(value.into());
                compiled_arg_values.push(value);
            }
        } else {
            for arg in args {
                let value = self.emit_expr(arg)?;
                compiled_args.push(value.into());
                compiled_arg_values.push(value);
            }
        }

        let validates_arguments = param_meta_clone.as_ref().is_some_and(|parameters| {
            parameters
                .iter()
                .any(|parameter| !parameter.decorators.is_empty())
        });
        if validates_arguments {
            self.emit_validation_issues_begin();
            if let (Some(parameters), Some(parameter_types)) =
                (param_meta_clone.as_ref(), expected_params.as_ref())
            {
                for ((parameter, parameter_type), value) in parameters
                    .iter()
                    .zip(parameter_types)
                    .zip(compiled_arg_values.iter().copied())
                {
                    let ParamPattern::Name(target) = &parameter.pattern else {
                        continue;
                    };
                    let validator_type =
                        super::validators::validator_parameter_type(parameter, parameter_type);
                    self.emit_parameter_validator_call(
                        callee,
                        target,
                        value,
                        &validator_type,
                        &parameter.decorators,
                    )?;
                }
            }
            self.emit_validation_issues_finish();
        }

        let may_throw = self
            .fn_throws
            .get(callee)
            .is_some_and(|throws| !throws.is_empty())
            || self.throwing_closures.contains(callee)
            || self
                .load_var(callee)
                .is_some_and(|(_, ty)| matches!(ty, Type::Throws { .. }));
        let landing = if may_throw {
            self.begin_throwing_call_landing()?
        } else {
            None
        };
        let call = if let Some((fn_val, _, _)) = generic_call {
            self.builder
                .build_call(fn_val, &compiled_args, "generic_call")
                .unwrap()
        } else if let Some(fn_val) = self.fns.get(callee).copied() {
            self.builder
                .build_call(fn_val, &compiled_args, "call")
                .unwrap()
        } else {
            let (slot, ty) = self
                .load_var(callee)
                .ok_or_else(|| format!("error: call to unknown function `{callee}`"))?;
            let ty = match ty {
                Type::Throws { function, .. } => *function,
                other => other,
            };
            let Type::Function {
                ref params,
                ref ret,
            } = ty
            else {
                return Err(format!("error: `{callee}` is not callable"));
            };
            let is_varargs = params.last().is_some_and(|_| {
                self.fn_param_metadata
                    .get(callee)
                    .and_then(|m| m.last())
                    .is_some_and(|p| p.rest)
            });
            if !is_varargs && compiled_args.len() != params.len() {
                return Err(format!(
                    "error: `{callee}` expects {} argument(s), found {}",
                    params.len(),
                    compiled_args.len()
                ));
            }
            let closure = self
                .builder
                .build_load(self.llvm_type(&ty), slot, callee)
                .unwrap()
                .into_struct_value();
            let pointer = self
                .builder
                .build_extract_value(closure, 0, "closure_function")
                .unwrap()
                .into_pointer_value();
            let environment = self
                .builder
                .build_extract_value(closure, 1, "closure_environment")
                .unwrap();
            let mut closure_args = vec![environment.into()];
            closure_args.extend(compiled_args);
            self.builder
                .build_indirect_call(
                    self.llvm_closure_fn_type(params, ret),
                    pointer,
                    &closure_args,
                    "call",
                )
                .unwrap()
        };
        self.end_throwing_call_landing(landing);

        for (argument, value) in borrowed_optional_boxes {
            self.release_borrowed_optional_argument(&argument, value);
        }
        for (value, ty, expression) in async_argument_cleanups {
            self.emit_async_cleanup_source_expression(value, &ty, &expression)?;
        }
        for closure in closure_arg_temporaries {
            self.emit_closure_release(closure)?;
        }

        let result = call
            .try_as_basic_value()
            .basic()
            .unwrap_or_else(|| self.context.i32_type().const_int(0, false).into());
        if is_user_defined && !async_callee {
            let owned_container = self.fn_owned_returns.contains(callee);
            let conditionally_owned_container =
                !owned_container && self.fn_conditionally_owned_returns.contains(callee);
            if let Some(ty) = self
                .expr_type_hint(expr)
                .or_else(|| self.generic_call_return_type(callee, type_args, args))
                .map(|ty| self.resolve_type(&ty))
                .filter(|ty| {
                    self.async_owned_value_supported(ty)
                        || ((owned_container || conditionally_owned_container)
                            && native_collection_handle(transparent_codegen_type(ty)))
                })
            {
                if conditionally_owned_container {
                    let global = self.get_or_declare_return_owned();
                    let bit = self
                        .builder
                        .build_load(
                            self.context.bool_type(),
                            global.as_pointer_value(),
                            "return.owned",
                        )
                        .unwrap()
                        .into_int_value();
                    self.record_conditionally_owned_result(result, ty, bit);
                } else {
                    self.record_owned_native_result(result, ty);
                }
            }
        }
        Ok(result)
    }
}
