use super::*;
use crate::statements::ThrownValue;
use tyrus_core::ast::ExprKind;
use tyrus_core::span::Span;
use tyrus_decimal::Decimal;

fn codegen_const_int(expr: &Expr) -> Option<i128> {
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

/// Parse a decimal literal without routing through a binary float. The pair is
/// the temporary ABI representation `coefficient × 10^exponent`; runtime
/// arithmetic will apply the language precision and rounding contract.
fn codegen_decimal_literal(text: &str) -> Result<(i128, i32), String> {
    Decimal::parse(text)
        .map(|value| (value.coefficient, value.exponent))
        .ok_or_else(|| "decimal literal is outside the decimal range".to_string())
}

fn codegen_constant_decimal(expr: &Expr) -> Option<Result<Decimal, String>> {
    match expr.kind() {
        ExprKind::DecimalLit(value) => Some(codegen_decimal_literal(value.text()).map(
            |(coefficient, exponent)| Decimal {
                coefficient,
                exponent,
            },
        )),
        ExprKind::Neg(inner) => codegen_constant_decimal(inner).map(|value| {
            value.and_then(|value| {
                Decimal {
                    coefficient: value
                        .coefficient
                        .checked_neg()
                        .ok_or_else(|| "decimal result is out of range".to_string())?,
                    exponent: value.exponent,
                }
                .normalized()
                .map_err(|_| "decimal result is out of range".to_string())
            })
        }),
        ExprKind::FieldAccess { object, field } if matches!(object.kind(), ExprKind::Ident(name) if name == "decimal") =>
        {
            let coefficient = match field.as_str() {
                "MIN" => -Decimal::MAX_COEFFICIENT,
                "MAX" => Decimal::MAX_COEFFICIENT,
                _ => return None,
            };
            Some(Ok(Decimal {
                coefficient,
                exponent: Decimal::MAX_ADJUSTED_EXPONENT - 33,
            }))
        }
        ExprKind::BinOp { op, left, right } => {
            let left = codegen_constant_decimal(left)?;
            let right = codegen_constant_decimal(right)?;
            Some(left.and_then(|left| {
                right.and_then(|right| {
                    match op {
                        BinOp::Add => left.add(right),
                        BinOp::Sub => left.sub(right),
                        BinOp::Mul => left.mul(right),
                        BinOp::Div => left.div(right),
                        BinOp::Pow => left.pow(right),
                        _ => return Err("not a decimal arithmetic expression".to_string()),
                    }
                    .map_err(|error| format!("decimal constant evaluation failed: {error:?}"))
                })
            }))
        }
        _ => None,
    }
}

fn is_numeric_type(ty: &Type) -> bool {
    is_integer_type(ty) || is_float_type(ty)
}

fn promise_native_field(name: &str, ty: Type) -> Field {
    Field {
        name: name.to_string(),
        ty,
        readonly: false,
        optional: false,
        decorators: Vec::new(),
        span: Span::DUMMY,
    }
}

fn promise_settled_codegen_type(payload: Type) -> Type {
    Type::Union(vec![
        Type::Object(vec![
            promise_native_field(
                "status",
                Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(
                    "fulfilled".to_string(),
                )))),
            ),
            promise_native_field("value", payload),
        ]),
        Type::Object(vec![
            promise_native_field(
                "status",
                Type::Lit(Box::new(Expr::synthetic(ExprKind::StrLit(
                    "rejected".to_string(),
                )))),
            ),
            promise_native_field("reason", Type::Named("Error".to_string())),
        ]),
    ])
}

fn int_type_bounds(ty: &Type) -> Option<(i128, Option<i128>)> {
    tyrus_core::numeric_types::integer_type_bounds(transparent_codegen_type(ty))
}

fn int_type_bits(ty: &Type) -> Option<u32> {
    tyrus_core::numeric_types::integer_type_bits(transparent_codegen_type(ty))
}

fn exact_codegen_interface_match(expected_fields: &[Field], actual_fields: &[Field]) -> bool {
    expected_fields.len() == actual_fields.len()
        && expected_fields.iter().all(|expected| {
            !expected.optional
                && !expected.readonly
                && actual_fields
                    .iter()
                    .find(|actual| actual.name == expected.name)
                    .is_some_and(|actual| {
                        !actual.optional
                            && !actual.readonly
                            && transparent_codegen_type(&actual.ty)
                                == transparent_codegen_type(&expected.ty)
                    })
        })
        && actual_fields.iter().all(|actual| {
            expected_fields
                .iter()
                .any(|expected| expected.name == actual.name)
        })
}

#[path = "algorithm_calls/algorithm_calls.rs"]
mod algorithm_calls;
#[path = "archive_calls/archive_calls.rs"]
mod archive_calls;
#[path = "async_values/async_values.rs"]
mod async_values;
#[path = "closures/closures.rs"]
mod closures;
#[path = "compress_calls/compress_calls.rs"]
mod compress_calls;
#[path = "conversions/conversions.rs"]
mod conversions;
#[path = "db_calls/db_calls.rs"]
mod db_calls;
#[path = "field_access/field_access.rs"]
mod field_access;
#[path = "fn_calls/fn_calls.rs"]
mod fn_calls;
#[path = "fs_calls/fs_calls.rs"]
mod fs_calls;
#[path = "math_calls/math_calls.rs"]
mod math_calls;
#[path = "net_calls/net_calls.rs"]
mod net_calls;
#[path = "operators/operators.rs"]
mod operators;
#[path = "os_calls/os_calls.rs"]
mod os_calls;
#[path = "simd_calls/simd_calls.rs"]
mod simd_calls;
#[path = "task_calls/task_calls.rs"]
mod task_calls;
#[path = "temporal_calls/temporal_calls.rs"]
mod temporal_calls;
#[path = "temporal_plain_calls/temporal_plain_calls.rs"]
mod temporal_plain_calls;
#[path = "type_hints/type_hints.rs"]
mod type_hints;

impl<'ctx> Codegen<'ctx> {
    // The async splitter rewrites an awaited expression to an `await#N` local.
    // On `Ok`, `?` moves its payload into the declaration, so the source frame
    // slot must stop owning it before the next suspension or frame drop.
    fn clear_consumed_async_await_value(
        &mut self,
        expression: &Expr,
        value_type: &Type,
    ) -> Result<(), String> {
        let ExprKind::Ident(name) = expression.kind() else {
            return Ok(());
        };
        if !name.starts_with("await#") {
            return Ok(());
        }
        let Some((slot, stored_type)) = self.async_frame_slot(name) else {
            return Err(format!(
                "internal: awaited `Result` temporary `{name}` has no async frame slot"
            ));
        };
        if self.resolve_type(&stored_type) != self.resolve_type(value_type) {
            return Err(format!(
                "internal: awaited value type `{stored_type}` does not match consumed `Result` type `{value_type}`"
            ));
        }
        self.mark_async_frame_value_initialized(name, false);
        let zero = self.llvm_type(&stored_type).const_zero();
        self.builder
            .build_store(slot, zero)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub(super) fn native_value_slot(
        &self,
        ty: BasicTypeEnum<'ctx>,
        value: BasicValueEnum<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let slot = self
            .builder
            .build_alloca(ty, name)
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(slot, value)
            .map_err(|error| error.to_string())?;
        Ok(slot)
    }

    fn const_i128_value(&self, value: i128) -> IntValue<'ctx> {
        let bits = value as u128;
        self.context
            .i128_type()
            .const_int_arbitrary_precision(&[bits as u64, (bits >> 64) as u64])
    }

    fn const_u128_value(&self, value: u128) -> IntValue<'ctx> {
        self.context
            .i128_type()
            .const_int_arbitrary_precision(&[value as u64, (value >> 64) as u64])
    }

    pub(super) fn emit_runtime_panic(&self, message: &str) {
        let msg_ptr = self
            .builder
            .build_global_string_ptr(message, "panic.message")
            .unwrap();
        let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
        let panic_fn = self.declare_libc_fn("tyrus_panic", &[ptr_type.into()], None);
        self.builder
            .build_call(panic_fn, &[msg_ptr.as_pointer_value().into()], "")
            .unwrap();
        self.builder.build_unreachable().unwrap();
    }

    pub(super) fn emit_zero_check(&self, divisor: IntValue<'ctx>, message: &str) {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("internal: zero check outside function");
        let zero_block = self.context.append_basic_block(function, "div.zero");
        let ok_block = self.context.append_basic_block(function, "div.ok");
        let is_zero = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                divisor,
                divisor.get_type().const_zero(),
                "divisor_zero",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(is_zero, zero_block, ok_block)
            .unwrap();
        self.builder.position_at_end(zero_block);
        self.emit_runtime_panic(message);
        self.builder.position_at_end(ok_block);
    }

    fn expr_is_known_non_zero(&self, expr: &Expr) -> bool {
        matches!(expr.kind(), ExprKind::Ident(name) if self.load_non_zero_var(name))
    }

    pub(super) fn emit_signed_overflow_check(
        &self,
        a: IntValue<'ctx>,
        b: IntValue<'ctx>,
        result: IntValue<'ctx>,
        same_sign_operands: bool,
        op_name: &str,
    ) {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("internal: overflow check outside function");
        let overflow_block = self.context.append_basic_block(function, "overflow.error");
        let ok_block = self.context.append_basic_block(function, "overflow.ok");

        let zero = a.get_type().const_zero();
        let a_sign = self
            .builder
            .build_int_compare(IntPredicate::SLT, a, zero, "a_sign")
            .unwrap();
        let b_sign = self
            .builder
            .build_int_compare(IntPredicate::SLT, b, zero, "b_sign")
            .unwrap();
        let r_sign = self
            .builder
            .build_int_compare(IntPredicate::SLT, result, zero, "r_sign")
            .unwrap();
        let operands_same_sign = self
            .builder
            .build_xor(a_sign, b_sign, "operands_same_sign")
            .unwrap();
        let operands_same_sign = self
            .builder
            .build_not(operands_same_sign, "operands_same_sign_not")
            .unwrap();
        let expected = if same_sign_operands {
            operands_same_sign
        } else {
            self.builder
                .build_not(operands_same_sign, "expected_diff_sign")
                .unwrap()
        };
        let sign_diff = self.builder.build_xor(a_sign, r_sign, "sign_diff").unwrap();
        let overflow = self
            .builder
            .build_and(expected, sign_diff, "overflow")
            .unwrap();
        self.builder
            .build_conditional_branch(overflow, overflow_block, ok_block)
            .unwrap();

        self.builder.position_at_end(overflow_block);
        let message = format!("tyrus runtime error: integer overflow in `{op_name}`");
        self.emit_runtime_panic(&message);
        self.builder.position_at_end(ok_block);
    }

    pub(super) fn emit_char_range_check(&self, value: IntValue<'ctx>, source: &Type) {
        let fits = self.emit_char_fits(value, source);
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("internal: char range check outside function");
        let error_block = self.context.append_basic_block(function, "cast.char.error");
        let ok_block = self.context.append_basic_block(function, "cast.char.ok");
        self.builder
            .build_conditional_branch(fits, ok_block, error_block)
            .unwrap();

        self.builder.position_at_end(error_block);
        self.emit_runtime_panic("tyrus runtime error: value is not a valid `char`");
        self.builder.position_at_end(ok_block);
    }

    fn emit_style_text(&mut self, expr: &Expr) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let form = {
            let is_style_text = |name: &str| name == "styleText" && self.load_var(name).is_none();
            tyrus_core::style_text::as_style_text(expr, &is_style_text)
        }?;
        match form {
            tyrus_core::style_text::StyleTextForm::Styled { styles, text } => {
                Some(self.emit_style_text_call(&styles, text))
            }
            _ => Some(Err(
                "internal: malformed styleText reached codegen".to_string()
            )),
        }
    }

    fn emit_style_text_call(
        &mut self,
        styles: &[String],
        text: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let open = tyrus_core::style_text::ansi_open(styles)
            .map_err(|name| format!("internal: unknown text style `{name}`"))?;
        let text_value = self.emit_expr(text)?;
        let open_ptr = self
            .const_string_abi(&open, "style.open")
            .into_struct_value();
        let close_ptr = self
            .const_string_abi(tyrus_core::style_text::ANSI_RESET, "style.close")
            .into_struct_value();
        let style_fn = self.declare_libc_fn(
            "tyrus_style",
            &[
                self.string_abi_type(),
                self.string_abi_type(),
                self.string_abi_type(),
            ],
            Some(self.string_abi_type()),
        );
        Ok(self
            .builder
            .build_call(
                style_fn,
                &[open_ptr.into(), text_value.into(), close_ptr.into()],
                "styled",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap())
    }

    fn emit_figlet(&mut self, expr: &Expr) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let form = {
            let is_figlet = |name: &str| name == "figlet" && self.load_var(name).is_none();
            tyrus_core::figlet::as_figlet(expr, &is_figlet)
        }?;
        match form {
            tyrus_core::figlet::FigletForm::Rendered { text, font } => {
                Some(self.emit_figlet_call(text, font))
            }
            _ => Some(Err("internal: malformed figlet reached codegen".to_string())),
        }
    }

    fn emit_figlet_call(
        &mut self,
        text: &Expr,
        font: Option<&str>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let text_value = self.emit_expr(text)?;
        let font = font.unwrap_or(tyrus_core::figlet::DEFAULT_FONT);
        let font_ptr = self
            .const_string_abi(font, "figlet.font")
            .into_struct_value();
        let figlet_fn = self.declare_libc_fn(
            "tyrus_figlet",
            &[self.string_abi_type(), self.string_abi_type()],
            Some(self.string_abi_type()),
        );
        Ok(self
            .builder
            .build_call(figlet_fn, &[text_value.into(), font_ptr.into()], "figlet")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap())
    }

    fn emit_array_static_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        if !matches!(object.kind(), ExprKind::Ident(name) if name == "Array")
            || self.load_var("Array").is_some()
        {
            return None;
        }
        if method == "of" {
            let element = match self.infer_array_literal_type(args) {
                Some(element) => element,
                None => return Some(Err("error: cannot infer Array.of element type".to_string())),
            };
            return Some(self.emit_array_literal(args, &element));
        }
        if method == "from" {
            let Some(source) = args.first() else {
                return Some(Err("error: Array.from expects 1 argument".to_string()));
            };
            let Some(array_ty) = self.expr_type_hint(source) else {
                return Some(Err("error: cannot infer Array.from source type".to_string()));
            };
            let Type::Array(element) = transparent_codegen_type(&array_ty) else {
                return Some(Err(
                    "error: Array.from requires an array argument".to_string()
                ));
            };
            let element = element.as_ref().clone();
            let result = self
                .emit_array_method(source, "values", &[])
                .map_err(|_| format!("error: cannot copy array of `{element}`"));
            return Some(result.inspect(|&value| {
                self.record_owned_native_result(value, Type::Array(Box::new(element)));
            }));
        }
        if method == "withCapacity" {
            return Some(Err(
                "error: `Array.withCapacity` requires a type annotation (e.g. `const arr: i32[] = Array.withCapacity(10)`)".to_string()
            ));
        }
        if method != "isArray" {
            return Some(Err(format!("error: unknown Array method `{method}`")));
        }
        let Some(arg) = args.first() else {
            return Some(Err(
                "error: method `Array.isArray` expects 1 argument, but 0 were provided".to_string(),
            ));
        };
        if args.len() != 1 {
            return Some(Err(format!(
                "error: method `Array.isArray` expects 1 argument, but {} were provided",
                args.len()
            )));
        }
        let is_array = self
            .expr_type_hint(arg)
            .is_some_and(|ty| matches!(transparent_codegen_type(&ty), Type::Array(_)));
        Some(self.emit_expr(arg).map(|_| {
            self.context
                .bool_type()
                .const_int(is_array as u64, false)
                .into()
        }))
    }

    fn emit_array_with_capacity(
        &mut self,
        args: &[Expr],
        element: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let Some(capacity_expr) = args.first() else {
            return Err("error: Array.withCapacity expects 1 argument".to_string());
        };
        let capacity = self.emit_expr_as(capacity_expr, &Type::Usize)?;
        let i64_type = self.context.i64_type();
        let capacity_i64 = self
            .builder
            .build_int_cast(capacity.into_int_value(), i64_type, "capacity_i64")
            .map_err(|e| format!("error: failed to cast capacity: {e}"))?;
        let llvm_element = self.llvm_type(element);
        let element_size = llvm_element
            .size_of()
            .ok_or_else(|| format!("error: array element type `{element}` has no native size"))?;
        let element_size_i64 = self
            .builder
            .build_int_cast(element_size, i64_type, "element_size_i64")
            .map_err(|e| format!("error: failed to cast element size: {e}"))?;
        let array_type = self.context.struct_type(
            &[
                self.context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into(),
                self.usize_type().into(),
                self.usize_type().into(),
            ],
            false,
        );
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let output_alloca = self.entry_alloca(array_type, "with_capacity.output")?;
        let function = self.declare_libc_fn(
            "tyrus_array_with_capacity",
            &[i64_type.into(), i64_type.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        let status = self
            .builder
            .build_call(
                function,
                &[
                    capacity_i64.into(),
                    element_size_i64.into(),
                    output_alloca.into(),
                ],
                "with_capacity_status",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .ok_or("error: tyrus_array_with_capacity did not return a status")?
            .into_int_value();
        self.emit_abort_if_negative(status, "withCapacity", "array.constructor")?;
        let result = self
            .builder
            .build_load(array_type, output_alloca, "with_capacity.array")
            .unwrap();
        self.record_owned_native_result(result, Type::Array(Box::new(element.clone())));
        Ok(result)
    }

    pub(super) fn release_borrowed_optional_argument(
        &mut self,
        argument: &Expr,
        value: BasicValueEnum<'ctx>,
    ) {
        if matches!(argument.kind(), ExprKind::Null | ExprKind::None) {
            return;
        }
        let source_is_optional = self
            .expr_type_hint(argument)
            .map(|hint| self.resolve_type(&hint))
            .is_some_and(|hint| matches!(transparent_codegen_type(&hint), Type::Optional(_)));
        if source_is_optional || !value.is_pointer_value() {
            return;
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[value.into()], "native.optional.argument.release")
            .unwrap();
    }

    pub(super) fn release_borrowed_optional_array_argument(
        &mut self,
        argument: &Expr,
        value: BasicValueEnum<'ctx>,
        element: &Type,
    ) -> Result<(), String> {
        if matches!(argument.kind(), ExprKind::Null | ExprKind::None) || !value.is_pointer_value() {
            return Ok(());
        }
        let source_is_optional = self
            .expr_type_hint(argument)
            .map(|hint| self.resolve_type(&hint))
            .is_some_and(|hint| matches!(transparent_codegen_type(&hint), Type::Optional(_)));
        if source_is_optional {
            return Ok(());
        }
        let array_type = Type::Array(Box::new(element.clone()));
        let array = self
            .builder
            .build_load(
                self.llvm_type(&array_type),
                value.into_pointer_value(),
                "native.optional.array.argument",
            )
            .map_err(|error| error.to_string())?;
        self.emit_async_cleanup_source_expression(array, &array_type, argument)?;
        self.release_borrowed_optional_argument(argument, value);
        Ok(())
    }

    pub(super) fn emit_join_handle_instance_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let object_type = self
            .expr_type_hint(object)
            .map(|ty| self.resolve_type(&ty))?;
        let Type::Apply {
            name,
            args: handle_args,
        } = object_type
        else {
            return None;
        };
        if name != "JoinHandle" || handle_args.len() != 1 {
            return None;
        }
        if !matches!(method, "join" | "abort") {
            return Some(Err(format!("error: unknown JoinHandle method `{method}`")));
        }
        if !args.is_empty() {
            return Some(Err(format!(
                "error: `JoinHandle.{method}` expects 0 arguments, found {}",
                args.len()
            )));
        }
        Some((|| {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let source = self.emit_expr(object)?.into_pointer_value();
            if method == "abort" {
                let reason = self
                    .const_string_abi("Aborted by caller", "join_handle.abort.reason")
                    .into_struct_value();
                let i64_type = self.context.i64_type();
                let cancel = self.declare_libc_fn(
                    "tyrus_task_cancel",
                    &[pointer.into(), self.string_abi_type(), i64_type.into()],
                    Some(self.context.i32_type().into()),
                );
                self.builder
                    .build_call(
                        cancel,
                        &[
                            source.into(),
                            reason.into(),
                            i64_type.const_int(u64::MAX, false).into(),
                        ],
                        "join_handle.abort",
                    )
                    .unwrap();
                return Ok(self.context.i32_type().const_zero().into());
            }
            // `.join()` consumes the handle, mirrors real Rust, where
            // `JoinHandle::join` takes `self` by value, rather than
            // retaining a second reference and leaving the original to leak:
            // `handle` is the *only* `*mut TyrusPromise` reference this
            // `JoinHandle<T>` ever held, so forwarding it as-is (no retain,
            // no free-after) transfers ownership to whatever awaits the
            // result, with nothing left over to release. The borrow checker
            // enforces the "exactly once" half of that (`maybe_move` in
            // `borrow_checker.rs`): a second `.join()`/`.abort()` on the same
            // handle is `E0382`, use of a moved value, not a runtime bug.
            Ok(source.into())
        })())
    }

    pub(super) fn owned_join_handle_expression(&self, expression: &Expr) -> bool {
        matches!(
            expression.kind(),
            ExprKind::FnCall { .. } | ExprKind::MethodCall { .. }
        ) && self.expr_type_hint(expression).is_some_and(|ty| {
            matches!(
                self.resolve_type(&ty),
                Type::Apply { name, args } if name == "JoinHandle" && args.len() == 1
            )
        })
    }

    pub(super) fn owned_mutex_expression(&self, expression: &Expr) -> bool {
        matches!(expression.kind(), ExprKind::FnCall { .. })
            && self.expr_type_hint(expression).is_some_and(|ty| {
                matches!(
                    self.resolve_type(&ty),
                    Type::Apply { name, args } if name == "Mutex" && args.len() == 1
                )
            })
    }

    pub(super) fn owned_rwlock_expression(&self, expression: &Expr) -> bool {
        matches!(expression.kind(), ExprKind::FnCall { .. })
            && self.expr_type_hint(expression).is_some_and(|ty| {
                matches!(
                    self.resolve_type(&ty),
                    Type::Apply { name, args } if name == "RwLock" && args.len() == 1
                )
            })
    }

    /// Whether `expression` is a fresh `Semaphore(...)` constructor call.
    /// Mirrors `owned_mutex_expression`, except `Semaphore` is `Type::Named`
    /// (no type parameter), not `Type::Apply`.
    /// Releases a `string` built for a native call's argument, once the call
    /// has read it.
    ///
    /// Every one of these runtime entry points copies the text on the calling
    /// thread, because the work outlives the call: `address.as_str().to_owned()`
    /// is the first thing each of them does. So the value built for the
    /// argument has no reader left the moment the call returns, and nothing was
    /// releasing it. A URL written inline, as `fetch.get(`${base}/note`)`,
    /// leaked one string per request, while the same call with the URL in a
    /// binding was clean, which is what made it read as a style question rather
    /// than a leak.
    ///
    /// The distinction is the whole point of routing through the
    /// expression-aware cleanup rather than freeing what was handed over: a
    /// binding passed in stays the caller's, and only an allocation made for
    /// this call is given up. Inside an async frame it also settles *where* the
    /// release goes, which is why the inline form used to fail to compile at
    /// all in a nested closure: with no registration consumed here, the pending
    /// entry was swept later, from a block that did not dominate the value.
    pub(super) fn emit_native_string_argument_release(
        &mut self,
        value: StructValue<'ctx>,
        expression: &Expr,
    ) -> Result<(), String> {
        self.emit_async_cleanup_source_expression(value.into(), &Type::Str, expression)
    }

    pub(super) fn owned_semaphore_expression(&self, expression: &Expr) -> bool {
        matches!(expression.kind(), ExprKind::FnCall { .. })
            && self.expr_type_hint(expression).is_some_and(|ty| {
                matches!(
                    self.resolve_type(&ty),
                    Type::Named(name) if name == "Semaphore"
                )
            })
    }

    pub(super) fn owned_http_server_expression(&self, expression: &Expr) -> bool {
        matches!(expression.kind(), ExprKind::MethodCall { method, .. } if method == "start")
            && self.expr_type_hint(expression).is_some_and(|ty| {
                matches!(
                    self.resolve_type(&ty),
                    Type::Named(name) if name == "HttpServer"
                )
            })
    }

    pub(super) fn owned_optional_sync_guard_expression(&self, expression: &Expr) -> bool {
        let ExprKind::MethodCall { method, .. } = expression.kind() else {
            return false;
        };
        if !matches!(
            method.as_str(),
            "tryLock" | "tryRead" | "tryWrite" | "tryAcquire"
        ) {
            return false;
        }
        self.expr_type_hint(expression).is_some_and(|ty| {
            let Type::Optional(inner) = self.resolve_type(&ty) else {
                return false;
            };
            matches!(
                transparent_codegen_type(&inner),
                Type::Apply { name, args }
                    if args.len() == 1
                        && matches!(name.as_str(), "MutexGuard" | "RwReadGuard" | "RwWriteGuard")
            ) || matches!(
                transparent_codegen_type(&inner),
                Type::Named(name) if name == "SemaphorePermit"
            )
        })
    }

    pub(super) fn emit_sender_instance_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let object_type = self
            .expr_type_hint(object)
            .map(|ty| self.resolve_type(&ty))?;
        let Type::Apply {
            name,
            args: sender_args,
        } = object_type
        else {
            return None;
        };
        if name != "Sender" || sender_args.len() != 1 {
            return None;
        }
        let payload_ty = sender_args[0].clone();
        if !matches!(method, "send" | "trySend") {
            return Some(Err(format!("error: unknown Sender method `{method}`")));
        }
        Some((|| {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let value = args
                .first()
                .ok_or_else(|| format!("error: `Sender.{method}` expects a value argument"))?;
            let emitted = self.emit_expr_as(value, &payload_ty)?;
            let emitted = if matches!(transparent_codegen_type(&payload_ty), Type::Str) {
                self.emit_string_clone(emitted.into_struct_value(), "channel.send.string.clone")?
            } else {
                emitted
            };
            let boxed = self.heap_alloc(self.llvm_type(&payload_ty), "channel.send.payload")?;
            self.builder
                .build_store(boxed, emitted)
                .map_err(|error| error.to_string())?;
            let drop_symbol = match transparent_codegen_type(&payload_ty) {
                Type::Str => "tyrus_drop_async_string",
                Type::Array(element) if element.is_copy() => "tyrus_drop_async_copy_array",
                _ if payload_ty.is_copy() => "tyrus_drop_async_box",
                _ => {
                    return Err(format!(
                        "error: channel payload `{payload_ty}` is not supported by native code generation"
                    ));
                }
            };
            let drop_payload = self.declare_libc_fn(drop_symbol, &[pointer.into()], None);
            let sender = self.emit_expr(object)?.into_pointer_value();
            if method == "send" {
                let send = self.declare_libc_fn(
                    "tyrus_channel_send",
                    &[pointer.into(), pointer.into(), pointer.into()],
                    Some(pointer.into()),
                );
                Ok(self
                    .builder
                    .build_call(
                        send,
                        &[
                            sender.into(),
                            boxed.into(),
                            drop_payload.as_global_value().as_pointer_value().into(),
                        ],
                        "channel.send",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap())
            } else {
                let try_send = self.declare_libc_fn(
                    "tyrus_channel_try_send",
                    &[pointer.into(), pointer.into(), pointer.into()],
                    Some(self.context.i32_type().into()),
                );
                let accepted = self
                    .builder
                    .build_call(
                        try_send,
                        &[
                            sender.into(),
                            boxed.into(),
                            drop_payload.as_global_value().as_pointer_value().into(),
                        ],
                        "channel.try_send",
                    )
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap()
                    .into_int_value();
                let boolean = self
                    .builder
                    .build_int_truncate(accepted, self.context.bool_type(), "channel.try_send.bool")
                    .map_err(|error| error.to_string())?;
                Ok(boolean.into())
            }
        })())
    }

    pub(super) fn emit_receiver_instance_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let object_type = self
            .expr_type_hint(object)
            .map(|ty| self.resolve_type(&ty))?;
        let Type::Apply {
            name,
            args: receiver_args,
        } = object_type
        else {
            return None;
        };
        if name != "Receiver" || receiver_args.len() != 1 {
            return None;
        }
        if !matches!(method, "recv" | "tryRecv" | "close") {
            return Some(Err(format!("error: unknown Receiver method `{method}`")));
        }
        if !args.is_empty() {
            return Some(Err(format!(
                "error: `Receiver.{method}` expects 0 arguments, found {}",
                args.len()
            )));
        }
        Some((|| {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let receiver = self.emit_expr(object)?.into_pointer_value();
            match method {
                "recv" => {
                    let recv = self.declare_libc_fn(
                        "tyrus_channel_recv",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    Ok(self
                        .builder
                        .build_call(recv, &[receiver.into()], "channel.recv")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap())
                }
                "tryRecv" => {
                    let try_recv = self.declare_libc_fn(
                        "tyrus_channel_try_recv",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    Ok(self
                        .builder
                        .build_call(try_recv, &[receiver.into()], "channel.try_recv")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap())
                }
                _ => {
                    let close =
                        self.declare_libc_fn("tyrus_channel_close", &[pointer.into()], None);
                    self.builder
                        .build_call(close, &[receiver.into()], "")
                        .unwrap();
                    Ok(self.context.i32_type().const_zero().into())
                }
            }
        })())
    }

    pub(super) fn emit_mutex_instance_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let object_type = self
            .expr_type_hint(object)
            .map(|ty| self.resolve_type(&ty))?;
        let Type::Apply {
            name,
            args: mutex_args,
        } = object_type
        else {
            return None;
        };
        if name != "Mutex" || mutex_args.len() != 1 {
            return None;
        }
        if !matches!(method, "lock" | "tryLock") {
            return Some(Err(format!("error: unknown Mutex method `{method}`")));
        }
        if !args.is_empty() {
            return Some(Err(format!(
                "error: `Mutex.{method}` expects 0 arguments, found {}",
                args.len()
            )));
        }
        Some((|| {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let mutex = self.emit_expr(object)?.into_pointer_value();
            match method {
                "lock" => {
                    let lock = self.declare_libc_fn(
                        "tyrus_mutex_lock",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    Ok(self
                        .builder
                        .build_call(lock, &[mutex.into()], "mutex.lock")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap())
                }
                _ => {
                    // `tyrus_mutex_try_lock` hands back `null` (lock held
                    // elsewhere) or the acquired guard's own pointer. That is
                    // *not* an `Optional<MutexGuard<T>>`, this comment used to
                    // claim it was, and the claim cost a segfault: a `T?` here
                    // is a heap box holding the payload, so the bare pointer
                    // has to be wrapped before anything treats it as optional.
                    // `Receiver.tryRecv` genuinely does reuse `T?`'s shape,
                    // because the payload it hands back is already boxed by
                    // `send`; that is the difference this once conflated.
                    let try_lock = self.declare_libc_fn(
                        "tyrus_mutex_try_lock",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    let acquired = self
                        .builder
                        .build_call(try_lock, &[mutex.into()], "mutex.try_lock")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    let guard = Type::Apply {
                        name: "MutexGuard".to_string(),
                        args: mutex_args.clone(),
                    };
                    self.emit_optional_pointer_box(acquired, &guard)
                }
            }
        })())
    }

    pub(super) fn emit_rwlock_instance_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let object_type = self
            .expr_type_hint(object)
            .map(|ty| self.resolve_type(&ty))?;
        let Type::Apply {
            name,
            args: rwlock_args,
        } = object_type
        else {
            return None;
        };
        if name != "RwLock" || rwlock_args.len() != 1 {
            return None;
        }
        if !matches!(method, "read" | "write" | "tryRead" | "tryWrite") {
            return Some(Err(format!("error: unknown RwLock method `{method}`")));
        }
        if !args.is_empty() {
            return Some(Err(format!(
                "error: `RwLock.{method}` expects 0 arguments, found {}",
                args.len()
            )));
        }
        Some((|| {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let rwlock = self.emit_expr(object)?.into_pointer_value();
            match method {
                "read" => {
                    let read = self.declare_libc_fn(
                        "tyrus_rwlock_read",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    Ok(self
                        .builder
                        .build_call(read, &[rwlock.into()], "rwlock.read")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap())
                }
                "write" => {
                    let write = self.declare_libc_fn(
                        "tyrus_rwlock_write",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    Ok(self
                        .builder
                        .build_call(write, &[rwlock.into()], "rwlock.write")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap())
                }
                "tryRead" => {
                    let try_read = self.declare_libc_fn(
                        "tyrus_rwlock_try_read",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    let acquired = self
                        .builder
                        .build_call(try_read, &[rwlock.into()], "rwlock.try_read")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    let guard = Type::Apply {
                        name: "RwReadGuard".to_string(),
                        args: rwlock_args.clone(),
                    };
                    self.emit_optional_pointer_box(acquired, &guard)
                }
                _ => {
                    let try_write = self.declare_libc_fn(
                        "tyrus_rwlock_try_write",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    let acquired = self
                        .builder
                        .build_call(try_write, &[rwlock.into()], "rwlock.try_write")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    let guard = Type::Apply {
                        name: "RwWriteGuard".to_string(),
                        args: rwlock_args.clone(),
                    };
                    self.emit_optional_pointer_box(acquired, &guard)
                }
            }
        })())
    }

    pub(super) fn emit_semaphore_instance_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let object_type = self
            .expr_type_hint(object)
            .map(|ty| self.resolve_type(&ty))?;
        let Type::Named(name) = object_type else {
            return None;
        };
        if name != "Semaphore" {
            return None;
        }
        if !matches!(method, "acquire" | "tryAcquire") {
            return Some(Err(format!("error: unknown Semaphore method `{method}`")));
        }
        if !args.is_empty() {
            return Some(Err(format!(
                "error: `Semaphore.{method}` expects 0 arguments, found {}",
                args.len()
            )));
        }
        Some((|| {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let semaphore = self.emit_expr(object)?.into_pointer_value();
            match method {
                "acquire" => {
                    let acquire = self.declare_libc_fn(
                        "tyrus_semaphore_acquire",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    Ok(self
                        .builder
                        .build_call(acquire, &[semaphore.into()], "semaphore.acquire")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap())
                }
                _ => {
                    let try_acquire = self.declare_libc_fn(
                        "tyrus_semaphore_try_acquire",
                        &[pointer.into()],
                        Some(pointer.into()),
                    );
                    let acquired = self
                        .builder
                        .build_call(try_acquire, &[semaphore.into()], "semaphore.try_acquire")
                        .unwrap()
                        .try_as_basic_value()
                        .basic()
                        .unwrap();
                    self.emit_optional_pointer_box(
                        acquired,
                        &Type::Named("SemaphorePermit".to_string()),
                    )
                }
            }
        })())
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn emit_expr(&mut self, expr: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        if let Some((slot, ty)) = self
            .async_await_values
            .last()
            .and_then(|values| values.get(&expr.span))
            .cloned()
        {
            return self
                .builder
                .build_load(self.llvm_type(&ty), slot, "async.continuation.value")
                .map_err(|error| error.to_string());
        }
        if matches!(
            expr.kind(),
            ExprKind::FnCall { .. }
                | ExprKind::MethodCall { .. }
                | ExprKind::OptionalMethodCall { .. }
                | ExprKind::CallValue { .. }
        ) && let Some(function) = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
        {
            self.emit_async_throw_region_marker(expr.span, function);
        }
        if let Some(styled) = self.emit_style_text(expr) {
            let styled = styled?;
            self.record_owned_native_result(styled, Type::Str);
            return Ok(styled);
        }
        if let Some(rendered) = self.emit_figlet(expr) {
            let rendered = rendered?;
            self.record_owned_native_result(rendered, Type::Str);
            return Ok(rendered);
        }
        match expr {
            Expr {
                kind: ExprKind::Comptime(inner),
                ..
            } => self.emit_expr(inner),
            Expr {
                kind: ExprKind::Try(inner),
                ..
            } => {
                let source_ty = self
                    .expr_type_hint(inner)
                    .map(|ty| self.resolve_type(&ty))
                    .ok_or("error: cannot determine the type propagated by `?`")?;
                let Type::Result { success, error } = source_ty.clone() else {
                    return Err("error: `?` requires a Result value".to_string());
                };
                let variants = vec![success.as_ref().clone(), error.as_ref().clone()];
                let value = self.emit_expr(inner)?;
                let owned_result = self.take_owned_native_result(value);
                let result_struct = value.into_struct_value();
                let tag = self
                    .builder
                    .build_extract_value(result_struct, 0, "result.tag")
                    .map_err(|error| error.to_string())?
                    .into_int_value();
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: `?` outside a function")?;
                let success_block = self
                    .context
                    .append_basic_block(current_fn, "result.success");
                let error_block = self.context.append_basic_block(current_fn, "result.error");
                let is_success = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        tag,
                        self.context.i8_type().const_zero(),
                        "result.is_success",
                    )
                    .map_err(|error| error.to_string())?;
                self.builder
                    .build_conditional_branch(is_success, success_block, error_block)
                    .map_err(|error| error.to_string())?;
                self.builder.position_at_end(error_block);
                let borrowed = match inner.kind() {
                    ExprKind::Ident(name) => {
                        !(name.starts_with("await#") && self.async_frame_slot(name).is_some())
                    }
                    ExprKind::FieldAccess { .. }
                    | ExprKind::IndexAccess { .. }
                    | ExprKind::Topic => true,
                    _ => false,
                };
                let (propagated, expected) =
                    self.propagated_error_result(&source_ty, value, borrowed)?;
                if !self.async_results.is_empty()
                    || !self.async_finally_completion_scopes.is_empty()
                {
                    self.clear_consumed_async_await_value(inner, &source_ty)?;
                    self.emit_async_try_propagation(current_fn, propagated, &expected)?;
                    self.builder.position_at_end(success_block);
                    let slot =
                        self.native_value_slot(self.llvm_type(&source_ty), value, "result.try")?;
                    let success_value = self.try_success_value(&variants, slot, &success)?;
                    self.clear_consumed_async_await_value(inner, &source_ty)?;
                    let success_release = owned_result.clone().or_else(|| {
                        let ExprKind::Ident(name) = inner.kind() else {
                            return None;
                        };
                        (name.starts_with("await#")
                            && self.async_frame_slot(name).is_some()
                            && self.async_owned_value_supported(&success))
                        .then_some(ReleasePlan::Deep)
                    });
                    if let Some(plan) = success_release {
                        self.record_owned_native_result_with(
                            success_value,
                            success.as_ref().clone(),
                            plan,
                        );
                    }
                    return Ok(success_value);
                }
                self.emit_deferred_finalizers_since(
                    current_fn,
                    0,
                    ScopeExitReason::TryPropagation,
                )?;
                self.emit_pending_finally(current_fn, false, None)?;
                if self.current_block_terminated() {
                    self.builder.position_at_end(success_block);
                    let slot =
                        self.native_value_slot(self.llvm_type(&source_ty), value, "result.try")?;
                    let success_value = self.try_success_value(&variants, slot, &success)?;
                    if let Some(plan) = owned_result.clone() {
                        self.record_owned_native_result_with(
                            success_value,
                            success.as_ref().clone(),
                            plan,
                        );
                    }
                    return Ok(success_value);
                }
                self.emit_pending_validation_cleanups(None);
                self.emit_owned_closure_cleanups(None)?;
                self.builder
                    .build_return(Some(&propagated))
                    .map_err(|error| error.to_string())?;
                self.builder.position_at_end(success_block);
                let slot =
                    self.native_value_slot(self.llvm_type(&source_ty), value, "result.try")?;
                let success_value = self.try_success_value(&variants, slot, &success)?;
                if let Some(plan) = owned_result {
                    self.record_owned_native_result_with(
                        success_value,
                        success.as_ref().clone(),
                        plan,
                    );
                }
                Ok(success_value)
            }
            Expr {
                kind: ExprKind::CallValue { callee, args, .. },
                ..
            } => {
                let callee_ty = self.expr_type_hint(callee).ok_or_else(|| {
                    "error: cannot determine the type of the called expression".to_string()
                })?;
                let Type::Function { params, ret } = self.resolve_type(&callee_ty) else {
                    return Err(format!("error: `{callee_ty}` is not callable"));
                };
                let closure = self.emit_expr(callee)?.into_struct_value();
                let pointer = self
                    .builder
                    .build_extract_value(closure, 0, "call_value_function")
                    .unwrap()
                    .into_pointer_value();
                let environment = self
                    .builder
                    .build_extract_value(closure, 1, "call_value_environment")
                    .unwrap();
                let mut call_args = vec![environment.into()];
                let mut closure_temporaries: Vec<BasicValueEnum<'ctx>> = Vec::new();
                if matches!(
                    callee.kind(),
                    ExprKind::ArrowFn { .. }
                        | ExprKind::FnCall { .. }
                        | ExprKind::MethodCall { .. }
                        | ExprKind::CallValue { .. }
                ) {
                    closure_temporaries.push(closure.into());
                }
                for (arg, param) in args.iter().zip(params.iter()) {
                    let value = self.emit_expr_as(arg, param)?;
                    if self.is_temporary_closure_argument(arg, param) {
                        closure_temporaries.push(value);
                    }
                    call_args.push(value.into());
                }
                let call = self
                    .builder
                    .build_indirect_call(
                        self.llvm_closure_fn_type(&params, &ret),
                        pointer,
                        &call_args,
                        "call_value",
                    )
                    .unwrap();
                let result = call
                    .try_as_basic_value()
                    .basic()
                    .unwrap_or_else(|| self.context.i32_type().const_int(0, false).into());
                for temporary in closure_temporaries {
                    self.emit_closure_release(temporary)?;
                }
                Ok(result)
            }
            Expr {
                kind: ExprKind::IntLit(n),
                ..
            } => self.emit_expr_as(expr, &int_literal_type(*n)),
            Expr {
                kind: ExprKind::UIntLit(n),
                ..
            } => Ok(self.const_u128_value(*n).into()),
            Expr {
                kind: ExprKind::FloatLit(f),
                ..
            } => Ok(const_float_literal(self.context.f64_type(), f).into()),
            Expr {
                kind: ExprKind::DecimalLit(value),
                ..
            } => {
                let (coefficient, exponent) = codegen_decimal_literal(value.text())?;
                let ty = self.llvm_type(&Type::Decimal).into_struct_type();
                Ok(ty
                    .const_named_struct(&[
                        self.const_i128_value(coefficient).into(),
                        self.context
                            .i32_type()
                            .const_int(exponent as u64, true)
                            .into(),
                    ])
                    .into())
            }
            Expr {
                kind: ExprKind::BoolLit(b),
                ..
            } => Ok(self.context.bool_type().const_int(*b as u64, false).into()),
            Expr {
                kind: ExprKind::Null | ExprKind::None,
                ..
            } => {
                let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                Ok(ptr_ty.const_null().into())
            }
            Expr {
                kind: ExprKind::Typeof(inner),
                ..
            } => {
                if let ExprKind::Ident(name) = inner.as_ref().kind()
                    && let Some((ptr, declared_ty)) = self.load_var(name)
                    && let Type::Union(ref variants) =
                        transparent_codegen_type(&declared_ty).clone()
                {
                    if let Some(concrete) = self.load_concrete_type(name) {
                        let s = Codegen::type_string_for_variant(&concrete);
                        let global = self.const_string_abi(&s, "typeof").into_struct_value();
                        return Ok(global.into());
                    }
                    let fn_val = self
                        .builder
                        .get_insert_block()
                        .and_then(inkwell::basic_block::BasicBlock::get_parent)
                        .ok_or("internal: typeof outside function")?;
                    return self.emit_union_typeof(ptr, variants, fn_val);
                }
                let hint = self.expr_type_hint(inner);
                let type_str = if let Some(ty) = hint {
                    Self::type_string_for_variant(&ty)
                } else {
                    self.emit_expr(inner)?;
                    "unknown".to_string()
                };
                let global = self
                    .const_string_abi(&type_str, "typeof")
                    .into_struct_value();
                Ok(global.into())
            }
            Expr {
                kind: ExprKind::ObjectLit(_),
                ..
            } => Err("error: object literal requires an expected interface type".to_string()),
            Expr {
                kind: ExprKind::ObjectUpdate { base, fields },
                ..
            } => self.emit_object_update(base, fields),
            Expr {
                kind: ExprKind::ArrayLit(values),
                ..
            } => {
                let element = self.infer_array_literal_type(values).ok_or_else(|| {
                    "error: empty array requires an explicit element type".to_string()
                })?;
                self.emit_array_literal(values, &element)
            }
            Expr {
                kind: ExprKind::ArrayRepeat { value, count },
                ..
            } => {
                let element = self.expr_type_hint(value).ok_or_else(|| {
                    "error: cannot infer element type for array repeat".to_string()
                })?;
                self.emit_array_repeat_literal(value, count, &element)
            }
            Expr {
                kind: ExprKind::FieldAccess { object, field },
                ..
            } => self.emit_field_access_expr(expr, object, field),
            Expr {
                kind: ExprKind::IndexAccess { object, index },
                ..
            } => {
                let object_type = self
                    .expr_type_hint(object)
                    .ok_or_else(|| "error: cannot infer indexed value type".to_string())?;
                if let ExprKind::StrLit(field) = index.kind()
                    && let Some(index) = self.struct_field_index(&object_type, field)
                {
                    let value = self.emit_expr(object)?.into_struct_value();
                    return self
                        .builder
                        .build_extract_value(value, index, field)
                        .map_err(|error| error.to_string());
                }
                if let Type::Map(key_ty, value_ty) = transparent_codegen_type(&object_type) {
                    let key_ty = key_ty.as_ref().clone();
                    let value_ty = value_ty.as_ref().clone();
                    let result = self.emit_map_get(object, index, &key_ty)?;
                    self.record_owned_native_result_with(
                        result,
                        Type::Optional(Box::new(value_ty)),
                        crate::ReleasePlan::Deep,
                    );
                    return Ok(result);
                }
                if let Type::Tuple(elements) = transparent_codegen_type(&object_type) {
                    let Expr {
                        kind: ExprKind::IntLit(index),
                        ..
                    } = index.as_ref()
                    else {
                        return Err("error: tuple index must be an integer literal".to_string());
                    };
                    let index = usize::try_from(*index)
                        .map_err(|_| "error: tuple index out of bounds".to_string())?;
                    if index >= elements.len() {
                        return Err("error: tuple index out of bounds".to_string());
                    }
                    let value = self.emit_expr(object)?.into_struct_value();
                    return self
                        .builder
                        .build_extract_value(value, index as u32, "tuple_index")
                        .map_err(|error| error.to_string());
                }
                self.emit_array_index(object, index)
            }

            Expr {
                kind: ExprKind::StrLit(s),
                ..
            } => {
                let global = self.const_string_abi(s, "str").into_struct_value();
                Ok(global.into())
            }

            Expr {
                kind: ExprKind::RegexLit { pattern, flags },
                ..
            } => {
                let pattern = tyrus_core::validators::regex_runtime_pattern(pattern, flags)?;
                let global = self.const_string_abi(&pattern, "regex").into_struct_value();
                Ok(global.into())
            }

            Expr {
                kind: ExprKind::NewSet { ty, initial },
                ..
            } => {
                let element = ty
                    .clone()
                    .or_else(|| self.new_set_element_type(initial.as_deref()))
                    .ok_or_else(|| "error: cannot infer Set element type".to_string())?;
                let set = self.emit_new_set(&element, initial.as_deref())?;
                self.record_owned_native_result(set, Type::Set(Box::new(element)));
                Ok(set)
            }
            Expr {
                kind:
                    ExprKind::NewMap {
                        key,
                        value,
                        initial,
                    },
                ..
            } => {
                let (key, value) = match (key, value) {
                    (Some(key), Some(value)) => (key.clone(), value.clone()),
                    _ => self
                        .new_map_entry_types(initial.as_deref())
                        .ok_or_else(|| "error: cannot infer Map types".to_string())?,
                };
                let map = self.emit_new_map(&key, &value, initial.as_deref())?;
                self.record_owned_native_result(map, Type::Map(Box::new(key), Box::new(value)));
                Ok(map)
            }

            Expr {
                kind: ExprKind::TemplateLit(parts),
                ..
            } => self.emit_template_literal(parts),

            Expr {
                kind: ExprKind::Ident(name),
                ..
            } => {
                let Some((ptr, ty)) = self.load_var(name) else {
                    if let Some(result) = self.materialize_function_value(name) {
                        return result;
                    }
                    if let Some(result) = self.emit_native_constant(name) {
                        return Ok(result);
                    }
                    if let Some(ty) = match name.as_str() {
                        "digest" => Some(Type::Named("DigestNamespace".to_string())),
                        "hmac" => Some(Type::Named("HmacNamespace".to_string())),
                        "cipher" => Some(Type::Named("CipherNamespace".to_string())),
                        "rand" => Some(Type::Named("RandNamespace".to_string())),
                        "argon2" => Some(Type::Named("Argon2Namespace".to_string())),
                        "jwt" => Some(Type::Named("JwtNamespace".to_string())),
                        _ => None,
                    } {
                        let llvm_ty = self.llvm_type(&ty);
                        let global =
                            self.module
                                .add_global(llvm_ty, None, &format!("__tyrus_ns_{name}"));
                        global.set_initializer(&llvm_ty.const_zero());
                        self.store_var(name, global.as_pointer_value(), ty);
                        let val = self
                            .builder
                            .build_load(llvm_ty, global.as_pointer_value(), name)
                            .unwrap();
                        return Ok(val);
                    }
                    return Err(format!("internal: `{name}` not in codegen scope"));
                };
                let resolved_ty = transparent_codegen_type(&ty);
                if let Type::Optional(inner) = &ty
                    && let Some(concrete) = self.load_concrete_type(name)
                    && transparent_codegen_type(&concrete) == transparent_codegen_type(inner)
                {
                    let optional_ptr = self
                        .builder
                        .build_load(self.llvm_type(&ty), ptr, name)
                        .unwrap()
                        .into_pointer_value();
                    let value = self
                        .builder
                        .build_load(self.llvm_type(inner), optional_ptr, name)
                        .unwrap();
                    return Ok(value);
                }
                if let Some(value) = self.load_narrowed_variant(name, ptr, resolved_ty) {
                    return value;
                }
                let llvm_ty = self.llvm_type(&ty);
                let val = self.builder.build_load(llvm_ty, ptr, name).unwrap();
                Ok(val)
            }

            Expr {
                kind: ExprKind::BinOp { op, left, right },
                ..
            } => self.emit_binop(op, left, right),
            Expr {
                kind: ExprKind::ChainedComparison { terms, ops },
                ..
            } => self.emit_chained_comparison(terms, ops),
            Expr {
                kind: ExprKind::Pipeline { left, right },
                ..
            } => {
                if tyrus_core::ast::contains_outer_topic(right) {
                    let value = self.emit_expr(left)?;
                    let ty = self.expr_type_hint(left).unwrap_or(Type::Unknown);
                    self.topic_values.push((value, ty));
                    let result = self.emit_expr(right);
                    self.topic_values.pop();
                    result
                } else {
                    let call = tyrus_core::ast::desugar_pipeline(left, right).ok_or_else(|| {
                        "error: the right side of `|>` must be a function or contain `%`"
                            .to_string()
                    })?;
                    self.emit_expr(&call)
                }
            }
            Expr {
                kind: ExprKind::Topic,
                ..
            } => self
                .topic_values
                .last()
                .map(|(value, _)| *value)
                .ok_or_else(|| "error: `%` can only be used on the right side of `|>`".to_string()),

            Expr {
                kind: ExprKind::Not(inner),
                ..
            } => {
                let bool_val = self.emit_condition_value(inner)?;
                let result = self.builder.build_not(bool_val, "not").unwrap();
                Ok(result.into())
            }

            Expr {
                kind: ExprKind::Neg(inner),
                ..
            } => {
                let val = self.emit_expr(inner)?;
                if val.is_int_value() {
                    let int_val = val.into_int_value();
                    let ty = int_val.get_type();
                    let zero = ty.const_int(0, false);
                    Ok(self
                        .builder
                        .build_int_sub(zero, int_val, "neg")
                        .unwrap()
                        .into())
                } else if val.is_float_value() {
                    let float_val = val.into_float_value();
                    let ty = float_val.get_type();
                    let neg_zero = ty.const_float(-0.0);
                    Ok(self
                        .builder
                        .build_float_sub(neg_zero, float_val, "neg")
                        .unwrap()
                        .into())
                } else {
                    Err("unary `-` requires a numeric value".to_string())
                }
            }

            Expr {
                kind: ExprKind::BitNot(inner),
                ..
            } => {
                let val = self.emit_expr(inner)?;
                if val.is_int_value() {
                    let int_val = val.into_int_value();
                    let ty = int_val.get_type();
                    let all_ones = ty.const_int(!0, false);
                    Ok(self
                        .builder
                        .build_xor(int_val, all_ones, "bitnot")
                        .unwrap()
                        .into())
                } else {
                    Err("unary `~` requires an integer value".to_string())
                }
            }

            Expr {
                kind:
                    ExprKind::Borrow {
                        place: Some(place), ..
                    },
                ..
            } => self.emit_expr(place),
            Expr {
                kind: ExprKind::Borrow { name, .. },
                ..
            } => {
                let (ptr, ty) = self
                    .load_var(name)
                    .ok_or_else(|| format!("internal: `{name}` not in codegen scope"))?;
                let resolved_ty = transparent_codegen_type(&ty);
                if let Some(value) = self.load_narrowed_variant(name, ptr, resolved_ty) {
                    return value;
                }
                let llvm_ty = self.llvm_type(&ty);
                let val = self.builder.build_load(llvm_ty, ptr, name).unwrap();
                Ok(val)
            }

            Expr {
                kind: ExprKind::Await(_),
                span,
            } => {
                let (slot, ty) = self
                    .async_await_values
                    .last()
                    .and_then(|values| values.get(span))
                    .cloned()
                    .ok_or_else(|| {
                        format!(
                            "await codegen is not implemented for continuation value at {}..{} in block {:?}",
                            span.start,
                            span.end,
                            self.builder
                                .get_insert_block()
                                .and_then(|block| block.get_name().to_str().ok().map(str::to_string))
                        )
                    })?;
                self.builder
                    .build_load(self.llvm_type(&ty), slot, "async.await.value")
                    .map_err(|error| error.to_string())
            }

            Expr {
                kind: ExprKind::Unwrap(inner),
                ..
            } => {
                let Type::Optional(value_type) = self
                    .expr_type_hint(inner)
                    .ok_or("error: cannot infer optional value type")?
                else {
                    return Err("error: cannot unwrap a non-optional value".to_string());
                };
                let pointer = self.emit_expr(inner)?.into_pointer_value();
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .ok_or("internal: unwrap expression outside function")?;
                let failure = self.context.append_basic_block(function, "unwrap.failure");
                let present = self.context.append_basic_block(function, "unwrap.present");
                let is_null = self
                    .builder
                    .build_is_null(pointer, "unwrap.is_null")
                    .unwrap();
                self.builder
                    .build_conditional_branch(is_null, failure, present)
                    .unwrap();

                self.builder.position_at_end(failure);
                self.emit_runtime_panic("tyrus runtime error: attempted to unwrap null");

                self.builder.position_at_end(present);
                self.builder
                    .build_load(self.llvm_type(&value_type), pointer, "optional_unwrap")
                    .map_err(|error| error.to_string())
            }

            Expr {
                kind:
                    ExprKind::FnCall {
                        callee,
                        type_args,
                        args,
                    },
                ..
            } => self.emit_fn_call_expr(expr, callee, type_args, args),

            Expr {
                kind:
                    ExprKind::MethodCall {
                        object,
                        method,
                        type_args,
                        args,
                        ..
                    },
                ..
            } => {
                if let ExprKind::Ident(type_name) = object.as_ref().kind()
                    && let Some(enum_info) = self.enums.get(type_name).cloned()
                    && let Some((_, _)) = enum_info.members.iter().find(|(m, _)| m == method)
                {
                    let variant_index = enum_info
                        .members
                        .iter()
                        .position(|(m, _)| m == method)
                        .unwrap();
                    return self.emit_enum_variant_constructor(
                        type_name,
                        method,
                        variant_index,
                        &enum_info,
                        args,
                    );
                }
                if let Some(result) = self.emit_test_method_call(object, method, args)? {
                    return Ok(result);
                }
                if let Some(result) = self.emit_array_static_method(object, method, args) {
                    return result;
                }
                if let Some(result) =
                    self.emit_promise_static_method(object, method, type_args, args)
                {
                    return result;
                }
                if let Some(result) = self.emit_algorithm_static_method(object, method, args) {
                    return result;
                }
                self.emit_method_call(object, method, type_args, args)
            }
            Expr {
                kind:
                    ExprKind::OptionalMethodCall {
                        object,
                        method,
                        args,
                        ..
                    },
                ..
            } => self.emit_optional_method_call(object, method, args),
            Expr {
                kind: ExprKind::OptionalFieldAccess { object, field },
                ..
            } => self.emit_optional_field_access(object, field),

            Expr {
                kind: ExprKind::PrefixIncr(inner),
                ..
            } => self.emit_incr_decr(inner, false, false),
            Expr {
                kind: ExprKind::PrefixDecr(inner),
                ..
            } => self.emit_incr_decr(inner, true, false),
            Expr {
                kind: ExprKind::PostfixIncr(inner),
                ..
            } => self.emit_incr_decr(inner, false, true),
            Expr {
                kind: ExprKind::PostfixDecr(inner),
                ..
            } => self.emit_incr_decr(inner, true, true),

            Expr {
                kind:
                    ExprKind::Ternary {
                        cond,
                        then_expr,
                        else_expr,
                    },
                ..
            } => self.emit_ternary(cond, then_expr, else_expr),

            Expr {
                kind:
                    ExprKind::If {
                        cond,
                        then_block,
                        else_block,
                    },
                ..
            } => self.emit_if_expr(cond, then_block, else_block.as_deref()),

            Expr {
                kind: ExprKind::Loop { body },
                ..
            } => self.emit_loop_expr(expr, body),

            Expr {
                kind:
                    ExprKind::ArrowFn {
                        is_async,
                        params,
                        ret_type,
                        body,
                        ..
                    },
                ..
            } => {
                if *is_async {
                    self.emit_async_arrow_fn(params, ret_type, body)
                } else {
                    let confined = std::mem::take(&mut self.confined_closure_pending);
                    self.emit_arrow_fn(params, ret_type, body, !confined)
                }
            }
            Expr {
                kind: ExprKind::TypeAssert { value, ty },
                ..
            } => {
                let source = self.expr_type_hint(value);
                let resolved = self.resolve_type(ty);
                if matches!(
                    value.kind(),
                    ExprKind::ArrayLit(_) | ExprKind::ArrayRepeat { .. }
                ) && matches!(transparent_codegen_type(&resolved), Type::Array(_))
                {
                    return self.emit_expr_as(value, &resolved);
                }
                if matches!(transparent_codegen_type(&resolved), Type::F128)
                    && let ExprKind::FloatLit(literal) = value.kind()
                {
                    return Ok(const_float_literal(self.context.f128_type(), literal).into());
                }
                if let Type::Optional(inner) = transparent_codegen_type(&resolved) {
                    let inner = inner.as_ref().clone();
                    if let Some(cast) =
                        self.emit_fallible_numeric_cast(value, source.as_ref(), &inner)
                    {
                        let cast = cast?;
                        self.record_owned_native_result_with(
                            cast,
                            Type::Optional(Box::new(inner)),
                            crate::ReleasePlan::Value,
                        );
                        return Ok(cast);
                    }
                }
                let val = self.emit_expr(value)?;
                self.coerce_value_with_source(value, val, source.as_ref(), ty)
            }
            Expr {
                kind: ExprKind::ConstAssert(inner),
                ..
            } => {
                if let ExprKind::ArrayLit(values) = inner.kind()
                    && let Type::Tuple(elements) = transparent_codegen_type(
                        &tyrus_core::type_checker::const_assertion_type(inner),
                    )
                    .clone()
                {
                    return self.emit_tuple_literal(values, &elements);
                }
                self.emit_expr(inner)
            }
            Expr {
                kind: ExprKind::Spread(inner),
                ..
            } => self.emit_expr(inner),

            Expr {
                kind: ExprKind::In { key, object },
                ..
            } => self.emit_in(key, object),

            Expr {
                kind: ExprKind::Is { expr, ty },
                ..
            } => self.emit_is(expr, ty),

            Expr {
                kind:
                    ExprKind::Range {
                        start,
                        end,
                        inclusive,
                    },
                ..
            } => self.emit_range_literal(start, end, *inclusive),
        }
    }

    fn emit_in(&mut self, key: &Expr, object: &Expr) -> Result<BasicValueEnum<'ctx>, String> {
        let object_ty = self.expr_type_hint(object).map(|t| self.resolve_type(&t));
        match object_ty.as_ref().map(transparent_codegen_type) {
            Some(Type::Range(element)) => {
                let element = element.as_ref().clone();
                let range_value = self.emit_expr(object)?.into_struct_value();
                let needle = self.emit_expr_as(key, &element)?;
                self.emit_range_contains(range_value, needle, &element)
            }
            Some(Type::Array(_) | Type::Tuple(_)) => {
                self.emit_array_method(object, "includes", std::slice::from_ref(key))
            }
            Some(Type::Map(map_key, map_value)) => {
                let map_key = map_key.as_ref().clone();
                let map_value = map_value.as_ref().clone();
                self.emit_map_method(
                    object,
                    &map_key,
                    &map_value,
                    "has",
                    std::slice::from_ref(key),
                )
            }
            Some(Type::Set(element)) => {
                let element = element.as_ref().clone();
                self.emit_set_method(object, &element, "has", std::slice::from_ref(key))
            }
            Some(Type::UnionFind(element)) => {
                let element = element.as_ref().clone();
                self.emit_set_method(object, &element, "has", std::slice::from_ref(key))
            }
            _ => self.emit_in_object_field_check(key, object),
        }
    }

    fn emit_is(&mut self, expr: &Expr, ty: &Type) -> Result<BasicValueEnum<'ctx>, String> {
        if let ExprKind::Ident(name) = expr.kind()
            && let Some(answer) = self.emit_error_subtype_is(name, ty)?
        {
            return Ok(answer);
        }
        let expr_ty = self.expr_type_hint(expr);
        let resolved = expr_ty.as_ref().map(|t| self.resolve_type(t));
        let bool_ty = self.context.bool_type();

        match resolved.as_ref().map(transparent_codegen_type) {
            Some(Type::Union(variants)) => {
                let target_idx = variants.iter().position(|v| self.types_assignable(v, ty));
                match target_idx {
                    Some(idx) => {
                        let emitted = self.emit_expr(expr)?;
                        let union_ptr = if emitted.is_pointer_value() {
                            emitted.into_pointer_value()
                        } else {
                            let slot = self
                                .builder
                                .build_alloca(emitted.get_type(), "is.union")
                                .map_err(|e| format!("is slot: {e}"))?;
                            self.builder
                                .build_store(slot, emitted)
                                .map_err(|e| format!("is store: {e}"))?;
                            slot
                        };
                        let tag = self.load_union_tag(variants, union_ptr)?;
                        let cmp = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                tag,
                                self.context.i8_type().const_int(idx as u64, false),
                                "is.cmp",
                            )
                            .map_err(|e| format!("is cmp: {e}"))?;
                        Ok(cmp.into())
                    }
                    None => Ok(bool_ty.const_int(0, false).into()),
                }
            }
            Some(_) => {
                let compatible = self.types_assignable(&resolved.unwrap(), ty);
                Ok(bool_ty.const_int(compatible as u64, false).into())
            }
            None => {
                self.emit_expr(expr)?;
                Ok(bool_ty.const_int(1, false).into())
            }
        }
    }

    pub(super) fn types_assignable(&self, candidate: &Type, target: &Type) -> bool {
        match (
            transparent_codegen_type(candidate),
            transparent_codegen_type(target),
        ) {
            (Type::Union(variants), _) => variants.iter().any(|v| self.types_assignable(v, target)),
            (_, Type::Union(variants)) => {
                variants.iter().any(|v| self.types_assignable(candidate, v))
            }
            (a, b) => a == b,
        }
    }

    fn emit_in_object_field_check(
        &mut self,
        key: &Expr,
        object: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let obj_ty = self.expr_type_hint(object).map(|t| self.resolve_type(&t));
        let field_names: Vec<String> = match &obj_ty {
            Some(Type::Named(n)) => self
                .interfaces
                .get(n.as_str())
                .map(|f| f.iter().map(|field| field.name.clone()).collect())
                .unwrap_or_default(),
            Some(Type::Object(fields)) => fields.iter().map(|f| f.name.clone()).collect(),
            _ => vec![],
        };

        let i1_ty = self.context.bool_type();
        let key_val = self.emit_expr(key)?.into_struct_value();
        let mut result = i1_ty.const_zero();
        for field in &field_names {
            let field_ptr = self.const_string_abi(field, "in.field").into_struct_value();
            let is_eq = self.emit_string_equals(key_val, field_ptr, "in.eq")?;
            result = self.builder.build_or(result, is_eq, "in.acc").unwrap();
        }
        Ok(result.into())
    }

    fn emit_range_literal(
        &mut self,
        start: &Expr,
        end: &Expr,
        inclusive: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let element = self
            .expr_type_hint(start)
            .or_else(|| self.expr_type_hint(end))
            .ok_or_else(|| "error: cannot infer the element type of a range".to_string())?;
        let start_val = self.emit_expr_as(start, &element)?;
        let end_val = self.emit_expr_as(end, &element)?;
        let struct_ty = self
            .llvm_type(&Type::Range(Box::new(element)))
            .into_struct_type();
        let value = struct_ty.get_undef();
        let value = self
            .builder
            .build_insert_value(value, start_val, 0, "range.start")
            .unwrap()
            .into_struct_value();
        let value = self
            .builder
            .build_insert_value(value, end_val, 1, "range.end")
            .unwrap()
            .into_struct_value();
        let inclusive_val = self.context.bool_type().const_int(inclusive as u64, false);
        let value = self
            .builder
            .build_insert_value(value, inclusive_val, 2, "range.inclusive")
            .unwrap()
            .into_struct_value();
        Ok(value.into())
    }

    pub(super) fn emit_range_contains(
        &mut self,
        range_value: inkwell::values::StructValue<'ctx>,
        needle: BasicValueEnum<'ctx>,
        element: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let start = self
            .builder
            .build_extract_value(range_value, 0, "range.start")
            .unwrap();
        let end = self
            .builder
            .build_extract_value(range_value, 1, "range.end")
            .unwrap();
        let inclusive = self
            .builder
            .build_extract_value(range_value, 2, "range.inclusive")
            .unwrap()
            .into_int_value();
        let unsigned = is_unsigned_int_type(transparent_codegen_type(element));
        let (ge_int, le_int, lt_int) = if unsigned {
            (IntPredicate::UGE, IntPredicate::ULE, IntPredicate::ULT)
        } else {
            (IntPredicate::SGE, IntPredicate::SLE, IntPredicate::SLT)
        };
        let ge = self
            .emit_comparison(
                needle,
                start,
                ge_int,
                inkwell::FloatPredicate::OGE,
                "range.ge",
            )?
            .into_int_value();
        let le = self
            .emit_comparison(
                needle,
                end,
                le_int,
                inkwell::FloatPredicate::OLE,
                "range.le",
            )?
            .into_int_value();
        let lt = self
            .emit_comparison(
                needle,
                end,
                lt_int,
                inkwell::FloatPredicate::OLT,
                "range.lt",
            )?
            .into_int_value();
        let end_ok = self
            .builder
            .build_select(inclusive, le, lt, "range.end_ok")
            .unwrap()
            .into_int_value();
        Ok(self
            .builder
            .build_and(ge, end_ok, "range.contains")
            .unwrap()
            .into())
    }

    fn emit_object_update(
        &mut self,
        base: &Expr,
        values: &[(String, Expr)],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let object_type = self
            .expr_type_hint(base)
            .ok_or_else(|| "error: cannot infer object type for object update".to_string())?;
        let Type::Named(name) = transparent_codegen_type(&object_type) else {
            return Err("error: object update requires an interface value".to_string());
        };
        let fields = self
            .interfaces
            .get(name)
            .cloned()
            .ok_or_else(|| format!("error: unknown interface `{name}`"))?;
        let mut result = self.emit_expr(base)?.into_struct_value();
        let mut field_plans: Vec<(usize, crate::ReleasePlan)> = Vec::new();
        for (field_name, expression) in values {
            let index = fields
                .iter()
                .position(|candidate| candidate.name == *field_name)
                .ok_or_else(|| format!("error: interface `{name}` has no field `{field_name}`"))?;
            let storage_type = field_storage_type(&fields[index]);
            self.validation_target_path.push(field_name.clone());
            let emitted = self.emit_expr_as(expression, &storage_type);
            self.validation_target_path.pop();
            let value = emitted?;
            // A `T?` field coerced from a bare value here passes through
            // `emit_optional_box`, which never registers its own release,
            // see the identical gate in `emit_object_literal_fields` for the
            // full reasoning. Registering it here, then immediately taking
            // it back via `take_owned_native_result` below (exactly like
            // that function), keeps the claim self-contained: the pending
            // entry never survives past this field, so it can never also
            // be picked up a second time by the caller's own "does this
            // value have a nested pending result" fallback (`VarDecl`'s
            // `has_nested_owned_native_results`), which is what happened
            // when this used to register the box but leave it dangling for
            // that fallback to find: both it and this function's own
            // discard freed the same pointer.
            let boxed_here = matches!(transparent_codegen_type(&storage_type), Type::Optional(_))
                && !matches!(expression.kind(), ExprKind::Null | ExprKind::None)
                && !self
                    .expr_type_hint(expression)
                    .map(|hint| self.resolve_type(&hint))
                    .is_some_and(|hint| {
                        matches!(transparent_codegen_type(&hint), Type::Optional(_))
                    })
                && self.pending_owned_native_results.is_empty();
            if boxed_here {
                self.record_owned_native_result_with(
                    value,
                    storage_type.clone(),
                    crate::ReleasePlan::Value,
                );
            }
            if let Some(plan) = self.take_owned_native_result(value) {
                field_plans.push((index, plan));
            }
            result = self
                .builder
                .build_insert_value(result, value, index as u32, field_name)
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        let value: BasicValueEnum<'ctx> = result.into();
        if !field_plans.is_empty() {
            self.record_owned_native_result_with(
                value,
                object_type.clone(),
                crate::ReleasePlan::Fields(field_plans),
            );
        }
        self.emit_named_object_validator_call(name, value)?;
        Ok(value)
    }

    fn emit_numeric_constant(&self, type_name: &str, field: &str) -> Option<BasicValueEnum<'ctx>> {
        match (type_name, field) {
            ("i8", "MIN") => Some(
                self.context
                    .i8_type()
                    .const_int(i8::MIN as u64, true)
                    .into(),
            ),
            ("i8", "MAX") => Some(
                self.context
                    .i8_type()
                    .const_int(i8::MAX as u64, true)
                    .into(),
            ),
            ("i16", "MIN") => Some(
                self.context
                    .i16_type()
                    .const_int(i16::MIN as u64, true)
                    .into(),
            ),
            ("i16", "MAX") => Some(
                self.context
                    .i16_type()
                    .const_int(i16::MAX as u64, true)
                    .into(),
            ),
            ("u8", "MIN") => Some(self.context.i8_type().const_zero().into()),
            ("u8", "MAX") => Some(self.context.i8_type().const_all_ones().into()),
            ("u16", "MIN") => Some(self.context.i16_type().const_zero().into()),
            ("u16", "MAX") => Some(self.context.i16_type().const_all_ones().into()),
            ("i32", "MIN") => Some(
                self.context
                    .i32_type()
                    .const_int(i32::MIN as u64, true)
                    .into(),
            ),
            ("i32", "MAX") => Some(
                self.context
                    .i32_type()
                    .const_int(i32::MAX as u64, true)
                    .into(),
            ),
            ("i64", "MIN") => Some(
                self.context
                    .i64_type()
                    .const_int(i64::MIN as u64, true)
                    .into(),
            ),
            ("i64", "MAX") => Some(
                self.context
                    .i64_type()
                    .const_int(i64::MAX as u64, true)
                    .into(),
            ),
            ("i128", "MIN") => Some(
                self.context
                    .i128_type()
                    .const_int_arbitrary_precision(&[0, 0x8000_0000_0000_0000])
                    .into(),
            ),
            ("i128", "MAX") => Some(
                self.context
                    .i128_type()
                    .const_int_arbitrary_precision(&[u64::MAX, 0x7fff_ffff_ffff_ffff])
                    .into(),
            ),
            ("u32", "MIN") => Some(self.context.i32_type().const_zero().into()),
            ("u32", "MAX") => Some(self.context.i32_type().const_all_ones().into()),
            ("u64", "MIN") => Some(self.context.i64_type().const_zero().into()),
            ("u64", "MAX") => Some(self.context.i64_type().const_all_ones().into()),
            ("u128", "MIN") => Some(self.context.i128_type().const_zero().into()),
            ("u128", "MAX") => Some(self.context.i128_type().const_all_ones().into()),
            ("f32", "MIN") => Some(self.context.f32_type().const_float(f32::MIN as f64).into()),
            ("f32", "MAX") => Some(self.context.f32_type().const_float(f32::MAX as f64).into()),
            ("f64", "MIN") => Some(self.context.f64_type().const_float(f64::MIN).into()),
            ("f64", "MAX") => Some(self.context.f64_type().const_float(f64::MAX).into()),
            ("f128", "MIN") => Some(
                unsafe {
                    self.context
                        .f128_type()
                        .const_float_from_string("-1.18973149535723176508575932662800702e4932")
                }
                .into(),
            ),
            ("f128", "MAX") => Some(
                unsafe {
                    self.context
                        .f128_type()
                        .const_float_from_string("1.18973149535723176508575932662800702e4932")
                }
                .into(),
            ),
            ("decimal", "MIN" | "MAX") => {
                let coefficient = if field == "MIN" {
                    -Decimal::MAX_COEFFICIENT
                } else {
                    Decimal::MAX_COEFFICIENT
                };
                Some(
                    self.llvm_type(&Type::Decimal)
                        .into_struct_type()
                        .const_named_struct(&[
                            self.const_i128_value(coefficient).into(),
                            self.context
                                .i32_type()
                                .const_int((Decimal::MAX_ADJUSTED_EXPONENT - 33) as u64, true)
                                .into(),
                        ])
                        .into(),
                )
            }
            _ => None,
        }
    }

    /// Moves `value` into the heap storage an optional payload lives in. An
    /// optional is a nullable pointer to a boxed payload, so a present value is
    /// always behind one allocation.
    /// Wraps a native "pointer, or null when absent" result in the ordinary
    /// `T?` box.
    ///
    /// `tyrus_mutex_try_lock` and the two `RwLock` try-methods hand back the
    /// acquired guard's own pointer, or null. That was described at those call
    /// sites as "exactly the `Optional<Guard>` representation already", and it
    /// is not one: a `T?` here is a heap box holding the payload, that is what
    /// `emit_optional_box` builds and what every consumer of an optional
    /// dereferences. There is no null-pointer optimization for this shape.
    ///
    /// Handing a bare guard pointer to `emit_async_clone_value_into`'s optional
    /// arm therefore made it read the *first field* of the guard struct, which
    /// is the `RwLock` pointer, and store that as if it were the guard. The
    /// frame's drop glue then unlocked a `TyrusRwLock` as a
    /// `TyrusRwLockWriteGuard`, dropping a permit that was never there:
    /// `fetch_sub` on `0x2`, a segfault from a nine-line program, on every run.
    ///
    /// The absent case stays a plain null rather than a box holding null, which
    /// is what every presence test in the compiler already checks for.
    fn emit_optional_pointer_box(
        &mut self,
        value: BasicValueEnum<'ctx>,
        inner: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: optional pointer box outside a function")?;
        let present_block = self
            .context
            .append_basic_block(function, "optional.handle.present");
        let empty_block = self
            .context
            .append_basic_block(function, "optional.handle.empty");
        let merge_block = self
            .context
            .append_basic_block(function, "optional.handle.merge");
        let present = self
            .builder
            .build_is_not_null(value.into_pointer_value(), "optional.handle.is_present")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_conditional_branch(present, present_block, empty_block)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(present_block);
        let boxed = self.emit_optional_box(value, inner)?;
        let present_end = self.builder.get_insert_block().unwrap();
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(empty_block);
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(merge_block);
        let result = self
            .builder
            .build_phi(pointer, "optional.handle")
            .map_err(|error| error.to_string())?;
        result.add_incoming(&[(&boxed, present_end), (&pointer.const_null(), empty_block)]);
        let result = result.as_basic_value();
        self.record_owned_native_result_with(
            result,
            Type::Optional(Box::new(inner.clone())),
            crate::ReleasePlan::Value,
        );
        Ok(result)
    }

    fn emit_narrowed_field_payload(
        &mut self,
        expr: &Expr,
        object_type: &Type,
        field: &str,
        extracted: BasicValueEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let Some(path) = super::statements::codegen_expression_path(expr) else {
            return Ok(extracted);
        };
        if !self.path_is_narrowed(&path) {
            return Ok(extracted);
        }
        let Some(Type::Optional(inner)) = self.field_type_on(object_type, field) else {
            return Ok(extracted);
        };
        if !extracted.is_pointer_value() {
            return Ok(extracted);
        }
        self.builder
            .build_load(
                self.llvm_type(&inner),
                extracted.into_pointer_value(),
                "narrowed.field.payload",
            )
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_optional_box(
        &mut self,
        value: BasicValueEnum<'ctx>,
        inner: &Type,
    ) -> Result<PointerValue<'ctx>, String> {
        let size = self
            .llvm_type(inner)
            .size_of()
            .ok_or_else(|| format!("error: optional type `{inner}` has no native size"))?;
        let (malloc, size) = self.declare_malloc(size);
        let storage = self
            .builder
            .build_call(malloc, &[size.into()], "optional_storage")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        self.builder.build_store(storage, value).unwrap();
        // Ownership is deliberately *not* registered here, even though the
        // header is exactly as freeable as `OptionalFieldAccess`'s (`Value`
        // would be the right plan). This helper is shared across call sites
        // with different control flow around them, and at least two of them,
        // `emit_checked_payload_cast`'s numeric-range branch, and whatever
        // built `&&=`/`||=`/`??=`'s short-circuit assignment value, use this
        // box's result *inside* a conditional branch that a caller further
        // merges into an outer phi. Registering the pre-merge value here
        // reproduced the original Phase 3 blocker: a release recorded before a
        // merge does not dominate a later free of it. Two attempts at a
        // supposedly-safe single call site each aborted a different IR
        // snapshot test with "Instruction does not dominate all uses" before
        // this was reverted to leaving the box unregistered, the flat, non-
        // scaling leak this leaves is `emit_optional_box`'s own gap, still
        // open, tracked in `tests/leaks/known/optional-chain.tyr`.
        Ok(storage)
    }

    fn const_i128(&self, value: i128) -> IntValue<'ctx> {
        let bits = value as u128;
        let low = (bits & u128::from(u64::MAX)) as u64;
        let high = (bits >> 64) as u64;
        self.context
            .i128_type()
            .const_int_arbitrary_precision(&[low, high])
    }

    fn emit_char_fits(&self, value: IntValue<'ctx>, source: &Type) -> IntValue<'ctx> {
        let i64_type = self.context.i64_type();
        let widened = if is_unsigned_int_type(source) {
            self.builder
                .build_int_z_extend(value, i64_type, "char.widened")
                .unwrap()
        } else {
            self.builder
                .build_int_s_extend(value, i64_type, "char.widened")
                .unwrap()
        };
        let at_least = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                widened,
                i64_type.const_int(0, false),
                "char.min",
            )
            .unwrap();
        let at_most = self
            .builder
            .build_int_compare(
                IntPredicate::SLE,
                widened,
                i64_type.const_int(0x10_FFFF, false),
                "char.max",
            )
            .unwrap();
        let above_surrogates = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                widened,
                i64_type.const_int(0xD800, false),
                "char.surrogate.low",
            )
            .unwrap();
        let below_surrogates = self
            .builder
            .build_int_compare(
                IntPredicate::SLE,
                widened,
                i64_type.const_int(0xDFFF, false),
                "char.surrogate.high",
            )
            .unwrap();
        let is_surrogate = self
            .builder
            .build_and(above_surrogates, below_surrogates, "char.surrogate")
            .unwrap();
        let not_surrogate = self
            .builder
            .build_not(is_surrogate, "char.not_surrogate")
            .unwrap();
        let in_range = self
            .builder
            .build_and(at_least, at_most, "char.in_range")
            .unwrap();
        self.builder
            .build_and(in_range, not_surrogate, "char.fits")
            .unwrap()
    }

    fn emit_integer_to_decimal(
        &mut self,
        value: IntValue<'ctx>,
        source: &Type,
    ) -> BasicValueEnum<'ctx> {
        const MAX_COEFFICIENT: i128 = 9_999_999_999_999_999_999_999_999_999_999_999;
        self.emit_integer_cast_range_check(value, Some(source), &Type::I128);
        let coefficient = self
            .numeric_cast_convert(value.into(), source, &Type::I128)
            .expect("integer-to-decimal conversion must lower to i128")
            .into_int_value();
        let max = self.const_i128_value(MAX_COEFFICIENT);
        let min = self.const_i128_value(-MAX_COEFFICIENT);
        let below = self
            .builder
            .build_int_compare(IntPredicate::SLT, coefficient, min, "decimal.cast.below")
            .unwrap();
        let above = self
            .builder
            .build_int_compare(IntPredicate::SGT, coefficient, max, "decimal.cast.above")
            .unwrap();
        let out_of_range = self
            .builder
            .build_or(below, above, "decimal.cast.range")
            .unwrap();
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .expect("internal: decimal cast outside function");
        let error_block = self
            .context
            .append_basic_block(function, "decimal.cast.error");
        let ok_block = self.context.append_basic_block(function, "decimal.cast.ok");
        self.builder
            .build_conditional_branch(out_of_range, error_block, ok_block)
            .unwrap();
        self.builder.position_at_end(error_block);
        self.emit_runtime_panic("tyrus runtime error: value out of range for `decimal`");
        self.builder.position_at_end(ok_block);
        let decimal = self.llvm_type(&Type::Decimal).into_struct_type();
        let value = self
            .builder
            .build_insert_value(
                decimal.get_undef(),
                coefficient,
                0,
                "decimal.cast.coefficient",
            )
            .unwrap();
        self.builder
            .build_insert_value(
                value.into_struct_value(),
                self.context.i32_type().const_zero(),
                1,
                "decimal.cast.exponent",
            )
            .unwrap()
            .into_struct_value()
            .into()
    }

    fn emit_decimal_to_integer(
        &mut self,
        value: StructValue<'ctx>,
        target: &Type,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let decimal = self.llvm_type(&Type::Decimal);
        let slot = self
            .builder
            .build_alloca(decimal, "decimal.to_int.value")
            .unwrap();
        self.builder.build_store(slot, value).unwrap();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let convert = self.declare_libc_fn(
            "tyrus_decimal_to_i128",
            &[pointer.into()],
            Some(self.context.i128_type().into()),
        );
        let integer = self
            .builder
            .build_call(convert, &[slot.into()], "decimal.to_int")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap();
        self.coerce_value_with_source(
            &Expr::synthetic(ExprKind::Null),
            integer,
            Some(&Type::I128),
            target,
        )
    }

    pub(super) fn emit_template_value_string(
        &mut self,
        value: BasicValueEnum<'ctx>,
        hint: Option<&Type>,
    ) -> Result<StructValue<'ctx>, String> {
        if value.is_struct_value() && value.into_struct_value().get_type() == self.string_type() {
            return Ok(value.into_struct_value());
        }
        if value.is_float_value() {
            return Ok(self.emit_format_f64(value.into_float_value()));
        }
        if value.is_int_value() {
            let integer = value.into_int_value();
            if matches!(hint.map(transparent_codegen_type), Some(Type::Char)) {
                return Ok(self.emit_format_char(integer));
            }
            let is_bool = matches!(hint.map(transparent_codegen_type), Some(Type::Bool))
                || integer.get_type().get_bit_width() == 1;
            if is_bool {
                let truthy = self
                    .const_string_abi("true", "template.bool.true")
                    .into_struct_value();
                let falsy = self
                    .const_string_abi("false", "template.bool.false")
                    .into_struct_value();
                return Ok(self
                    .builder
                    .build_select(integer, truthy, falsy, "template.bool")
                    .unwrap()
                    .into_struct_value());
            }
            let i128_type = self.context.i128_type();
            let unsigned = hint.is_some_and(is_unsigned_int_type);
            let (widened, formatter) = if unsigned {
                (
                    self.builder
                        .build_int_z_extend(integer, i128_type, "template.u128")
                        .unwrap(),
                    "tyrus_format_u128",
                )
            } else {
                (
                    self.builder
                        .build_int_s_extend(integer, i128_type, "template.i128")
                        .unwrap(),
                    "tyrus_format_i128",
                )
            };
            return Ok(self.emit_format_integer_runtime(widened, formatter, None));
        }
        if let Some(Type::Union(variants)) = hint.map(transparent_codegen_type) {
            return self.emit_template_union_string(value, variants);
        }
        Err(format!(
            "error: unsupported value in template interpolation (type `{}`)",
            hint.map_or_else(|| "unknown".to_string(), std::string::ToString::to_string)
        ))
    }

    fn emit_template_union_string(
        &mut self,
        value: BasicValueEnum<'ctx>,
        variants: &[Type],
    ) -> Result<StructValue<'ctx>, String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: template union outside function")?;

        let slot = self
            .builder
            .build_alloca(self.union_llvm_type(variants), "template.union")
            .map_err(|error| error.to_string())?;
        self.builder
            .build_store(slot, value)
            .map_err(|error| error.to_string())?;
        let tag = self.load_union_tag(variants, slot)?;

        let merge = self
            .context
            .append_basic_block(function, "template.union.merge");
        let mut arms = Vec::with_capacity(variants.len());
        let mut next_test = self
            .context
            .append_basic_block(function, "template.union.test.0");
        self.builder
            .build_unconditional_branch(next_test)
            .map_err(|error| error.to_string())?;

        for (index, variant) in variants.iter().enumerate() {
            let render = self
                .context
                .append_basic_block(function, &format!("template.union.render.{index}"));
            let miss = if index + 1 < variants.len() {
                self.context
                    .append_basic_block(function, &format!("template.union.test.{}", index + 1))
            } else {
                self.context
                    .append_basic_block(function, "template.union.unreachable")
            };
            self.builder.position_at_end(next_test);
            let matches = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    self.context.i8_type().const_int(index as u64, false),
                    &format!("template.union.is.{index}"),
                )
                .map_err(|error| error.to_string())?;
            self.builder
                .build_conditional_branch(matches, render, miss)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(render);
            let payload = self.load_union_payload(
                variants,
                slot,
                variant,
                &format!("template.union.value.{index}"),
            )?;
            let rendered = self.emit_template_value_string(payload, Some(variant))?;
            let incoming = self
                .builder
                .get_insert_block()
                .ok_or("internal: template union arm left no block")?;
            self.builder
                .build_unconditional_branch(merge)
                .map_err(|error| error.to_string())?;
            arms.push((rendered, incoming));
            next_test = miss;
        }

        self.builder.position_at_end(next_test);
        let empty = self.const_string_abi("", "template.union.empty");
        self.builder
            .build_unconditional_branch(merge)
            .map_err(|error| error.to_string())?;
        arms.push((empty.into_struct_value(), next_test));

        self.builder.position_at_end(merge);
        let phi = self
            .builder
            .build_phi(self.string_type(), "template.union.text")
            .map_err(|error| error.to_string())?;
        for (rendered, block) in &arms {
            phi.add_incoming(&[(rendered as &dyn inkwell::values::BasicValue, *block)]);
        }
        Ok(phi.as_basic_value().into_struct_value())
    }

    fn emit_assert_builtin(
        &mut self,
        callee: &str,
        args: &[Expr],
        span: tyrus_core::span::Span,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if callee == "assertThrows" {
            return self.emit_assert_throws(args, span).map(Some);
        }
        let (condition, default_message): (Expr, &str) = match callee {
            "assert" => (args[0].clone(), "assertion failed"),
            "assertEqual" => (
                Expr::synthetic(ExprKind::BinOp {
                    op: tyrus_core::ast::BinOp::Eq,
                    left: Box::new(args[0].clone()),
                    right: Box::new(args[1].clone()),
                }),
                "assertEqual failed: values are not equal",
            ),
            _ => return Ok(None),
        };
        let message = match (callee, args.get(1)) {
            ("assert", Some(message)) => message.clone(),
            _ => Expr::synthetic(ExprKind::StrLit(default_message.to_string())),
        };
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| "internal: assert outside a function".to_string())?;
        let passed = self.emit_expr_as(&condition, &Type::Bool)?.into_int_value();
        let fail = self.context.append_basic_block(function, "assert.fail");
        let cont = self.context.append_basic_block(function, "assert.cont");
        self.builder
            .build_conditional_branch(passed, cont, fail)
            .unwrap();
        self.builder.position_at_end(fail);
        if self.capture_test_assertions && !span.is_dummy() {
            let message = self.emit_expr_as(&message, &Type::Str)?.into_struct_value();
            let owns_message = self.forget_owned_native_result(message.into());
            let offset = self.context.i64_type().const_int(span.start as u64, false);
            let record = self.declare_libc_fn(
                "tyrus_test_record_assertion_failure",
                &[self.string_abi_type(), self.context.i64_type().into()],
                None,
            );
            self.builder
                .build_call(
                    record,
                    &[message.into(), offset.into()],
                    "assert.record_failure",
                )
                .map_err(|error| error.to_string())?;
            if owns_message {
                self.emit_string_release(message)?;
            }
            self.builder
                .build_unconditional_branch(cont)
                .map_err(|error| error.to_string())?;
        } else {
            self.emit_throw(&message, function)?;
        }
        self.builder.position_at_end(cont);
        Ok(Some(self.context.i32_type().const_zero().into()))
    }

    pub(super) fn emit_panic_builtin(&mut self, args: &[Expr]) -> Result<(), String> {
        let message = args
            .first()
            .ok_or_else(|| "error: `panic` expects 1 argument, found 0".to_string())?;
        let message = self.emit_expr_as(message, &Type::Str)?.into_struct_value();
        let panic_fn = self.declare_libc_fn("tyrus_panic_user", &[self.string_abi_type()], None);
        self.builder
            .build_call(panic_fn, &[message.into()], "")
            .unwrap();
        self.builder.build_unreachable().unwrap();
        Ok(())
    }

    fn emit_assert_throws(
        &mut self,
        args: &[Expr],
        span: tyrus_core::span::Span,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| "internal: assertThrows outside a function".to_string())?;
        let ExprKind::Ident(name) = args[0].kind() else {
            return Err("internal: assertThrows argument is not a function name".to_string());
        };
        let thrown = self
            .fn_throws
            .get(name)
            .and_then(|types| types.first())
            .cloned()
            .ok_or_else(|| format!("internal: `{name}` declares no thrown type"))?;

        let id = self.next_assert_throws_id;
        self.next_assert_throws_id += 1;
        let flag = format!("__tyrus_threw_{id}");
        let caught = format!("__tyrus_caught_{id}");

        let mut catch_body = vec![Stmt::synthetic(StmtKind::Assign {
            name: flag.clone(),
            op: tyrus_core::ast::AssignOp::Assign,
            value: Expr::synthetic(ExprKind::BoolLit(true)),
        })];
        if let Some(expected) = args.get(1) {
            catch_body.push(Stmt::synthetic(StmtKind::Expr(Expr::new(
                ExprKind::FnCall {
                    callee: "assert".to_string(),
                    type_args: Vec::new(),
                    args: vec![
                        Expr::synthetic(ExprKind::BinOp {
                            op: tyrus_core::ast::BinOp::Eq,
                            left: Box::new(Expr::synthetic(ExprKind::Ident(caught.clone()))),
                            right: Box::new(expected.clone()),
                        }),
                        Expr::synthetic(ExprKind::StrLit(format!(
                            "assertThrows failed: `{name}` threw a different message"
                        ))),
                    ],
                },
                span,
            ))));
        }

        let statements = vec![
            Stmt::synthetic(StmtKind::VarDecl {
                mutable: true,
                name: flag.clone(),
                ty: Type::Bool,
                value: Some(Expr::synthetic(ExprKind::BoolLit(false))),
                onfail: None,
            }),
            Stmt::synthetic(StmtKind::TryCatch {
                try_body: vec![Stmt::synthetic(StmtKind::Expr(Expr::synthetic(
                    ExprKind::FnCall {
                        callee: name.clone(),
                        type_args: Vec::new(),
                        args: Vec::new(),
                    },
                )))],
                catch_binding: caught,
                catch_type: thrown,
                catch_type_span: tyrus_core::span::Span::DUMMY,
                catch_body,
                finally_body: None,
            }),
            Stmt::synthetic(StmtKind::Expr(Expr::new(
                ExprKind::FnCall {
                    callee: "assert".to_string(),
                    type_args: Vec::new(),
                    args: vec![
                        Expr::synthetic(ExprKind::Ident(flag)),
                        Expr::synthetic(ExprKind::StrLit(format!(
                            "assertThrows failed: `{name}` did not throw"
                        ))),
                    ],
                },
                span,
            ))),
        ];

        self.push_scope();
        for statement in &statements {
            self.emit_stmt(statement, function)?;
        }
        self.pop_scope();
        Ok(self.context.i32_type().const_zero().into())
    }

    fn emit_conversion_builtin(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let ptr = self.context.ptr_type(inkwell::AddressSpace::default());
        match callee {
            "String" => {
                let value = self.emit_expr(&args[0])?;
                let hint = self.expr_type_hint(&args[0]);
                let result = self.emit_template_value_string(value, hint.as_ref())?;
                if hint.as_ref().is_some_and(|ty| {
                    let ty = transparent_codegen_type(ty);
                    is_integer_type(ty) || is_float_type(ty) || matches!(ty, Type::Char)
                }) {
                    self.record_owned_native_result(result.into(), Type::Str);
                }
                Ok(Some(result.into()))
            }
            "parseInt" | "parseFloat" | "Number" | "parseDecimal" => {
                let text = self.emit_expr_as(&args[0], &Type::Str)?.into_struct_value();
                let function = match callee {
                    "parseInt" => "tyrus_parse_i64",
                    "parseFloat" => "tyrus_parse_f64_leading",
                    "parseDecimal" => "tyrus_parse_decimal",
                    _ => "tyrus_parse_f64_whole",
                };
                let parse_fn =
                    self.declare_libc_fn(function, &[self.string_abi_type()], Some(ptr.into()));
                let result = self
                    .builder
                    .build_call(parse_fn, &[text.into()], callee)
                    .unwrap()
                    .try_as_basic_value()
                    .basic()
                    .unwrap();
                let target = match callee {
                    "parseInt" => Type::I64,
                    "parseDecimal" => Type::Decimal,
                    _ => Type::F64,
                };
                self.record_owned_native_result_with(
                    result,
                    Type::Optional(Box::new(target)),
                    crate::ReleasePlan::Value,
                );
                Ok(Some(result))
            }
            _ => Ok(None),
        }
    }

    fn emit_float_predicate_builtin(
        &mut self,
        callee: &str,
        args: &[Expr],
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        if callee != "isNaN" {
            return Ok(None);
        }
        let value = self.emit_expr(&args[0])?.into_float_value();
        let result = self
            .builder
            .build_float_compare(inkwell::FloatPredicate::UNO, value, value, "is_nan")
            .map_err(|error| error.to_string())?;
        Ok(Some(result.into()))
    }

    pub(super) fn emit_template_literal(
        &mut self,
        parts: &[TemplatePart],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let mut pieces: Vec<StructValue<'ctx>> = Vec::with_capacity(parts.len());
        let mut disposable_intermediates: Vec<StructValue<'ctx>> = Vec::new();

        for part in parts {
            match part {
                TemplatePart::Text(text) => pieces.push(self.const_string(text)),
                TemplatePart::Expr(expr, spec) => {
                    let value = self.emit_expr(expr)?;
                    let hint = self.expr_type_hint(expr);
                    let text = match spec {
                        Some(FormatSpec::FixedPoint(precision)) => {
                            let text = self
                                .emit_format_f64_precision(value.into_float_value(), *precision);
                            disposable_intermediates.push(text);
                            text
                        }
                        Some(FormatSpec::ZeroPadded(width)) => {
                            let text = self.emit_format_int_padded(
                                value.into_int_value(),
                                hint.as_ref(),
                                *width,
                            );
                            disposable_intermediates.push(text);
                            text
                        }
                        None => {
                            let text = self.emit_template_value_string(value, hint.as_ref())?;
                            if !value.is_struct_value() {
                                disposable_intermediates.push(text);
                            }
                            text
                        }
                    };
                    pieces.push(text);
                }
            }
        }

        let joined = self.emit_string_join(&pieces, "template")?;
        for intermediate in disposable_intermediates {
            self.emit_string_release(intermediate)?;
        }
        self.record_owned_native_result(joined.into(), Type::Str);
        Ok(joined.into())
    }

    /// A named function used as a value becomes a closure `{ function, null }`.
    /// Returns `None` when `name` is not a known named function (so the caller
    /// falls back to its own error). The closure's function pointer is a thunk
    /// that adapts the environment-first closure ABI to the named function's
    /// plain signature, discarding the (always null) environment.
    pub(super) fn materialize_function_value(
        &mut self,
        name: &str,
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let (param_types, ret) = self.fn_signatures.get(name).cloned()?;
        let fn_val = *self.fns.get(name)?;
        Some((|| {
            let thunk = self.build_fn_value_thunk(name, fn_val, &param_types, &ret)?;
            let closure_type = self
                .llvm_type(&Type::Function {
                    params: param_types.clone(),
                    ret: Box::new(ret.clone()),
                })
                .into_struct_type();
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let mut closure = closure_type.get_undef();
            closure = self
                .builder
                .build_insert_value(
                    closure,
                    thunk.as_global_value().as_pointer_value(),
                    0,
                    "fnval.function",
                )
                .unwrap()
                .into_struct_value();
            closure = self
                .builder
                .build_insert_value(closure, pointer.const_null(), 1, "fnval.env")
                .unwrap()
                .into_struct_value();
            Ok(closure.into())
        })())
    }

    /// The closure-ABI thunk `__tyrus_fnval_<name>(env, args…)` that forwards to
    /// the named function `fn_val(args…)`. Generated once per function.
    fn build_fn_value_thunk(
        &mut self,
        name: &str,
        fn_val: FunctionValue<'ctx>,
        param_types: &[Type],
        ret: &Type,
    ) -> Result<FunctionValue<'ctx>, String> {
        let thunk_name = format!("__tyrus_fnval_{name}");
        if let Some(existing) = self.module.get_function(&thunk_name) {
            return Ok(existing);
        }
        let thunk_type = self.llvm_closure_fn_type(param_types, ret);
        let thunk = self.module.add_function(&thunk_name, thunk_type, None);
        let outer_block = self.builder.get_insert_block();
        let entry = self.context.append_basic_block(thunk, "entry");
        self.builder.position_at_end(entry);
        let args: Vec<BasicMetadataValueEnum<'ctx>> = (0..param_types.len())
            .map(|index| thunk.get_nth_param(index as u32 + 1).unwrap().into())
            .collect();
        let call = self
            .builder
            .build_call(fn_val, &args, "fnval.call")
            .unwrap();
        if matches!(ret, Type::Void) {
            self.builder.build_return(None).unwrap();
        } else {
            let value = call
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| format!("internal: `{name}` thunk lost its return value"))?;
            self.builder.build_return(Some(&value)).unwrap();
        }
        if let Some(block) = outer_block {
            self.builder.position_at_end(block);
        }
        Ok(thunk)
    }

    fn emit_escaping_env_drop(
        &mut self,
        symbol: &str,
        environment_type: inkwell::types::StructType<'ctx>,
        captures: &[(String, PointerValue<'ctx>, Type)],
    ) -> Result<Option<FunctionValue<'ctx>>, String> {
        if !captures
            .iter()
            .any(|(_, _, ty)| self.escaping_capture_is_owned(ty) || self.capture_is_closure(ty))
        {
            return Ok(None);
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let restore = self.builder.get_insert_block();
        let drop = self.module.add_function(
            &format!("{symbol}_environment_drop"),
            self.context.void_type().fn_type(&[pointer.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(drop, "entry");
        self.builder.position_at_end(entry);
        let environment = drop.get_first_param().unwrap().into_pointer_value();
        for (index, (name, _, ty)) in captures.iter().enumerate() {
            let owned = self.escaping_capture_is_owned(ty);
            let closure = self.capture_is_closure(ty);
            if !owned && !closure {
                continue;
            }
            let field = self
                .builder
                .build_struct_gep(environment_type, environment, index as u32 + 2, name)
                .unwrap();
            if owned {
                self.emit_async_drop_value_in_place(drop, field, ty)?;
            } else {
                let captured = self
                    .builder
                    .build_load(self.llvm_type(ty), field, name)
                    .unwrap();
                self.emit_closure_release(captured)?;
            }
        }
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[environment.into()], "")
            .unwrap();
        self.builder.build_return(None).unwrap();
        if let Some(block) = restore {
            self.builder.position_at_end(block);
        }
        Ok(Some(drop))
    }

    pub(super) fn emit_expr_for_side_effects(
        &mut self,
        expr: &Expr,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        match expr.kind() {
            ExprKind::If {
                cond,
                then_block,
                else_block,
            } => self.emit_if_stmt(cond, then_block, else_block.as_deref(), fn_val),
            ExprKind::BinOp {
                op: op @ (BinOp::And | BinOp::Or),
                left,
                right,
            } => self.emit_logical_side_effect(op, left, right, fn_val),
            _ => {
                if let Some(ty) = self.expr_type_hint(expr).map(|ty| self.resolve_type(&ty))
                    && matches!(transparent_codegen_type(&ty), Type::Function { .. })
                    && matches!(
                        expr.kind(),
                        ExprKind::FnCall { .. }
                            | ExprKind::MethodCall { .. }
                            | ExprKind::CallValue { .. }
                            | ExprKind::ArrowFn { .. }
                    )
                {
                    let value = self.emit_expr(expr)?;
                    self.emit_closure_release(value)?;
                    return Ok(());
                }
                let value = self.emit_expr(expr)?;
                if !self.async_results.is_empty()
                    && let Some(ty) = self.expr_type_hint(expr).map(|ty| self.resolve_type(&ty))
                {
                    self.emit_async_cleanup_source_expression(value, &ty, expr)?;
                }
                Ok(())
            }
        }
    }

    fn collect_string_concat_chain<'e>(
        &mut self,
        left: &'e Expr,
        right: &'e Expr,
    ) -> Option<Vec<&'e Expr>> {
        let mut parts = vec![right];
        let mut node = left;
        loop {
            match node.kind() {
                ExprKind::BinOp {
                    op: BinOp::Add,
                    left: inner_left,
                    right: inner_right,
                } => {
                    parts.push(inner_right);
                    node = inner_left;
                }
                _ => {
                    parts.push(node);
                    break;
                }
            }
        }
        if parts.len() < 3 {
            return None;
        }
        parts.reverse();
        if parts
            .iter()
            .all(|part| matches!(self.expr_type_hint(part), Some(Type::Str)))
        {
            Some(parts)
        } else {
            None
        }
    }

    fn emit_string_concat_chain(
        &mut self,
        parts: &[&Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let mut values = Vec::with_capacity(parts.len());
        for part in parts {
            values.push(self.emit_expr_as(part, &Type::Str)?);
        }
        let structs = values
            .iter()
            .map(|value| value.into_struct_value())
            .collect::<Vec<_>>();
        let concatenated = self.emit_string_concat_many(&structs)?;
        if !self.async_results.is_empty() {
            for (value, part) in values.iter().zip(parts) {
                self.emit_async_cleanup_source_expression(*value, &Type::Str, part)?;
            }
        }
        self.record_owned_native_result(concatenated, Type::Str);
        Ok(concatenated)
    }

    fn emit_short_circuit_boolean(
        &mut self,
        op: &BinOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let left_value = self.emit_condition_value(left)?;
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: logical expression outside function")?;
        let right_block = self.context.append_basic_block(function, "logical.right");
        let merge_block = self.context.append_basic_block(function, "logical.merge");
        let left_block = self.builder.get_insert_block().unwrap();

        match op {
            BinOp::And => self
                .builder
                .build_conditional_branch(left_value, right_block, merge_block)
                .unwrap(),
            BinOp::Or => self
                .builder
                .build_conditional_branch(left_value, merge_block, right_block)
                .unwrap(),
            _ => unreachable!(),
        };

        self.builder.position_at_end(right_block);
        self.push_scope();
        if matches!(op, BinOp::And)
            && let Some(name) = Self::detect_optional_presence_narrowing(left)
            && let Some(concrete) = self.optional_present_type(&name)
        {
            self.store_concrete_type(&name, concrete);
        }
        let right_value = self.emit_condition_value(right)?;
        self.pop_scope();
        self.builder
            .build_unconditional_branch(merge_block)
            .unwrap();
        let evaluated_right_block = self.builder.get_insert_block().unwrap();

        self.builder.position_at_end(merge_block);
        let phi = self
            .builder
            .build_phi(self.context.bool_type(), "logical")
            .unwrap();
        phi.add_incoming(&[
            (&left_value, left_block),
            (&right_value, evaluated_right_block),
        ]);
        Ok(phi.as_basic_value())
    }

    fn http_response_header_call(&mut self, expr: &Expr) -> bool {
        let ExprKind::MethodCall { object, method, .. } = expr.kind() else {
            return false;
        };
        if method != "header" {
            return false;
        }
        matches!(
            self.expr_type_hint(object),
            Some(Type::Named(ref name))
                if name == "HttpClientResponse" || name == "HttpClientStreamResponse"
        )
    }

    pub(super) fn emit_value_equality(
        &mut self,
        lv: BasicValueEnum<'ctx>,
        rv: BasicValueEnum<'ctx>,
        left_expr: &Expr,
        right_expr: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if lv.is_int_value() {
            Ok(self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    lv.into_int_value(),
                    rv.into_int_value(),
                    "eq",
                )
                .unwrap()
                .into())
        } else if self.is_string_value(lv) && self.is_string_value(rv) {
            self.emit_string_equality(lv.into_struct_value(), rv.into_struct_value(), true)
        } else if (lv.is_pointer_value() || self.is_string_value(lv))
            && (rv.is_pointer_value() || self.is_string_value(rv))
        {
            let operand = if matches!(right_expr.kind(), ExprKind::Null) {
                lv
            } else if matches!(left_expr.kind(), ExprKind::Null) {
                rv
            } else {
                return Err("error: pointer equality is only supported against `null`".to_string());
            };
            let is_null = if self.is_string_value(operand) {
                self.string_is_null(operand.into_struct_value())?
            } else {
                self.builder
                    .build_is_null(operand.into_pointer_value(), "is_null")
                    .unwrap()
            };
            Ok(is_null.into())
        } else if let Some(equality) =
            self.emit_union_literal_equality(lv, rv, left_expr, right_expr)?
        {
            Ok(equality)
        } else if lv.is_float_value() && rv.is_float_value() {
            Ok(self
                .builder
                .build_float_compare(
                    inkwell::FloatPredicate::OEQ,
                    lv.into_float_value(),
                    rv.into_float_value(),
                    "feq",
                )
                .unwrap()
                .into())
        } else {
            Err(format!(
                "internal: cannot compare a `{}` with a `{}`",
                self.expr_type_hint(left_expr)
                    .map_or_else(|| "unknown".to_string(), |ty| ty.to_string()),
                self.expr_type_hint(right_expr)
                    .map_or_else(|| "unknown".to_string(), |ty| ty.to_string()),
            ))
        }
    }

    fn emit_union_literal_equality(
        &mut self,
        lv: BasicValueEnum<'ctx>,
        rv: BasicValueEnum<'ctx>,
        left_expr: &Expr,
        right_expr: &Expr,
    ) -> Result<Option<BasicValueEnum<'ctx>>, String> {
        let union_side = |expr: &Expr| {
            self.expr_type_hint(expr)
                .map(|ty| self.resolve_type(&ty))
                .and_then(|ty| match transparent_codegen_type(&ty) {
                    Type::Union(variants) => Some(variants.clone()),
                    _ => None,
                })
        };
        let (variants, union_value, literal) = match (union_side(left_expr), union_side(right_expr))
        {
            (Some(variants), None) => (variants, lv, right_expr),
            (None, Some(variants)) => (variants, rv, left_expr),
            _ => return Ok(None),
        };
        let Some(index) = variants.iter().position(
            |variant| matches!(variant, Type::Lit(value) if value.kind() == literal.kind()),
        ) else {
            return Ok(None);
        };
        if !union_value.is_struct_value() {
            return Ok(None);
        }
        let tag = self
            .builder
            .build_extract_value(union_value.into_struct_value(), 0, "eq.union.tag")
            .map_err(|error| error.to_string())?
            .into_int_value();
        Ok(Some(
            self.builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    self.context.i8_type().const_int(index as u64, false),
                    "eq.union.is",
                )
                .map_err(|error| error.to_string())?
                .into(),
        ))
    }

    pub(super) fn emit_string_equality(
        &self,
        left: StructValue<'ctx>,
        right: StructValue<'ctx>,
        equal: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let are_equal = self.emit_string_equals(left, right, "string_equal")?;
        Ok(if equal {
            are_equal.into()
        } else {
            self.builder
                .build_not(are_equal, "string_not_equal")
                .map_err(|error| error.to_string())?
                .into()
        })
    }

    fn http_status_arg(&self, arg: &Expr) -> Option<u16> {
        let Expr {
            kind: ExprKind::FieldAccess { object, field },
            ..
        } = arg
        else {
            return None;
        };
        let Expr {
            kind: ExprKind::Ident(enum_name),
            ..
        } = object.as_ref()
        else {
            return None;
        };
        self.enums.get(enum_name).and_then(|enum_info| {
            enum_info.members.iter().find_map(|(member, value)| {
                if member == field {
                    match value.kind() {
                        ExprKind::IntLit(value) => u16::try_from(*value).ok(),
                        _ => None,
                    }
                } else {
                    None
                }
            })
        })
    }

    fn load_narrowed_variant(
        &self,
        name: &str,
        ptr: PointerValue<'ctx>,
        resolved_ty: &Type,
    ) -> Option<Result<BasicValueEnum<'ctx>, String>> {
        let variants = match resolved_ty {
            Type::Union(variants) => variants.clone(),
            Type::Result { success, error } => {
                vec![success.as_ref().clone(), error.as_ref().clone()]
            }
            _ => return None,
        };
        let concrete = self.load_concrete_type(name)?;
        if matches!(
            transparent_codegen_type(&concrete),
            Type::Union(_) | Type::Result { .. }
        ) {
            return None;
        }
        Some(self.load_union_payload(&variants, ptr, &concrete, name))
    }

    fn algorithm_constructor_type(&self, callee: &str, type_args: &[Type]) -> Option<Type> {
        use tyrus_core::algorithm::*;
        let elem = || type_args.first().cloned().unwrap_or(Type::Unknown);
        let pair = || {
            (
                type_args.first().cloned().unwrap_or(Type::Unknown),
                type_args.get(1).cloned().unwrap_or(Type::Unknown),
            )
        };
        Some(match callee {
            UNION_FIND_NEW_TARGET => Type::UnionFind(Box::new(elem())),
            BINARY_HEAP_NEW_TARGET => Type::BinaryHeap(Box::new(elem())),
            BINOMIAL_HEAP_NEW_TARGET => Type::BinomialHeap(Box::new(elem())),
            COUNTER_NEW_TARGET => Type::Counter(Box::new(elem())),
            COUNTER_FROM_ARRAY_TARGET => Type::Counter(Box::new(elem())),
            DEQUE_NEW_TARGET => Type::Deque(Box::new(elem())),
            FENWICK_TREE_NEW_TARGET => Type::FenwickTree,
            BUBBLE_SORT_INFO_TARGET => Type::Named("BubbleSortInfo".to_string()),
            BINARY_SEARCH_INFO_TARGET => Type::Named("SearchInfo".to_string()),
            INTERPOLATION_SEARCH_INFO_TARGET => Type::Named("InterpolationSearchInfo".to_string()),
            EXPONENTIAL_SEARCH_INFO_TARGET => Type::Named("ExponentialSearchInfo".to_string()),
            QUICKSELECT_INFO_TARGET => Type::Named("SelectionInfo".to_string()),
            DIJKSTRA_INFO_TARGET => Type::Named("DijkstraInfo".to_string()),
            A_STAR_INFO_TARGET => Type::Named("AStarInfo".to_string()),
            BELLMAN_FORD_INFO_TARGET => Type::Named("BellmanFordInfo".to_string()),
            SPFA_INFO_TARGET => Type::Named("SpfaInfo".to_string()),
            JOHNSON_INFO_TARGET => Type::Named("JohnsonInfo".to_string()),
            FLOYD_WARSHALL_INFO_TARGET => Type::Named("FloydWarshallInfo".to_string()),
            KRUSKAL_INFO_TARGET => Type::Named("KruskalInfo".to_string()),
            PRIM_INFO_TARGET => Type::Named("PrimInfo".to_string()),
            BORUVKA_INFO_TARGET => Type::Named("BoruvkaInfo".to_string()),
            STEINER_TREE_INFO_TARGET => Type::Named("SteinerTreeInfo".to_string()),
            EDMONDS_KARP_INFO_TARGET => Type::Named("EdmondsKarpInfo".to_string()),
            DINIC_INFO_TARGET => Type::Named("DinicInfo".to_string()),
            PUSH_RELABEL_INFO_TARGET => Type::Named("PushRelabelInfo".to_string()),
            MIN_CUT_INFO_TARGET => Type::Named("MinCutInfo".to_string()),
            MIN_COST_MAX_FLOW_INFO_TARGET => Type::Named("MinCostMaxFlowInfo".to_string()),
            TOPOLOGICAL_KAHN_INFO_TARGET => Type::Named("TopologicalSortKahnInfo".to_string()),
            TOPOLOGICAL_DFS_INFO_TARGET => Type::Named("TopologicalSortDfsInfo".to_string()),
            TARJAN_SCC_INFO_TARGET => Type::Named("TarjanSccInfo".to_string()),
            KOSARAJU_SCC_INFO_TARGET => Type::Named("KosarajuSccInfo".to_string()),
            FLEURY_EULERIAN_INFO_TARGET => Type::Named("FleuryEulerianInfo".to_string()),
            HIERHOLZER_EULERIAN_INFO_TARGET => Type::Named("HierholzerEulerianInfo".to_string()),
            FIND_BRIDGES_INFO_TARGET => Type::Named("FindBridgesInfo".to_string()),
            FIND_CUT_VERTICES_INFO_TARGET => Type::Named("FindCutVerticesInfo".to_string()),
            BICONNECTED_COMPONENTS_INFO_TARGET => {
                Type::Named("BiconnectedComponentsInfo".to_string())
            }
            EDMONDS_BLOSSOM_INFO_TARGET => Type::Named("EdmondsBlossomInfo".to_string()),
            HUNGARIAN_INFO_TARGET => Type::Named("HungarianInfo".to_string()),
            BUBBLE_SORT_NEW_TARGET => Type::BubbleSort,
            SEGMENT_TREE_NEW_TARGET => Type::SegmentTree,
            SPARSE_TABLE_NEW_TARGET => Type::SparseTable,
            SEGMENT_TREE_LAZY_NEW_TARGET => Type::SegmentTreeLazy,
            BIT_SET_NEW_TARGET => Type::BitSet,
            BIT_SET_FROM_ARRAY_TARGET => Type::BitSet,
            TRIE_NEW_TARGET => Type::Trie,
            SUFFIX_TREE_NEW_TARGET => Type::SuffixTree,
            LCA_NEW_TARGET => Type::Lca,
            HLD_NEW_TARGET => Type::Hld,
            CENTROID_NEW_TARGET => Type::Centroid,
            BLOOM_FILTER_NEW_TARGET => Type::BloomFilter,
            LINKED_LIST_NEW_TARGET => Type::LinkedList(Box::new(elem())),
            LINKED_LIST_FROM_ARRAY_TARGET => Type::LinkedList(Box::new(elem())),
            DOUBLE_LINKED_LIST_NEW_TARGET => Type::DoubleLinkedList(Box::new(elem())),
            DOUBLE_LINKED_LIST_FROM_ARRAY_TARGET => Type::DoubleLinkedList(Box::new(elem())),
            AVL_TREE_NEW_TARGET => Type::AvlTree(Box::new(elem())),
            RED_BLACK_TREE_NEW_TARGET => Type::RedBlackTree(Box::new(elem())),
            SPLAY_TREE_NEW_TARGET => Type::SplayTree(Box::new(elem())),
            TREAP_NEW_TARGET => Type::Treap(Box::new(elem())),
            SORTED_SET_NEW_TARGET => Type::SortedSet(Box::new(elem())),
            SKIP_LIST_NEW_TARGET => Type::SkipList(Box::new(elem())),
            SORTED_MAP_NEW_TARGET => Type::SortedMap(Box::new(pair().0), Box::new(pair().1)),
            GRAPH_NEW_TARGET => Type::Graph(Box::new(pair().0), Box::new(pair().1)),
            _ => return None,
        })
    }

    pub(super) fn field_type_on(&self, object_type: &Type, field: &str) -> Option<Type> {
        if matches!(
            transparent_codegen_type(object_type),
            Type::Map(_, _)
                | Type::Set(_)
                | Type::BinaryHeap(_)
                | Type::BinomialHeap(_)
                | Type::UnionFind(_)
        ) && field == "size"
        {
            return Some(Type::Usize);
        }
        if matches!(transparent_codegen_type(object_type), Type::Array(_))
            && matches!(field, "length" | "capacity")
        {
            return Some(Type::Usize);
        }
        if matches!(transparent_codegen_type(object_type), Type::Str) && field == "length" {
            return Some(Type::Usize);
        }
        if let Type::Apply { name, args } = transparent_codegen_type(object_type) {
            let side = match (name.as_str(), args.as_slice(), field) {
                ("DuplexStream", [read, _], "readable")
                | ("TransformStream", [_, read], "readable") => {
                    Some(("ReadableStream", read.clone()))
                }
                ("DuplexStream", [_, write], "writable")
                | ("TransformStream", [write, _], "writable") => {
                    Some(("WritableStream", write.clone()))
                }
                _ => None,
            };
            if let Some((stream, item)) = side {
                return Some(Type::Apply {
                    name: stream.to_string(),
                    args: vec![item],
                });
            }
        }
        if matches!(transparent_codegen_type(object_type), Type::Regex) {
            return Some(match field {
                "source" | "flags" => Type::Str,
                "global" | "ignoreCase" | "multiline" | "dotAll" | "sticky" | "unicode"
                | "hasIndices" => Type::Bool,
                "lastIndex" => Type::I32,
                _ => return None,
            });
        }
        match transparent_codegen_type(object_type) {
            Type::Named(name) => self
                .interfaces
                .get(name)?
                .iter()
                .find(|candidate| candidate.name == *field)
                .map(field_storage_type),
            Type::Object(fields) => fields
                .iter()
                .find(|candidate| candidate.name == *field)
                .map(field_storage_type),
            _ => None,
        }
    }

    pub(super) fn resolve_binding_type(
        &self,
        ty: &Type,
        value: Option<&Expr>,
    ) -> Result<Type, String> {
        if !matches!(ty, Type::Infer) {
            return Ok(self.resolve_type(ty));
        }
        let value = value.ok_or_else(|| {
            "internal: inferred binding without an initializer reached code generation".to_string()
        })?;
        if let Expr {
            kind: ExprKind::ArrowFn {
                params, ret_type, ..
            },
            ..
        } = value
        {
            return Ok(self.resolve_type(&Type::Function {
                params: params.iter().map(|p| p.ty.clone()).collect(),
                ret: Box::new(ret_type.clone()),
            }));
        }
        let inferred = self.infer_decl_type(value).ok_or_else(|| {
            format!(
                "error: cannot infer a type for binding `{}`; add an explicit `: type` annotation",
                expr_display(value)
            )
        })?;
        Ok(self.resolve_type(&inferred))
    }

    pub(super) fn resolve_onfail_binding_type(
        &self,
        ty: &Type,
        value: &Expr,
        has_onfail: bool,
    ) -> Result<Type, String> {
        if !has_onfail || !matches!(ty, Type::Infer) {
            return self.resolve_binding_type(ty, Some(value));
        }
        let source_type = self
            .expr_type_hint(value)
            .map(|source_type| self.resolve_type(&source_type))
            .ok_or("internal: cannot determine the Result type used by `onfail`")?;
        let Type::Result { success, .. } = source_type else {
            return self.resolve_binding_type(ty, Some(value));
        };
        Ok(self.resolve_type(&success))
    }

    pub(super) fn declare_int_pow(&mut self, int_type: IntType<'ctx>) -> FunctionValue<'ctx> {
        let name = format!("__tyrus_pow_i{}", int_type.get_bit_width());
        if let Some(function) = self.module.get_function(&name) {
            return function;
        }

        let outer_block = self.builder.get_insert_block();
        let function_type = int_type.fn_type(&[int_type.into(), int_type.into()], false);
        let function = self.module.add_function(&name, function_type, None);
        let entry = self.context.append_basic_block(function, "entry");
        let invalid = self
            .context
            .append_basic_block(function, "invalid_exponent");
        let init = self.context.append_basic_block(function, "init");
        let condition = self.context.append_basic_block(function, "condition");
        let body = self.context.append_basic_block(function, "body");
        let done = self.context.append_basic_block(function, "done");
        let zero = int_type.const_zero();
        let one = int_type.const_int(1, false);
        let base = function.get_nth_param(0).unwrap().into_int_value();
        let exponent = function.get_nth_param(1).unwrap().into_int_value();

        self.builder.position_at_end(entry);
        let is_negative = self
            .builder
            .build_int_compare(IntPredicate::SLT, exponent, zero, "is_negative")
            .unwrap();
        self.builder
            .build_conditional_branch(is_negative, invalid, init)
            .unwrap();

        self.builder.position_at_end(invalid);
        self.emit_runtime_panic("error: integer exponent must be non-negative");

        self.builder.position_at_end(init);
        let result_slot = self.builder.build_alloca(int_type, "result").unwrap();
        let base_slot = self.builder.build_alloca(int_type, "base").unwrap();
        let exponent_slot = self.builder.build_alloca(int_type, "exponent").unwrap();
        self.builder.build_store(result_slot, one).unwrap();
        self.builder.build_store(base_slot, base).unwrap();
        self.builder.build_store(exponent_slot, exponent).unwrap();
        self.builder.build_unconditional_branch(condition).unwrap();

        self.builder.position_at_end(condition);
        let current_exponent = self
            .builder
            .build_load(int_type, exponent_slot, "current_exponent")
            .unwrap()
            .into_int_value();
        let has_more = self
            .builder
            .build_int_compare(IntPredicate::SGT, current_exponent, zero, "has_more")
            .unwrap();
        self.builder
            .build_conditional_branch(has_more, body, done)
            .unwrap();

        self.builder.position_at_end(body);
        let current_result = self
            .builder
            .build_load(int_type, result_slot, "current_result")
            .unwrap()
            .into_int_value();
        let current_base = self
            .builder
            .build_load(int_type, base_slot, "current_base")
            .unwrap()
            .into_int_value();
        let odd_bit = self
            .builder
            .build_and(current_exponent, one, "odd_bit")
            .unwrap();
        let is_odd = self
            .builder
            .build_int_compare(IntPredicate::NE, odd_bit, zero, "is_odd")
            .unwrap();
        let multiplied = self
            .builder
            .build_int_mul(current_result, current_base, "multiplied")
            .unwrap();
        let next_result = self
            .builder
            .build_select(is_odd, multiplied, current_result, "next_result")
            .unwrap();
        let next_base = self
            .builder
            .build_int_mul(current_base, current_base, "next_base")
            .unwrap();
        let next_exponent = self
            .builder
            .build_right_shift(current_exponent, one, false, "next_exponent")
            .unwrap();
        self.builder.build_store(result_slot, next_result).unwrap();
        self.builder.build_store(base_slot, next_base).unwrap();
        self.builder
            .build_store(exponent_slot, next_exponent)
            .unwrap();
        self.builder.build_unconditional_branch(condition).unwrap();

        self.builder.position_at_end(done);
        let result = self
            .builder
            .build_load(int_type, result_slot, "power_result")
            .unwrap();
        self.builder.build_return(Some(&result)).unwrap();

        if let Some(block) = outer_block {
            self.builder.position_at_end(block);
        }
        function
    }

    fn emit_incr_decr(
        &mut self,
        inner: &Expr,
        is_dec: bool,
        is_post: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let val = self.emit_expr(inner)?;
        let (ptr, ty) = self.load_var_for_mutation(inner)?;
        let llvm_ty = self.llvm_type(&ty);
        let one = self.numeric_one(llvm_ty);
        let result = if val.is_int_value() {
            let int_val = val.into_int_value();
            let one_int = one.into_int_value();
            if is_dec {
                self.builder
                    .build_int_sub(int_val, one_int, "decr")
                    .unwrap()
                    .into()
            } else {
                self.builder
                    .build_int_add(int_val, one_int, "incr")
                    .unwrap()
                    .into()
            }
        } else {
            let float_val = val.into_float_value();
            let one_float = one.into_float_value();
            if is_dec {
                self.builder
                    .build_float_sub(float_val, one_float, "decr")
                    .unwrap()
                    .into()
            } else {
                self.builder
                    .build_float_add(float_val, one_float, "incr")
                    .unwrap()
                    .into()
            }
        };
        self.builder.build_store(ptr, result).unwrap();
        if is_post { Ok(val) } else { Ok(result) }
    }

    fn ternary_arm_is_branchless(&self, expr: &Expr) -> bool {
        match expr.kind() {
            ExprKind::IntLit(_)
            | ExprKind::UIntLit(_)
            | ExprKind::FloatLit(_)
            | ExprKind::BoolLit(_) => true,
            ExprKind::Ident(name) => self.load_var(name).is_some(),
            ExprKind::FieldAccess { object, field } => {
                let ExprKind::Ident(name) = object.kind() else {
                    return false;
                };
                numeric_constant_type(name, field).is_some()
                    || self
                        .enums
                        .get(name)
                        .is_some_and(|info| info.members.iter().any(|(member, _)| member == field))
            }
            _ => false,
        }
    }

    fn ternary_branchless_type(&self, ty: &Type) -> bool {
        is_integer_type(ty)
            || matches!(
                transparent_codegen_type(ty),
                Type::Bool | Type::Char | Type::F32 | Type::F64
            )
    }

    fn ternary_common_arm_type(&mut self, then_expr: &Expr, else_expr: &Expr) -> Option<Type> {
        let then_ty = self.expr_type_hint(then_expr)?;
        let else_ty = self.expr_type_hint(else_expr)?;
        if then_ty == else_ty || !is_integer_type(&then_ty) || !is_integer_type(&else_ty) {
            return None;
        }
        let then_is_literal = Self::is_bare_integer_literal(then_expr);
        let else_is_literal = Self::is_bare_integer_literal(else_expr);
        match (then_is_literal, else_is_literal) {
            (true, false) => Some(else_ty),
            (false, true) => Some(then_ty),
            _ => None,
        }
    }

    fn is_bare_integer_literal(expr: &Expr) -> bool {
        match expr.kind() {
            ExprKind::IntLit(_) | ExprKind::UIntLit(_) => true,
            ExprKind::Neg(inner) => Self::is_bare_integer_literal(inner),
            _ => false,
        }
    }

    fn emit_ternary(
        &mut self,
        cond: &Expr,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: ternary outside function")?;
        let cond_val = self.emit_condition_value(cond)?;
        let arm_ty = self.ternary_common_arm_type(then_expr, else_expr);

        if Self::detect_optional_presence_narrowing(cond).is_none()
            && self.ternary_arm_is_branchless(then_expr)
            && self.ternary_arm_is_branchless(else_expr)
        {
            let unified = match arm_ty.clone() {
                Some(ty) => Some(ty),
                None => match (
                    self.expr_type_hint(then_expr),
                    self.expr_type_hint(else_expr),
                ) {
                    (Some(then_ty), Some(else_ty)) if then_ty == else_ty => Some(then_ty),
                    _ => None,
                },
            };
            if let Some(unified) = unified
                && self.ternary_branchless_type(&unified)
            {
                let then_val = self.emit_ternary_arm(then_expr, arm_ty.as_ref())?;
                let else_val = self.emit_ternary_arm(else_expr, arm_ty.as_ref())?;
                if then_val.get_type() == else_val.get_type() {
                    return Ok(self
                        .builder
                        .build_select(cond_val, then_val, else_val, "tern.select")
                        .unwrap());
                }
            }
        }

        let then_bb = self.context.append_basic_block(current_fn, "tern.then");
        let else_bb = self.context.append_basic_block(current_fn, "tern.else");
        let merge_bb = self.context.append_basic_block(current_fn, "tern.merge");

        let branch = self
            .builder
            .build_conditional_branch(cond_val, then_bb, else_bb)
            .unwrap();

        self.builder.position_at_end(then_bb);
        self.push_scope();
        if let Some(name) = Self::detect_optional_presence_narrowing(cond)
            && let Some(concrete) = self.optional_present_type(&name)
        {
            self.store_concrete_type(&name, concrete);
        }
        let pending_before_then = self.pending_owned_native_result_refs();
        let then_val = self.emit_ternary_arm(then_expr, arm_ty.as_ref())?;
        let then_owned = self.take_owned_native_result_with_type(then_val);
        let then_locals = self.take_arm_local_owned_results(&pending_before_then);
        self.pop_scope();
        let then_end = self.builder.get_insert_block().unwrap();
        let then_exit = self.builder.build_unconditional_branch(merge_bb).unwrap();

        self.builder.position_at_end(else_bb);
        let pending_before_else = self.pending_owned_native_result_refs();
        let else_val = self.emit_ternary_arm(else_expr, arm_ty.as_ref())?;
        let else_owned = self.take_owned_native_result_with_type(else_val);
        let else_locals = self.take_arm_local_owned_results(&pending_before_else);
        let else_end = self.builder.get_insert_block().unwrap();
        let else_exit = self.builder.build_unconditional_branch(merge_bb).unwrap();

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(then_val.get_type(), "tern.phi")
            .unwrap();
        phi.add_incoming(&[(&then_val, then_end), (&else_val, else_end)]);
        let result = phi.as_basic_value();
        for (locals, exit) in [(then_locals, then_exit), (else_locals, else_exit)] {
            for (value, ty, plan) in locals {
                self.carry_arm_local_owned_result(value, ty, &plan, branch, exit, merge_bb);
            }
        }

        match (then_owned, else_owned) {
            (Some((ty, plan)), Some(_)) => {
                self.record_owned_native_result_with(result, ty, plan);
            }
            (None, None) => {}
            (Some((ty, plan)), None) => {
                self.carry_ternary_owned_result(result, ty, &plan, branch, then_exit, merge_bb)
            }
            (None, Some((ty, plan))) => {
                self.carry_ternary_owned_result(result, ty, &plan, branch, else_exit, merge_bb)
            }
        }

        Ok(result)
    }

    /// A value an arm allocated but did not yield, such as the `Result` a `?`
    /// unwrapped, exists only on that arm's path. Left pending, it would be
    /// released at the end of the statement, after the merge, where it does not
    /// dominate the release and LLVM rejects the function. Releasing it before
    /// the arm leaves is not safe either: a `?` payload is a shallow copy of the
    /// `Result`'s contents, so the arm's value would point at freed memory.
    fn take_arm_local_owned_results(
        &mut self,
        pending_before: &[inkwell::llvm_sys::prelude::LLVMValueRef],
    ) -> Vec<(BasicValueEnum<'ctx>, Type, ReleasePlan)> {
        let (kept, arm_local) = std::mem::take(&mut self.pending_owned_native_results)
            .into_iter()
            .partition(|(value, _, _)| pending_before.contains(&value.as_value_ref()));
        self.pending_owned_native_results = kept;
        arm_local
    }

    fn pending_owned_native_result_refs(&self) -> Vec<inkwell::llvm_sys::prelude::LLVMValueRef> {
        self.pending_owned_native_results
            .iter()
            .map(|(value, _, _)| value.as_value_ref())
            .collect()
    }

    /// Carries an arm-local value past the merge so it is still released at the
    /// end of the statement, like the same value outside a ternary. The value
    /// goes to a slot and an owned bit records whether this arm ran: the bit is
    /// cleared before the branch and set only on the arm that allocated, so the
    /// release after the merge is dominated and happens exactly once.
    ///
    /// The conditional release frees deeply, so an arm-local value with any
    /// other release plan stays untracked rather than being freed incorrectly.
    fn carry_arm_local_owned_result(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: Type,
        plan: &ReleasePlan,
        branch: inkwell::values::InstructionValue<'ctx>,
        arm_exit: inkwell::values::InstructionValue<'ctx>,
        merge: inkwell::basic_block::BasicBlock<'ctx>,
    ) {
        if !matches!(plan, ReleasePlan::Deep) {
            return;
        }
        let bit_type = self.context.bool_type();
        let Ok(value_slot) = self.entry_alloca(value.get_type(), "tern.owned.value") else {
            return;
        };
        let Ok(bit_slot) = self.entry_alloca(bit_type, "tern.owned.bit") else {
            return;
        };
        self.builder.position_before(&branch);
        self.builder
            .build_store(bit_slot, bit_type.const_zero())
            .unwrap();
        self.builder.position_before(&arm_exit);
        self.builder.build_store(value_slot, value).unwrap();
        self.builder
            .build_store(bit_slot, bit_type.const_all_ones())
            .unwrap();
        self.builder.position_at_end(merge);
        let carried = self
            .builder
            .build_load(value.get_type(), value_slot, "tern.owned.carried")
            .unwrap();
        let owned = self
            .builder
            .build_load(bit_type, bit_slot, "tern.owned.ran")
            .unwrap()
            .into_int_value();
        self.record_conditionally_owned_result(carried, ty, owned);
    }

    /// Carries ownership of a ternary result when only one arm allocates.
    /// The merged value is a phi, so the runtime ownership bit follows which
    /// arm ran and guards cleanup of that phi at its eventual consumer.
    fn carry_ternary_owned_result(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: Type,
        plan: &ReleasePlan,
        branch: inkwell::values::InstructionValue<'ctx>,
        owned_arm_exit: inkwell::values::InstructionValue<'ctx>,
        merge: inkwell::basic_block::BasicBlock<'ctx>,
    ) {
        if !matches!(plan, ReleasePlan::Deep) || !self.async_results.is_empty() {
            return;
        }
        let bit_type = self.context.bool_type();
        let Ok(bit_slot) = self.entry_alloca(bit_type, "ternary.result.owned") else {
            return;
        };
        self.builder.position_before(&branch);
        self.builder
            .build_store(bit_slot, bit_type.const_zero())
            .unwrap();
        self.builder.position_before(&owned_arm_exit);
        self.builder
            .build_store(bit_slot, bit_type.const_all_ones())
            .unwrap();
        self.builder.position_at_end(merge);
        let owned = self
            .builder
            .build_load(bit_type, bit_slot, "ternary.result.owned.path")
            .unwrap()
            .into_int_value();
        self.record_conditionally_owned_result(value, ty, owned);
    }

    fn emit_ternary_arm(
        &mut self,
        expr: &Expr,
        arm_ty: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match arm_ty {
            Some(ty) => self.emit_expr_as(expr, ty),
            None => self.emit_expr(expr),
        }
    }

    /// The one implementation of the recoverable native error check that
    /// `AGENTS.md` mandates: take the last error the module recorded, and throw
    /// `<Module>Error` when it is present. Every `guard_<module>_runtime_error`
    /// is a call to this with its own names; the manual call after each dispatch
    /// stays where it was, because only the caller knows which dispatch can fail.
    pub(crate) fn guard_module_runtime_error(
        &mut self,
        module: &str,
        error_type: &'static str,
        thrown: ThrownValue,
        structured_async_rejection: bool,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let taken_ty = self
            .context
            .struct_type(&[self.string_abi_type(), self.string_abi_type()], false);
        let taken_slot = self
            .builder
            .build_alloca(taken_ty, &format!("{module}.error.taken"))
            .unwrap();
        self.builder
            .build_store(taken_slot, taken_ty.const_zero())
            .unwrap();
        let take = self.declare_libc_fn(
            &format!("tyrus_{module}_take_error"),
            &[pointer.into()],
            None,
        );
        self.builder
            .build_call(take, &[taken_slot.into()], "")
            .unwrap();
        let code_field = self
            .builder
            .build_struct_gep(
                taken_ty,
                taken_slot,
                0,
                &format!("{module}.error.code_field"),
            )
            .unwrap();
        let code = self
            .builder
            .build_load(
                self.string_abi_type(),
                code_field,
                &format!("{module}.error.code"),
            )
            .unwrap()
            .into_struct_value();
        let failed = self
            .builder
            .build_not(
                self.string_is_null(code)?,
                &format!("{module}.error.present"),
            )
            .map_err(|error| error.to_string())?;

        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| format!("error: {module} call is not inside a function"))?;
        let failure = self
            .context
            .append_basic_block(function, &format!("{module}.error"));
        let success = self
            .context
            .append_basic_block(function, &format!("{module}.ok"));
        self.builder
            .build_conditional_branch(failed, failure, success)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(failure);
        let message_field = self
            .builder
            .build_struct_gep(
                taken_ty,
                taken_slot,
                1,
                &format!("{module}.error.message_field"),
            )
            .unwrap();
        let message = self
            .builder
            .build_load(
                self.string_abi_type(),
                message_field,
                &format!("{module}.error.message"),
            )
            .unwrap();
        let error_type = Type::Named(error_type.to_string());
        let error = self
            .builder
            .build_insert_value(
                self.llvm_type(&error_type).into_struct_type().const_zero(),
                code,
                0,
                &format!("{module}.error.value.code"),
            )
            .map_err(|error| error.to_string())?;
        let error = self
            .builder
            .build_insert_value(error, message, 1, &format!("{module}.error.value.message"))
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let async_throw = structured_async_rejection
            .then(|| {
                self.async_results.last().copied().map(|promise| {
                    (
                        promise,
                        crate::AsyncRejection::Structured {
                            code_index: 0,
                            message_index: 1,
                        },
                    )
                })
            })
            .flatten();
        self.emit_recoverable_runtime_error_cleanups()?;
        self.emit_throw_value_in_async(
            error.into(),
            thrown,
            function,
            async_throw,
            self.context.bool_type().const_zero(),
            &error_type,
        )?;

        self.builder.position_at_end(success);
        Ok(())
    }

    pub(crate) fn guard_temporal_runtime_error(&mut self) -> Result<(), String> {
        self.guard_module_runtime_error(
            "temporal",
            "TemporalError",
            ThrownValue::Coded("TemporalError"),
            false,
        )
    }

    fn load_var_for_mutation(&self, expr: &Expr) -> Result<(PointerValue<'ctx>, Type), String> {
        match expr.kind() {
            ExprKind::Ident(name) => self
                .load_var(name)
                .ok_or_else(|| format!("internal: `{name}` not in codegen scope")),
            _ => Err("internal: increment/decrement target must be a variable".to_string()),
        }
    }

    fn numeric_one(&self, ty: BasicTypeEnum<'ctx>) -> BasicValueEnum<'ctx> {
        match ty {
            BasicTypeEnum::IntType(int_ty) => int_ty.const_int(1, false).into(),
            BasicTypeEnum::FloatType(float_ty) => float_ty.const_float(1.0).into(),
            _ => panic!("numeric_one called on non-numeric type"),
        }
    }
}

fn http_reply_schema_for_status(reply: &Type, status: u16) -> Option<Type> {
    match transparent_codegen_type(reply) {
        Type::Object(fields) => fields
            .iter()
            .find(|field| field.name == status.to_string())
            .map(field_storage_type),
        Type::Named(schema) => Some(Type::Named(schema.clone())),
        other => Some(other.clone()),
    }
}

fn parse_runtime_fn(callee: &str, target: &Type) -> Option<&'static str> {
    Some(match (callee, transparent_codegen_type(target)) {
        ("parseInt", Type::I8) => "tyrus_parse_i8",
        ("parseInt", Type::I16) => "tyrus_parse_i16",
        ("parseInt", Type::I32) => "tyrus_parse_i32",
        ("parseInt", Type::I64) => "tyrus_parse_i64",
        ("parseInt", Type::I128) => "tyrus_parse_i128",
        ("parseInt", Type::Isize) => "tyrus_parse_isize",
        ("parseInt", Type::U8) => "tyrus_parse_u8",
        ("parseInt", Type::U16) => "tyrus_parse_u16",
        ("parseInt", Type::U32) => "tyrus_parse_u32",
        ("parseInt", Type::U64) => "tyrus_parse_u64",
        ("parseInt", Type::U128) => "tyrus_parse_u128",
        ("parseInt", Type::Usize) => "tyrus_parse_usize",
        ("parseFloat", Type::F32) => "tyrus_parse_f32_leading",
        ("parseFloat", Type::F64) => "tyrus_parse_f64_leading",
        ("Number", Type::F32) => "tyrus_parse_f32_whole",
        ("Number", Type::F64) => "tyrus_parse_f64_whole",
        ("parseDecimal", Type::Decimal) => "tyrus_parse_decimal",
        _ => return None,
    })
}

fn int_literal_type(value: i128) -> Type {
    if value >= i128::from(i32::MIN) && value <= i128::from(i32::MAX) {
        Type::I32
    } else if value >= i128::from(i64::MIN) && value <= i128::from(i64::MAX) {
        Type::I64
    } else {
        Type::I128
    }
}
