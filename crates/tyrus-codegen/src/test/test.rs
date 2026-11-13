use super::*;
use inkwell::{FloatPredicate, IntPredicate};

const NOT_MATCHER: &str = "not";

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_test_fn_call(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        use tyrus_core::test::*;
        match callee {
            DESCRIBE_TARGET => self.emit_test_describe(args).map(Some),
            EXPECT_TARGET => self.emit_test_expect(args).map(Some),
            TEST_TARGET | IT_TARGET => self.emit_test_register(Some(0), args).map(Some),
            BEFORE_ALL_TARGET => self.emit_test_register(Some(1), args).map(Some),
            AFTER_ALL_TARGET => self.emit_test_register(Some(2), args).map(Some),
            BEFORE_EACH_TARGET => self.emit_test_register(Some(3), args).map(Some),
            AFTER_EACH_TARGET => self.emit_test_register(Some(4), args).map(Some),
            _ => Ok(None),
        }
    }

    fn emit_test_describe(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 2 {
            return Err(format!(
                "error: `describe` expects 2 arguments, found {}",
                args.len()
            ));
        }
        self.emit_test_registration(
            "tyrus_test_describe_mode",
            Some(0),
            Some(&args[0]),
            &args[1],
        )
    }

    fn emit_test_expect(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!(
                "error: `expect` expects 1 argument, found {}",
                args.len()
            ));
        }
        self.emit_expr(&args[0])?;
        let assertion = self.llvm_type(&Type::Named(tyrus_core::test::ASSERTION_TYPE.to_string()));
        Ok(assertion.const_zero())
    }

    fn emit_test_register(
        &mut self,
        kind: Option<i32>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let kind = kind.expect("test kinds are always provided");
        let callback = if kind == 0 {
            if args.len() != 2 {
                return Err(format!(
                    "error: `test`/`it` expect 2 arguments, found {}",
                    args.len()
                ));
            }
            &args[1]
        } else {
            if args.len() != 1 {
                return Err(format!(
                    "error: test hooks expect 1 argument, found {}",
                    args.len()
                ));
            }
            &args[0]
        };
        self.emit_test_registration(
            "tyrus_test_register",
            Some(kind),
            (kind == 0).then_some(&args[0]),
            callback,
        )
    }

    fn emit_test_registration(
        &mut self,
        runtime_name: &str,
        kind: Option<i32>,
        name: Option<&Expr>,
        callback: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let name_value = match name {
            Some(name) => self.emit_expr_as(name, &Type::Str)?.into_struct_value(),
            None => self.llvm_type(&Type::Str).into_struct_type().const_zero(),
        };

        let callback_return = match callback.kind() {
            ExprKind::ArrowFn { is_async, .. } if *is_async => Type::Apply {
                name: "Promise".to_string(),
                args: vec![Type::Void],
            },
            _ => Type::Void,
        };
        let callback_type = Type::Function {
            params: Vec::new(),
            ret: Box::new(callback_return.clone()),
        };
        let capture_assertions = kind == Some(0);
        let was_capturing_assertions = self.capture_test_assertions;
        self.capture_test_assertions = capture_assertions;
        let closure = match callback.kind() {
            ExprKind::ArrowFn {
                params,
                ret_type,
                body,
                is_async,
                ..
            } => self.emit_contextual_arrow_fn(
                params,
                ret_type,
                body,
                &[],
                &callback_return,
                *is_async,
                true,
            )?,
            _ => self.emit_expr_as(callback, &callback_type)?,
        };
        self.capture_test_assertions = was_capturing_assertions;
        let _ = self.pending_closure_drop_fn.take();

        let closure_ty = self.llvm_type(&callback_type).into_struct_type();
        let context = self.heap_alloc(closure_ty.into(), "test.context")?;
        self.builder.build_store(context, closure).unwrap();

        let wrapper = self.emit_test_wrapper(closure_ty, &callback_return, capture_assertions)?;
        let drop_fn =
            self.declare_libc_fn("tyrus_drop_promise_closure_context", &[ptr_ty.into()], None);

        let mut params: Vec<BasicTypeEnum<'ctx>> = Vec::new();
        let mut call_args: Vec<BasicMetadataValueEnum<'ctx>> = Vec::new();
        if let Some(kind) = kind {
            params.push(i32_ty.into());
            call_args.push(i32_ty.const_int(kind as u64, false).into());
        }
        params.push(self.string_abi_type());
        params.push(ptr_ty.into());
        params.push(ptr_ty.into());
        params.push(ptr_ty.into());
        call_args.push(name_value.into());
        call_args.push(wrapper.as_global_value().as_pointer_value().into());
        call_args.push(context.into());
        call_args.push(drop_fn.as_global_value().as_pointer_value().into());

        let function = self.declare_libc_fn(runtime_name, &params, None);
        self.builder
            .build_call(function, &call_args, "test.registration")
            .map_err(|error| error.to_string())?;
        Ok(self.context.i8_type().const_zero().into())
    }

    fn emit_test_wrapper(
        &mut self,
        closure_ty: StructType<'ctx>,
        callback_return: &Type,
        capture_assertions: bool,
    ) -> Result<FunctionValue<'ctx>, String> {
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_ty = self.context.i32_type();
        let name = format!("__tyrus_test_wrapper_{}", self.arrow_fn_count);
        self.arrow_fn_count += 1;
        let wrapper_ty = i32_ty.fn_type(&[ptr_ty.into(), ptr_ty.into()], false);
        let wrapper = self.module.add_function(&name, wrapper_ty, None);
        let saved_block = self.builder.get_insert_block();

        let entry = self
            .context
            .append_basic_block(wrapper, "test.wrapper.entry");
        self.builder.position_at_end(entry);
        let result_ptr = wrapper.get_nth_param(0).unwrap().into_pointer_value();
        let context_ptr = wrapper.get_nth_param(1).unwrap().into_pointer_value();
        let context_value = self
            .builder
            .build_load(closure_ty, context_ptr, "test.wrapper.closure")
            .map_err(|error| error.to_string())?;

        let ctx_ty = self.catch_ctx_type();
        let err_slot = self
            .builder
            .build_alloca(self.string_type(), "test.wrapper.err")
            .unwrap();
        self.builder
            .build_store(err_slot, self.string_type().const_zero())
            .unwrap();
        let ctx_slot = self
            .builder
            .build_alloca(ctx_ty, "test.wrapper.ctx")
            .unwrap();
        ctx_slot
            .as_instruction()
            .unwrap()
            .set_alignment(16)
            .unwrap();
        let owned_slot = self
            .builder
            .build_alloca(self.context.bool_type(), "test.wrapper.owned")
            .unwrap();
        self.builder
            .build_store(owned_slot, self.context.bool_type().const_zero())
            .unwrap();

        let catch_top_global = self.get_or_declare_catch_top();
        let catch_top_ptr = catch_top_global.as_pointer_value();

        let prev = self
            .builder
            .build_load(ptr_ty, catch_top_ptr, "test.wrapper.prev")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(
                self.builder
                    .build_struct_gep(ctx_ty, ctx_slot, 2, "test.wrapper.ctx.prev")
                    .map_err(|error| error.to_string())?,
                prev,
            )
            .unwrap();
        self.builder
            .build_store(
                self.builder
                    .build_struct_gep(ctx_ty, ctx_slot, 1, "test.wrapper.ctx.err_ptr")
                    .map_err(|error| error.to_string())?,
                err_slot,
            )
            .unwrap();
        self.builder
            .build_store(
                self.builder
                    .build_struct_gep(ctx_ty, ctx_slot, 3, "test.wrapper.ctx.owned_ptr")
                    .map_err(|error| error.to_string())?,
                owned_slot,
            )
            .unwrap();
        self.builder.build_store(catch_top_ptr, ctx_slot).unwrap();

        let jbuf = self
            .builder
            .build_struct_gep(ctx_ty, ctx_slot, 0, "test.wrapper.jbuf")
            .map_err(|error| error.to_string())?;
        let setjmp = self.declare_setjmp();
        let setjmp_ret = self
            .builder
            .build_call(setjmp, &self.setjmp_arguments(jbuf), "test.wrapper.setjmp")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "internal: setjmp returned no value".to_string())?
            .into_int_value();
        let caught = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                setjmp_ret,
                i32_ty.const_zero(),
                "test.wrapper.caught",
            )
            .map_err(|error| error.to_string())?;

        let body_bb = self
            .context
            .append_basic_block(wrapper, "test.wrapper.body");
        let catch_bb = self
            .context
            .append_basic_block(wrapper, "test.wrapper.catch");
        self.builder
            .build_conditional_branch(caught, catch_bb, body_bb)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(body_bb);
        if capture_assertions {
            let begin = self.declare_libc_fn("tyrus_test_begin_assertions", &[], None);
            self.builder
                .build_call(begin, &[], "test.wrapper.begin_assertions")
                .map_err(|error| error.to_string())?;
        }
        let context_value = context_value.into_struct_value();
        let function_ptr = self
            .builder
            .build_extract_value(context_value, 0, "test.wrapper.fn")
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let environment = self
            .builder
            .build_extract_value(context_value, 1, "test.wrapper.env")
            .map_err(|error| error.to_string())?;
        let call = self
            .builder
            .build_indirect_call(
                self.llvm_closure_fn_type(&[], callback_return),
                function_ptr,
                &[environment.into()],
                "test.wrapper.call",
            )
            .map_err(|error| error.to_string())?;
        if matches!(callback_return, Type::Apply { name, .. } if name == "Promise") {
            let promise = call
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| "internal: async test lost its Promise".to_string())?;
            let promise_field = self
                .builder
                .build_struct_gep(
                    self.result_slot_type(),
                    result_ptr,
                    2,
                    "test.wrapper.promise",
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_store(promise_field, promise)
                .map_err(|error| error.to_string())?;
        }
        self.builder
            .build_store(catch_top_ptr, prev)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_return(Some(&i32_ty.const_zero()))
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(catch_bb);
        self.builder
            .build_store(catch_top_ptr, prev)
            .map_err(|error| error.to_string())?;
        let status_field = self
            .builder
            .build_struct_gep(
                self.result_slot_type(),
                result_ptr,
                0,
                "test.wrapper.status",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(status_field, i32_ty.const_int(1, false))
            .map_err(|error| error.to_string())?;
        let i64_ty = self.context.i64_type();
        let tag = self
            .builder
            .build_load(
                i64_ty,
                self.builder
                    .build_struct_gep(ctx_ty, ctx_slot, 4, "test.wrapper.ctx.tag")
                    .map_err(|error| error.to_string())?,
                "test.wrapper.tag",
            )
            .map_err(|error| error.to_string())?
            .into_int_value();
        let cell = self
            .builder
            .build_load(
                ptr_ty,
                self.builder
                    .build_struct_gep(ctx_ty, ctx_slot, 5, "test.wrapper.ctx.cell")
                    .map_err(|error| error.to_string())?,
                "test.wrapper.cell",
            )
            .map_err(|error| error.to_string())?
            .into_pointer_value();
        let is_string = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                tag,
                i64_ty.const_int(self.thrown_type_tag(&Type::Str), false),
                "test.wrapper.threw_string",
            )
            .map_err(|error| error.to_string())?;
        let take_bb = self
            .context
            .append_basic_block(wrapper, "test.wrapper.take");
        let taken_bb = self
            .context
            .append_basic_block(wrapper, "test.wrapper.taken");
        self.builder
            .build_conditional_branch(is_string, take_bb, taken_bb)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(take_bb);
        let thrown = self
            .builder
            .build_load(self.string_type(), cell, "test.wrapper.thrown")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(err_slot, thrown)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_unconditional_branch(taken_bb)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(taken_bb);
        let free = self.declare_libc_fn("free", &[ptr_ty.into()], None);
        self.builder
            .build_call(free, &[cell.into()], "")
            .map_err(|error| error.to_string())?;
        let message = self
            .builder
            .build_load(self.string_type(), err_slot, "test.wrapper.message")
            .map_err(|error| error.to_string())?;
        let message_field = self
            .builder
            .build_struct_gep(
                self.result_slot_type(),
                result_ptr,
                1,
                "test.wrapper.message",
            )
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(message_field, message)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_return(Some(&i32_ty.const_int(1, false)))
            .map_err(|error| error.to_string())?;

        if let Some(block) = saved_block {
            self.builder.position_at_end(block);
        }
        Ok(wrapper)
    }

    fn result_slot_type(&self) -> StructType<'ctx> {
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        self.context.struct_type(
            &[
                self.context.i32_type().into(),
                self.string_type().into(),
                ptr_ty.into(),
            ],
            false,
        )
    }

    pub(super) fn emit_test_method_call(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        use tyrus_core::test::*;

        if let ExprKind::Ident(target) = object.kind()
            && let Some(modifier) = test_modifier(target, method)
        {
            return self.emit_test_modifier(target, modifier, args).map(Some);
        }

        let mut matchers: Vec<&str> = vec![method];
        let mut receiver = object;
        loop {
            match receiver.kind() {
                ExprKind::MethodCall {
                    object: inner_object,
                    method: inner_method,
                    ..
                }
                | ExprKind::FieldAccess {
                    object: inner_object,
                    field: inner_method,
                } if is_chain_modifier(inner_method) => {
                    matchers.push(inner_method.as_str());
                    receiver = inner_object;
                }
                _ => break,
            }
        }
        let ExprKind::FnCall {
            callee,
            args: call_args,
            ..
        } = receiver.kind()
        else {
            return Ok(None);
        };
        if callee != EXPECT_TARGET {
            return Ok(None);
        }
        let actual = match call_args.first() {
            Some(actual) if call_args.len() == 1 => actual,
            _ => return Ok(None),
        };

        let matcher_name = matchers[0];
        let negative = matchers
            .iter()
            .skip(1)
            .filter(|modifier| **modifier == NOT_MATCHER)
            .count()
            % 2
            == 1;
        let rejects = matchers
            .iter()
            .skip(1)
            .any(|modifier| *modifier == REJECTS_MODIFIER);
        let resolves = matchers
            .iter()
            .skip(1)
            .any(|modifier| *modifier == RESOLVES_MODIFIER);

        if matcher_name == NOT_MATCHER {
            self.emit_expr(actual)?;
            return Ok(Some(self.context.i8_type().const_zero().into()));
        }

        if rejects {
            return self
                .emit_test_promise_rejects(actual, args.first(), negative)
                .map(Some);
        }

        if matcher_name == "toThrow" {
            return self
                .emit_test_to_throw(actual, args.first(), negative)
                .map(Some);
        }

        let (actual_value, actual_ty) = if resolves {
            self.emit_test_promise_value(actual)?
        } else {
            let value = self.emit_expr(actual)?;
            let ty = self
                .expr_type_hint(actual)
                .map_or(Type::Unknown, |ty| self.resolve_type(&ty));
            (value, ty)
        };

        let shows_actual = self.bind_test_operand(ACTUAL_TEMP, actual_value, &actual_ty)?;
        let expected_ctx = expected_operand_type(matcher_name, &actual_ty);
        let expected_arg;
        let has_expected = expected_ctx.is_some();
        let actual_is_text = matches!(resolve_scalar_kind(&actual_ty), ScalarKind::Str);
        let expected_is_text = expected_ctx
            .as_ref()
            .is_some_and(|ty| matches!(resolve_scalar_kind(ty), ScalarKind::Str));
        let args: &[Expr] = match (expected_ctx, args.first()) {
            (Some(ctx), Some(expected)) => {
                let value = self.emit_expr_as(expected, &ctx)?;
                self.bind_test_operand(EXPECTED_TEMP, value, &ctx)?;
                expected_arg = [Expr::synthetic(ExprKind::Ident(EXPECTED_TEMP.to_string()))];
                &expected_arg
            }
            _ => args,
        };

        let passed = match matcher_name {
            "toBe" | "toEqual" | "toStrictEqual" => {
                self.emit_test_equal(actual_value, args, &actual_ty, matcher_name)?
            }
            "toBeTruthy" => self.emit_test_truthy(actual_value, &actual_ty, true, matcher_name)?,
            "toBeFalsy" => self.emit_test_truthy(actual_value, &actual_ty, false, matcher_name)?,
            "toBeNull" => self.emit_test_nullness(actual_value, &actual_ty, false, matcher_name)?,
            "toBeDefined" => {
                self.emit_test_nullness(actual_value, &actual_ty, true, matcher_name)?
            }
            "toBeUndefined" => {
                self.emit_test_nullness(actual_value, &actual_ty, false, matcher_name)?
            }
            "toBeGreaterThan"
            | "toBeGreaterThanOrEqual"
            | "toBeLessThan"
            | "toBeLessThanOrEqual" => {
                self.emit_test_ordering(actual_value, args, &actual_ty, matcher_name)?
            }
            "toBeCloseTo" => {
                self.emit_test_close_to(actual_value, args, &actual_ty, matcher_name)?
            }
            "toHaveLength" => {
                self.emit_test_has_length(actual_value, args, &actual_ty, matcher_name)?
            }
            "toContain" => self.emit_test_contains(actual_value, args, &actual_ty, matcher_name)?,
            "toMatch" => self.emit_test_matches(actual_value, args, &actual_ty, matcher_name)?,
            _ => return Ok(None),
        };

        let passed = if negative {
            self.builder
                .build_not(passed, "test.matcher.negate")
                .map_err(|error| error.to_string())?
        } else {
            passed
        };

        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| "internal: matcher outside a function".to_string())?;
        let fail = self
            .context
            .append_basic_block(function, "test.matcher.fail");
        let cont = self
            .context
            .append_basic_block(function, "test.matcher.cont");
        self.builder
            .build_conditional_branch(passed, cont, fail)
            .map_err(|error| error.to_string())?;
        self.builder.position_at_end(fail);
        let structural = matches!(matcher_name, "toBe" | "toEqual" | "toStrictEqual")
            .then(|| match transparent_codegen_type(&actual_ty) {
                Type::Array(_) => Some("arrays"),
                Type::Named(_) | Type::Object(_) => Some("objects"),
                _ => None,
            })
            .flatten();
        let message = if let Some(kind) = structural {
            let negation = if negative { "not " } else { "" };
            Expr::synthetic(ExprKind::StrLit(format!(
                "expected the {kind} {negation}to be equal"
            )))
        } else {
            let subject = match transparent_codegen_type(&actual_ty) {
                Type::Array(_) => "the array",
                Type::Named(_) | Type::Object(_) => "the object",
                _ => "value",
            };
            assertion_message(
                matcher_name,
                negative,
                shows_actual.then_some(actual_is_text),
                has_expected.then_some(expected_is_text),
                subject,
            )
        };
        self.emit_throw(&message, function)?;
        self.builder.position_at_end(cont);
        Ok(Some(self.context.i8_type().const_zero().into()))
    }

    fn emit_test_modifier(
        &mut self,
        target: &str,
        modifier: tyrus_core::test::TestModifier,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        use tyrus_core::test::{DESCRIBE_TARGET, TestModifier};
        if modifier == TestModifier::Each {
            return self.emit_test_each(args);
        }
        if modifier == TestModifier::Todo {
            let name = args
                .first()
                .ok_or_else(|| "error: `test.todo` expects 1 argument, found 0".to_string())?;
            let name_value = self.emit_expr_as(name, &Type::Str)?.into_struct_value();
            let todo = self.declare_libc_fn("tyrus_test_todo", &[self.string_abi_type()], None);
            self.builder
                .build_call(todo, &[name_value.into()], "test.todo")
                .map_err(|error| error.to_string())?;
            return Ok(self.context.i8_type().const_zero().into());
        }
        if args.len() != 2 {
            return Err(format!(
                "error: test modifiers expect 2 arguments, found {}",
                args.len()
            ));
        }
        let skip = modifier == TestModifier::Skip;
        let (runtime_name, kind) = if target == DESCRIBE_TARGET {
            ("tyrus_test_describe_mode", if skip { 1 } else { 2 })
        } else {
            ("tyrus_test_register", if skip { 5 } else { 6 })
        };
        self.emit_test_registration(runtime_name, Some(kind), Some(&args[0]), &args[1])
    }

    fn emit_test_each(&mut self, args: &[Expr]) -> Result<BasicValueEnum<'ctx>, String> {
        use tyrus_core::ast::{Param, ParamPattern, Stmt, StmtKind};
        if args.len() != 3 {
            return Err(format!(
                "error: `test.each` expects 3 arguments, found {}",
                args.len()
            ));
        }
        let ExprKind::ArrayLit(cases) = args[1].kind() else {
            return Err(
                "error: the cases of `test.each` must be an array literal, so one test can be registered per case"
                    .to_string(),
            );
        };
        let element = self
            .expr_type_hint(&args[1])
            .map(|ty| self.resolve_type(&ty))
            .and_then(|ty| match transparent_codegen_type(&ty) {
                Type::Array(element) => Some(element.as_ref().clone()),
                _ => None,
            });

        for (index, case) in cases.iter().enumerate() {
            let name = Expr::synthetic(ExprKind::TemplateLit(vec![
                TemplatePart::Expr(args[0].clone(), None),
                TemplatePart::Text(format!(" [{index}]")),
            ]));
            let body = match args[2].kind() {
                ExprKind::Ident(_) => vec![Stmt::synthetic(StmtKind::Expr(Expr::synthetic(
                    ExprKind::CallValue {
                        callee: Box::new(args[2].clone()),
                        type_args: Vec::new(),
                        args: vec![case.clone()],
                    },
                )))],
                ExprKind::ArrowFn { params, body, .. } => {
                    let [parameter] = params.as_slice() else {
                        return Err(format!(
                            "error: the callback of `test.each` takes exactly one case, but it declares {}",
                            params.len()
                        ));
                    };
                    let ParamPattern::Name(name) = &parameter.pattern else {
                        return Err(
                            "error: the callback of `test.each` must name its case parameter"
                                .to_string(),
                        );
                    };
                    let declared = match (&parameter.ty, &element) {
                        (Type::Infer, Some(element)) => element.clone(),
                        (declared, _) => declared.clone(),
                    };
                    let mut statements = vec![Stmt::synthetic(StmtKind::VarDecl {
                        mutable: false,
                        name: name.clone(),
                        ty: declared,
                        value: Some(case.clone()),
                        onfail: None,
                    })];
                    statements.extend(body.iter().cloned());
                    statements
                }
                _ => {
                    return Err(
                        "error: the callback of `test.each` must be written inline or named"
                            .to_string(),
                    );
                }
            };
            let case_test = Expr::synthetic(ExprKind::ArrowFn {
                is_async: false,
                type_params: Vec::new(),
                params: Vec::<Param>::new(),
                ret_type: Type::Void,
                throws: Vec::new(),
                body,
            });
            self.emit_test_registration("tyrus_test_register", Some(0), Some(&name), &case_test)?;
        }
        Ok(self.context.i8_type().const_zero().into())
    }

    fn emit_test_promise_value(
        &mut self,
        actual: &Expr,
    ) -> Result<(BasicValueEnum<'ctx>, Type), String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| "internal: `.resolves` outside a function".to_string())?;
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let promise_type = self
            .expr_type_hint(actual)
            .map(|ty| self.resolve_type(&ty))
            .ok_or_else(|| {
                "error: `.resolves` expects a promise, such as the result of an `async` function"
                    .to_string()
            })?;
        let payload = crate::core::promise_payload_type(transparent_codegen_type(&promise_type))
            .cloned()
            .ok_or_else(|| {
                format!("error: `.resolves` expects a promise, found `{promise_type}`")
            })?;
        let payload = self.resolve_type(&payload);
        if matches!(payload, Type::Void) {
            return Err(
                "error: `.resolves` needs a promise that resolves to a value, and this one resolves to `void`"
                    .to_string(),
            );
        }
        let promise = self.emit_expr(actual)?;
        let BasicValueEnum::PointerValue(promise) = promise else {
            return Err(
                "error: `.resolves` expects a promise, such as the result of an `async` function"
                    .to_string(),
            );
        };
        let settle = self.declare_libc_fn(
            "tyrus_test_settle",
            &[ptr_ty.into()],
            Some(self.context.i32_type().into()),
        );
        let state = self
            .builder
            .build_call(settle, &[promise.into()], "test.resolves.state")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "internal: settling a promise returned no value".to_string())?
            .into_int_value();
        let fulfilled = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                state,
                self.context.i32_type().const_int(1, false),
                "test.resolves.fulfilled",
            )
            .map_err(|error| error.to_string())?;

        let borrowed = self.declare_libc_fn(
            "tyrus_test_rejection_message",
            &[ptr_ty.into()],
            Some(self.string_abi_type()),
        );
        let borrowed = self
            .builder
            .build_call(borrowed, &[promise.into()], "test.resolves.borrowed")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "internal: the rejection message is missing".to_string())?;
        self.bind_test_operand(REJECTION_TEMP, borrowed, &Type::Str)?;
        let copied = self.emit_expr_as(
            &Expr::synthetic(ExprKind::TemplateLit(vec![TemplatePart::Expr(
                Expr::synthetic(ExprKind::Ident(REJECTION_TEMP.to_string())),
                None,
            )])),
            &Type::Str,
        )?;
        self.bind_owned_test_string(REJECTION_TEMP, copied)?;

        let promise_free = self.declare_libc_fn("tyrus_promise_free", &[ptr_ty.into()], None);
        let take = self
            .context
            .append_basic_block(function, "test.resolves.take");
        let failed = self
            .context
            .append_basic_block(function, "test.resolves.failed");
        self.builder
            .build_conditional_branch(fulfilled, take, failed)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(failed);
        self.builder
            .build_call(promise_free, &[promise.into()], "")
            .map_err(|error| error.to_string())?;
        self.emit_throw(
            &Expr::synthetic(ExprKind::TemplateLit(vec![
                TemplatePart::Text(
                    "expected the promise to resolve, but it rejected with \"".to_string(),
                ),
                TemplatePart::Expr(
                    Expr::synthetic(ExprKind::Ident(REJECTION_TEMP.to_string())),
                    None,
                ),
                TemplatePart::Text("\"".to_string()),
            ])),
            function,
        )?;

        self.builder.position_at_end(take);
        let take_value = self.declare_libc_fn(
            "tyrus_promise_take_value",
            &[ptr_ty.into()],
            Some(ptr_ty.into()),
        );
        let boxed = self
            .builder
            .build_call(take_value, &[promise.into()], "test.resolves.boxed")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "internal: the resolved value is missing".to_string())?
            .into_pointer_value();
        let value = self
            .builder
            .build_load(self.llvm_type(&payload), boxed, "test.resolves.value")
            .map_err(|error| error.to_string())?;
        let free = self.declare_libc_fn("free", &[ptr_ty.into()], None);
        self.builder
            .build_call(free, &[boxed.into()], "")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_call(promise_free, &[promise.into()], "")
            .map_err(|error| error.to_string())?;
        if !payload.is_copy() {
            self.record_owned_native_result(value, payload.clone());
        }
        Ok((value, payload))
    }

    fn emit_test_promise_rejects(
        &mut self,
        actual: &Expr,
        expected: Option<&Expr>,
        negative: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| "internal: `.rejects` outside a function".to_string())?;
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let promise = self.emit_expr(actual)?;
        let BasicValueEnum::PointerValue(promise) = promise else {
            return Err(
                "error: `.rejects` expects a promise, such as the result of an `async` function"
                    .to_string(),
            );
        };
        let settle = self.declare_libc_fn(
            "tyrus_test_settle",
            &[ptr_ty.into()],
            Some(self.context.i32_type().into()),
        );
        let state = self
            .builder
            .build_call(settle, &[promise.into()], "test.rejects.state")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "internal: settling a promise returned no value".to_string())?
            .into_int_value();
        let rejected = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                state,
                self.context.i32_type().const_int(2, false),
                "test.rejects.rejected",
            )
            .map_err(|error| error.to_string())?;

        let borrowed = self.declare_libc_fn(
            "tyrus_test_rejection_message",
            &[ptr_ty.into()],
            Some(self.string_abi_type()),
        );
        let borrowed = self
            .builder
            .build_call(borrowed, &[promise.into()], "test.rejects.borrowed")
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "internal: the rejection message is missing".to_string())?;
        self.bind_test_operand(REJECTION_TEMP, borrowed, &Type::Str)?;
        let copied = self.emit_expr_as(
            &Expr::synthetic(ExprKind::TemplateLit(vec![TemplatePart::Expr(
                Expr::synthetic(ExprKind::Ident(REJECTION_TEMP.to_string())),
                None,
            )])),
            &Type::Str,
        )?;
        self.bind_owned_test_string(REJECTION_TEMP, copied)?;
        let free = self.declare_libc_fn("tyrus_promise_free", &[ptr_ty.into()], None);
        self.builder
            .build_call(free, &[promise.into()], "")
            .map_err(|error| error.to_string())?;

        let expected_matches = match expected {
            Some(expected) => {
                let value = self.emit_expr_as(expected, &Type::Str)?;
                self.bind_test_operand(EXPECTED_TEMP, value, &Type::Str)?;
                Some(self.emit_string_equals(
                    copied.into_struct_value(),
                    value.into_struct_value(),
                    "test.rejects.same_message",
                )?)
            }
            None => None,
        };

        let text = |value: &str| TemplatePart::Text(value.to_string());
        let part = |binding: &str| {
            TemplatePart::Expr(Expr::synthetic(ExprKind::Ident(binding.to_string())), None)
        };
        let cont = self
            .context
            .append_basic_block(function, "test.rejects.cont");

        if negative {
            let failed = match expected_matches {
                Some(same) => self
                    .builder
                    .build_and(rejected, same, "test.rejects.failed")
                    .map_err(|error| error.to_string())?,
                None => rejected,
            };
            let fail = self
                .context
                .append_basic_block(function, "test.rejects.fail");
            self.builder
                .build_conditional_branch(failed, fail, cont)
                .map_err(|error| error.to_string())?;
            self.builder.position_at_end(fail);
            let message = match expected {
                Some(_) => Expr::synthetic(ExprKind::TemplateLit(vec![
                    text("expected the promise not to reject with \""),
                    part(EXPECTED_TEMP),
                    text("\""),
                ])),
                None => Expr::synthetic(ExprKind::TemplateLit(vec![
                    text("expected the promise not to reject, but it rejected with \""),
                    part(REJECTION_TEMP),
                    text("\""),
                ])),
            };
            self.emit_throw(&message, function)?;
        } else {
            let resolved_fail = self
                .context
                .append_basic_block(function, "test.rejects.resolved");
            let checked = self
                .context
                .append_basic_block(function, "test.rejects.checked");
            self.builder
                .build_conditional_branch(rejected, checked, resolved_fail)
                .map_err(|error| error.to_string())?;
            self.builder.position_at_end(resolved_fail);
            self.emit_throw(
                &Expr::synthetic(ExprKind::StrLit(
                    "expected the promise to reject, but it resolved".to_string(),
                )),
                function,
            )?;
            self.builder.position_at_end(checked);
            match expected_matches {
                Some(same) => {
                    let mismatch = self
                        .context
                        .append_basic_block(function, "test.rejects.mismatch");
                    self.builder
                        .build_conditional_branch(same, cont, mismatch)
                        .map_err(|error| error.to_string())?;
                    self.builder.position_at_end(mismatch);
                    let message = Expr::synthetic(ExprKind::TemplateLit(vec![
                        text("expected the promise to reject with \""),
                        part(EXPECTED_TEMP),
                        text("\", but it rejected with \""),
                        part(REJECTION_TEMP),
                        text("\""),
                    ]));
                    self.emit_throw(&message, function)?;
                }
                None => {
                    self.builder
                        .build_unconditional_branch(cont)
                        .map_err(|error| error.to_string())?;
                }
            }
        }

        self.builder.position_at_end(cont);
        Ok(self.context.i8_type().const_zero().into())
    }

    fn emit_test_to_throw(
        &mut self,
        actual: &Expr,
        expected: Option<&Expr>,
        negative: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        use tyrus_core::ast::{AssignOp, BinOp, Stmt, StmtKind};
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| "internal: toThrow outside a function".to_string())?;
        let ExprKind::Ident(name) = actual.kind() else {
            return Err("internal: `toThrow` expects a function name".to_string());
        };
        let thrown = self
            .fn_throws
            .get(name)
            .and_then(|types| types.first())
            .cloned()
            .ok_or_else(|| format!("internal: `{name}` declares no thrown type"))?;
        let display = tyrus_core::module_loader::original_global_name(name).to_string();
        let thrown_is_text = matches!(thrown, Type::Str);

        let id = self.next_assert_throws_id;
        self.next_assert_throws_id += 1;
        let flag = format!("__tyrus_threw_{id}");
        let caught = format!("__tyrus_caught_{id}");
        let wanted = format!("__tyrus_wanted_{id}");

        let ident = |binding: &str| Expr::synthetic(ExprKind::Ident(binding.to_string()));
        let text = |value: String| TemplatePart::Text(value);
        let part = |binding: &str| TemplatePart::Expr(ident(binding), None);
        let template = |parts: Vec<TemplatePart>| Expr::synthetic(ExprKind::TemplateLit(parts));
        let assert = |condition: Expr, message: Expr| {
            Stmt::synthetic(StmtKind::Expr(Expr::synthetic(ExprKind::FnCall {
                callee: "assert".to_string(),
                type_args: Vec::new(),
                args: vec![condition, message],
            })))
        };
        let equals = |left: Expr, right: Expr| {
            Expr::synthetic(ExprKind::BinOp {
                op: BinOp::Eq,
                left: Box::new(left),
                right: Box::new(right),
            })
        };

        let mut statements = vec![Stmt::synthetic(StmtKind::VarDecl {
            mutable: true,
            name: flag.clone(),
            ty: Type::Bool,
            value: Some(Expr::synthetic(ExprKind::BoolLit(false))),
            onfail: None,
        })];
        if let Some(expected) = expected {
            statements.push(Stmt::synthetic(StmtKind::VarDecl {
                mutable: false,
                name: wanted.clone(),
                ty: Type::Str,
                value: Some(expected.clone()),
                onfail: None,
            }));
        }

        let mut catch_body = vec![Stmt::synthetic(StmtKind::Assign {
            name: flag.clone(),
            op: AssignOp::Assign,
            value: Expr::synthetic(ExprKind::BoolLit(true)),
        })];
        match (negative, expected.is_some()) {
            (false, true) => catch_body.push(assert(
                equals(ident(&caught), ident(&wanted)),
                template(vec![
                    text(format!("expected {display} to throw \"")),
                    part(&wanted),
                    text("\", but it threw \"".to_string()),
                    part(&caught),
                    text("\"".to_string()),
                ]),
            )),
            (true, true) => catch_body.push(assert(
                Expr::synthetic(ExprKind::Not(Box::new(equals(
                    ident(&caught),
                    ident(&wanted),
                )))),
                template(vec![
                    text(format!("expected {display} not to throw \"")),
                    part(&wanted),
                    text("\"".to_string()),
                ]),
            )),
            (true, false) => {
                let message = if thrown_is_text {
                    template(vec![
                        text(format!("expected {display} not to throw, but it threw \"")),
                        part(&caught),
                        text("\"".to_string()),
                    ])
                } else {
                    Expr::synthetic(ExprKind::StrLit(format!("expected {display} not to throw")))
                };
                catch_body.push(assert(Expr::synthetic(ExprKind::BoolLit(false)), message));
            }
            (false, false) => {}
        }

        statements.push(Stmt::synthetic(StmtKind::TryCatch {
            try_body: vec![Stmt::synthetic(StmtKind::Expr(Expr::synthetic(
                ExprKind::FnCall {
                    callee: name.clone(),
                    type_args: Vec::new(),
                    args: Vec::new(),
                },
            )))],
            catch_binding: caught.clone(),
            catch_type: thrown,
            catch_type_span: tyrus_core::span::Span::DUMMY,
            catch_body,
            finally_body: None,
        }));
        if !negative {
            statements.push(assert(
                ident(&flag),
                Expr::synthetic(ExprKind::StrLit(format!("expected {display} to throw"))),
            ));
        }

        self.push_scope();
        for statement in &statements {
            self.emit_stmt(statement, function)?;
        }
        self.pop_scope();
        Ok(self.context.i8_type().const_zero().into())
    }

    fn bind_test_operand(
        &mut self,
        name: &str,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<bool, String> {
        let slot = self.entry_alloca(value.get_type(), name)?;
        self.builder
            .build_store(slot, value)
            .map_err(|error| error.to_string())?;
        self.store_var(name, slot, ty.clone());
        Ok(is_renderable(ty))
    }

    fn bind_owned_test_string(
        &mut self,
        name: &str,
        value: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        self.bind_test_operand(name, value, &Type::Str)?;
        self.forget_owned_native_result(value);
        self.mark_owned_synchronous_value_with(name, ReleasePlan::Deep);
        Ok(())
    }

    fn emit_test_equal(
        &mut self,
        actual: BasicValueEnum<'ctx>,
        args: &[Expr],
        actual_ty: &Type,
        matcher_name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let expected = &args[0];
        match resolve_scalar_kind(actual_ty) {
            ScalarKind::Str => {
                let expected = self.emit_expr_as(expected, &Type::Str)?.into_struct_value();
                self.emit_string_equals(actual.into_struct_value(), expected, "test.equal")
            }
            ScalarKind::Int => {
                let expected = self.emit_expr(expected)?.into_int_value();
                self.coerce_int_eq(actual.into_int_value(), expected, "test.equal")
            }
            ScalarKind::Float => {
                let expected = self.emit_expr(expected)?.into_float_value();
                self.builder
                    .build_float_compare(
                        FloatPredicate::OEQ,
                        actual.into_float_value(),
                        expected,
                        "test.equal",
                    )
                    .map_err(|error| error.to_string())
            }
            ScalarKind::Bool => {
                let expected = self.emit_expr(expected)?.into_int_value();
                self.builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        actual.into_int_value(),
                        expected,
                        "test.equal",
                    )
                    .map_err(|error| error.to_string())
            }
            ScalarKind::Char => {
                let expected = self.emit_expr(expected)?.into_int_value();
                self.builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        actual.into_int_value(),
                        expected,
                        "test.equal",
                    )
                    .map_err(|error| error.to_string())
            }
            ScalarKind::Other => {
                if let Type::Array(element) = transparent_codegen_type(actual_ty) {
                    return self.emit_test_array_equals(
                        actual,
                        expected,
                        actual_ty,
                        element,
                        matcher_name,
                    );
                }
                if matches!(
                    transparent_codegen_type(actual_ty),
                    Type::Named(_) | Type::Object(_)
                ) {
                    let expected_value = self.emit_expr_as(expected, actual_ty)?;
                    return self
                        .emit_structural_equality(actual, expected_value, actual_ty)
                        .map_err(|_| {
                            format!(
                                "error: `{matcher_name}` on a value of type `{actual_ty}` is not supported yet"
                            )
                        });
                }
                Err(format!(
                    "error: `{matcher_name}` on a value of type `{actual_ty}` is not supported yet"
                ))
            }
        }
    }

    fn emit_test_array_equals(
        &mut self,
        actual: BasicValueEnum<'ctx>,
        expected: &Expr,
        actual_ty: &Type,
        element: &Type,
        matcher_name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        if matches!(resolve_scalar_kind(element), ScalarKind::Other) {
            let expected_value = self.emit_expr_as(expected, actual_ty)?;
            return self
                .emit_structural_equality(actual, expected_value, actual_ty)
                .map_err(|_| {
                    format!(
                        "error: `{matcher_name}` on a value of type `{actual_ty}` is not supported yet"
                    )
                });
        }
        let expected_value = self.emit_expr_as(expected, actual_ty)?;
        let array_ty = self.llvm_type(actual_ty);
        let left = self.entry_alloca(array_ty, "test.equal.actual")?;
        self.builder
            .build_store(left, actual)
            .map_err(|error| error.to_string())?;
        let right = self.entry_alloca(array_ty, "test.equal.expected")?;
        self.builder
            .build_store(right, expected_value)
            .map_err(|error| error.to_string())?;
        let element_size = self
            .llvm_type(element)
            .size_of()
            .ok_or_else(|| format!("error: array element type `{element}` has no native size"))?;
        let kind = self
            .context
            .i32_type()
            .const_int(self.array_element_kind(element), false);
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let equals = self.declare_libc_fn(
            "tyrus_array_equals",
            &[
                ptr_ty.into(),
                ptr_ty.into(),
                self.context.i64_type().into(),
                self.context.i32_type().into(),
            ],
            Some(self.context.bool_type().into()),
        );
        self.builder
            .build_call(
                equals,
                &[left.into(), right.into(), element_size.into(), kind.into()],
                "test.array.equals",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "internal: array equality returned no value".to_string())
            .map(|value| value.into_int_value())
    }

    fn emit_test_truthy(
        &mut self,
        actual: BasicValueEnum<'ctx>,
        actual_ty: &Type,
        truthy: bool,
        matcher_name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let passed = match resolve_scalar_kind(actual_ty) {
            ScalarKind::Str => {
                let len = self.string_byte_len(actual.into_struct_value())?;
                self.builder
                    .build_int_compare(
                        IntPredicate::UGT,
                        len,
                        self.usize_type().const_zero(),
                        "test.truthy",
                    )
                    .map_err(|error| error.to_string())?
            }
            ScalarKind::Int => {
                let value = actual.into_int_value();
                self.builder
                    .build_int_compare(
                        IntPredicate::NE,
                        value,
                        value.get_type().const_zero(),
                        "test.truthy",
                    )
                    .map_err(|error| error.to_string())?
            }
            ScalarKind::Float => {
                let value = actual.into_float_value();
                self.builder
                    .build_float_compare(
                        FloatPredicate::ONE,
                        value,
                        value.get_type().const_zero(),
                        "test.truthy",
                    )
                    .map_err(|error| error.to_string())?
            }
            ScalarKind::Bool | ScalarKind::Char => {
                let value = actual.into_int_value();
                self.builder
                    .build_int_compare(
                        IntPredicate::NE,
                        value,
                        value.get_type().const_zero(),
                        "test.truthy",
                    )
                    .map_err(|error| error.to_string())?
            }
            ScalarKind::Other => {
                if matches!(actual_ty, Type::Optional(_)) {
                    let is_null = self
                        .builder
                        .build_is_null(actual.into_pointer_value(), "test.truthy.null")
                        .map_err(|error| error.to_string())?;
                    self.builder
                        .build_not(is_null, "test.truthy")
                        .map_err(|error| error.to_string())?
                } else {
                    return Err(format!(
                        "error: `{matcher_name}` on a value of type `{actual_ty}` is not supported yet"
                    ));
                }
            }
        };
        if truthy {
            Ok(passed)
        } else {
            self.builder
                .build_not(passed, "test.falsy")
                .map_err(|error| error.to_string())
        }
    }

    fn emit_test_nullness(
        &mut self,
        actual: BasicValueEnum<'ctx>,
        actual_ty: &Type,
        defined: bool,
        matcher_name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let passed = if matches!(actual_ty, Type::Optional(_)) {
            let is_null = self
                .builder
                .build_is_null(actual.into_pointer_value(), "test.null")
                .map_err(|error| error.to_string())?;
            if defined {
                self.builder
                    .build_not(is_null, "test.defined")
                    .map_err(|error| error.to_string())?
            } else {
                is_null
            }
        } else if defined {
            self.context.bool_type().const_int(1, false)
        } else {
            self.context.bool_type().const_zero()
        };
        let _ = (actual, matcher_name);
        Ok(passed)
    }

    fn emit_test_ordering(
        &mut self,
        actual: BasicValueEnum<'ctx>,
        args: &[Expr],
        actual_ty: &Type,
        matcher_name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let expected = self.emit_expr(&args[0])?;
        match resolve_scalar_kind(actual_ty) {
            ScalarKind::Int => self
                .builder
                .build_int_compare(
                    int_ordering_predicate(actual_ty, matcher_name),
                    actual.into_int_value(),
                    expected.into_int_value(),
                    "test.ordering",
                )
                .map_err(|error| error.to_string()),
            ScalarKind::Float => {
                let predicate = match matcher_name {
                    "toBeGreaterThan" => FloatPredicate::OGT,
                    "toBeGreaterThanOrEqual" => FloatPredicate::OGE,
                    "toBeLessThan" => FloatPredicate::OLT,
                    "toBeLessThanOrEqual" => FloatPredicate::OLE,
                    _ => unreachable!(),
                };
                self.builder
                    .build_float_compare(
                        predicate,
                        actual.into_float_value(),
                        expected.into_float_value(),
                        "test.ordering",
                    )
                    .map_err(|error| error.to_string())
            }
            _ => Err(format!(
                "error: `{matcher_name}` on a value of type `{actual_ty}` is not supported yet"
            )),
        }
    }

    fn emit_test_close_to(
        &mut self,
        actual: BasicValueEnum<'ctx>,
        args: &[Expr],
        actual_ty: &Type,
        matcher_name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let ScalarKind::Float = resolve_scalar_kind(actual_ty) else {
            return Err(format!(
                "error: `{matcher_name}` expects a floating-point actual value, found `{actual_ty}`"
            ));
        };
        let expected = self.emit_expr(&args[0])?.into_float_value();
        let diff = self
            .builder
            .build_float_sub(actual.into_float_value(), expected, "test.closeto.diff")
            .map_err(|error| error.to_string())?;
        let threshold = self.context.f64_type().const_float(0.005);
        let lower = self
            .builder
            .build_float_compare(
                FloatPredicate::OGT,
                diff,
                self.context.f64_type().const_float(-0.005),
                "test.closeto.lower",
            )
            .map_err(|error| error.to_string())?;
        let upper = self
            .builder
            .build_float_compare(FloatPredicate::OLT, diff, threshold, "test.closeto.upper")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_and(lower, upper, "test.closeto")
            .map_err(|error| error.to_string())
    }

    fn emit_test_has_length(
        &mut self,
        actual: BasicValueEnum<'ctx>,
        args: &[Expr],
        actual_ty: &Type,
        matcher_name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let expected = self.emit_expr(&args[0])?.into_int_value();
        let length = match transparent_codegen_type(actual_ty) {
            Type::Array(_) => self
                .builder
                .build_extract_value(actual.into_struct_value(), 1, "test.length.array")
                .map_err(|error| error.to_string())?
                .into_int_value(),
            Type::Str => self.string_byte_len(actual.into_struct_value())?,
            _ => {
                return Err(format!(
                    "error: `{matcher_name}` on a value of type `{actual_ty}` is not supported yet"
                ));
            }
        };
        let expected = self
            .builder
            .build_int_cast(expected, length.get_type(), "test.length.expected")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_int_compare(IntPredicate::EQ, length, expected, "test.length")
            .map_err(|error| error.to_string())
    }

    fn emit_test_contains(
        &mut self,
        actual: BasicValueEnum<'ctx>,
        args: &[Expr],
        actual_ty: &Type,
        matcher_name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        if let Type::Array(element) = transparent_codegen_type(actual_ty) {
            return self.emit_test_array_contains(
                actual,
                &args[0],
                actual_ty,
                element,
                matcher_name,
            );
        }
        self.emit_test_substring(actual, args, actual_ty, matcher_name)
    }

    fn emit_test_array_contains(
        &mut self,
        actual: BasicValueEnum<'ctx>,
        expected: &Expr,
        actual_ty: &Type,
        element: &Type,
        matcher_name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        if matches!(resolve_scalar_kind(element), ScalarKind::Other) {
            return Err(format!(
                "error: `{matcher_name}` on a value of type `{actual_ty}` is not supported yet: only arrays of numbers, `boolean`, `char` or `string` are searched"
            ));
        }
        let array_ty = self.llvm_type(actual_ty);
        let array_slot = self.entry_alloca(array_ty, "test.contains.array")?;
        self.builder
            .build_store(array_slot, actual)
            .map_err(|error| error.to_string())?;
        let needle = self.emit_expr_as(expected, element)?;
        let needle_slot = self.entry_alloca(self.llvm_type(element), "test.contains.value")?;
        self.builder
            .build_store(needle_slot, needle)
            .map_err(|error| error.to_string())?;
        let element_size = self
            .llvm_type(element)
            .size_of()
            .ok_or_else(|| format!("error: array element type `{element}` has no native size"))?;
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let index_of = self.declare_libc_fn(
            "tyrus_array_index_of",
            &[
                ptr_ty.into(),
                ptr_ty.into(),
                self.context.i64_type().into(),
                self.context.i32_type().into(),
                self.context.bool_type().into(),
            ],
            Some(self.context.i64_type().into()),
        );
        let index = self
            .builder
            .build_call(
                index_of,
                &[
                    array_slot.into(),
                    needle_slot.into(),
                    element_size.into(),
                    self.context
                        .i32_type()
                        .const_int(self.array_element_kind(element), false)
                        .into(),
                    self.context.bool_type().const_zero().into(),
                ],
                "test.contains.index",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "internal: array search returned no value".to_string())?
            .into_int_value();
        self.builder
            .build_int_compare(
                IntPredicate::NE,
                index,
                self.context.i64_type().const_all_ones(),
                "test.contains.found",
            )
            .map_err(|error| error.to_string())
    }

    fn emit_test_matches(
        &mut self,
        actual: BasicValueEnum<'ctx>,
        args: &[Expr],
        actual_ty: &Type,
        matcher_name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        self.emit_test_substring(actual, args, actual_ty, matcher_name)
    }

    fn emit_test_substring(
        &mut self,
        actual: BasicValueEnum<'ctx>,
        args: &[Expr],
        actual_ty: &Type,
        matcher_name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        if !matches!(transparent_codegen_type(actual_ty), Type::Str) {
            return Err(format!(
                "error: `{matcher_name}` on a value of type `{actual_ty}` is not supported yet"
            ));
        }
        let needle = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
        // `tyrus_string_index_of` takes the position to start from as a third
        // argument, and searching a whole string means starting at zero.
        // Declaring it with two left that parameter reading whatever the
        // register happened to hold: usually zero, so every test passed, and
        // under AddressSanitizer something else, so the search started past the
        // end and `toContain`/`toMatch` answered false for a substring that was
        // right there.
        let index_of = self.declare_libc_fn(
            "tyrus_string_index_of",
            &[
                self.string_abi_type(),
                self.string_abi_type(),
                self.context.i64_type().into(),
            ],
            Some(self.context.i64_type().into()),
        );
        let from_start = self.context.i64_type().const_zero();
        let index = self
            .builder
            .build_call(
                index_of,
                &[
                    actual.into_struct_value().into(),
                    needle.into(),
                    from_start.into(),
                ],
                "test.substring",
            )
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| "internal: string search returned no value".to_string())?
            .into_int_value();
        self.builder
            .build_int_compare(
                IntPredicate::NE,
                index,
                self.context.i64_type().const_all_ones(),
                "test.substring.found",
            )
            .map_err(|error| error.to_string())
    }

    fn coerce_int_eq(
        &self,
        left: IntValue<'ctx>,
        right: IntValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let right = if left.get_type() != right.get_type() {
            self.builder
                .build_int_cast(right, left.get_type(), "test.equal.coerced")
                .map_err(|error| error.to_string())?
        } else {
            right
        };
        self.builder
            .build_int_compare(IntPredicate::EQ, left, right, name)
            .map_err(|error| error.to_string())
    }
}

const ACTUAL_TEMP: &str = "__tyrus_test_actual";
const EXPECTED_TEMP: &str = "__tyrus_test_expected";
const REJECTION_TEMP: &str = "__tyrus_test_rejection";

fn expected_operand_type(matcher_name: &str, actual_ty: &Type) -> Option<Type> {
    match matcher_name {
        "toBe"
        | "toEqual"
        | "toStrictEqual"
        | "toBeGreaterThan"
        | "toBeGreaterThanOrEqual"
        | "toBeLessThan"
        | "toBeLessThanOrEqual"
        | "toBeCloseTo" => match resolve_scalar_kind(actual_ty) {
            ScalarKind::Other => None,
            _ => Some(actual_ty.clone()),
        },
        "toHaveLength" => Some(Type::I64),
        "toContain" => match transparent_codegen_type(actual_ty) {
            Type::Array(element) => Some(element.as_ref().clone()),
            _ => Some(Type::Str),
        },
        "toMatch" => Some(Type::Str),
        _ => None,
    }
}

fn is_renderable(ty: &Type) -> bool {
    !matches!(ty, Type::Optional(_)) && !matches!(resolve_scalar_kind(ty), ScalarKind::Other)
}

fn assertion_message(
    matcher_name: &str,
    negative: bool,
    shows_actual: Option<bool>,
    has_expected: Option<bool>,
    subject: &str,
) -> Expr {
    let relation = match matcher_name {
        "toBe" => "to be",
        "toEqual" => "to equal",
        "toStrictEqual" => "to strictly equal",
        "toBeTruthy" => "to be truthy",
        "toBeFalsy" => "to be falsy",
        "toBeNull" => "to be null",
        "toBeDefined" => "to be defined",
        "toBeUndefined" => "to be undefined",
        "toBeGreaterThan" => "to be greater than",
        "toBeGreaterThanOrEqual" => "to be greater than or equal to",
        "toBeLessThan" => "to be less than",
        "toBeLessThanOrEqual" => "to be less than or equal to",
        "toBeCloseTo" => "to be close to",
        "toHaveLength" => "to have length",
        "toContain" => "to contain",
        "toMatch" => "to match",
        other => other,
    };

    fn operand(parts: &mut Vec<TemplatePart>, name: &str, quoted: bool) {
        let quote = if quoted { "\"" } else { "" };
        parts.push(TemplatePart::Text(quote.to_string()));
        parts.push(TemplatePart::Expr(
            Expr::synthetic(ExprKind::Ident(name.to_string())),
            None,
        ));
        parts.push(TemplatePart::Text(quote.to_string()));
    }

    let mut parts = vec![TemplatePart::Text("expected ".to_string())];
    match shows_actual {
        Some(quoted) => {
            operand(&mut parts, ACTUAL_TEMP, quoted);
            parts.push(TemplatePart::Text(" ".to_string()));
        }
        None => parts.push(TemplatePart::Text(format!("{subject} "))),
    }
    if negative {
        parts.push(TemplatePart::Text("not ".to_string()));
    }
    parts.push(TemplatePart::Text(relation.to_string()));
    if let Some(quoted) = has_expected {
        parts.push(TemplatePart::Text(" ".to_string()));
        operand(&mut parts, EXPECTED_TEMP, quoted);
    }
    Expr::synthetic(ExprKind::TemplateLit(parts))
}

enum ScalarKind {
    Str,
    Int,
    Float,
    Bool,
    Char,
    Other,
}

fn resolve_scalar_kind(ty: &Type) -> ScalarKind {
    match ty {
        Type::Str | Type::Regex => ScalarKind::Str,
        Type::I8
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
        | Type::Usize => ScalarKind::Int,
        Type::F32 | Type::F64 => ScalarKind::Float,
        Type::Bool => ScalarKind::Bool,
        Type::Char => ScalarKind::Char,
        _ => ScalarKind::Other,
    }
}

fn int_ordering_predicate(ty: &Type, matcher_name: &str) -> IntPredicate {
    let unsigned = matches!(
        ty,
        Type::U8 | Type::U16 | Type::U32 | Type::U64 | Type::U128 | Type::Usize
    );
    let lesser = matcher_name.starts_with("toBeLessThan");
    let or_equal = matcher_name.ends_with("OrEqual");
    match (unsigned, lesser, or_equal) {
        (true, true, true) => IntPredicate::ULE,
        (true, true, false) => IntPredicate::ULT,
        (true, false, true) => IntPredicate::UGE,
        (true, false, false) => IntPredicate::UGT,
        (false, true, true) => IntPredicate::SLE,
        (false, true, false) => IntPredicate::SLT,
        (false, false, true) => IntPredicate::SGE,
        (false, false, false) => IntPredicate::SGT,
    }
}
