use super::*;
use tyrus_core::ast::ExprKind;
use tyrus_core::ast::StmtKind;

pub(super) struct ArrayView<'ctx> {
    pub(super) slot: PointerValue<'ctx>,
    value: inkwell::values::StructValue<'ctx>,
    data: PointerValue<'ctx>,
    length: IntValue<'ctx>,
    array_type: Type,
    pub(super) element_type: Type,
    llvm_array_type: BasicTypeEnum<'ctx>,
}

#[path = "reductions/reductions.rs"]
mod reductions;
#[path = "transforms/transforms.rs"]
mod transforms;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_functional_array_method(
        &mut self,
        object: &Expr,
        method: &str,
        args: &[Expr],
        expected: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if method == "sort"
            && let ExprKind::Ident(name) = object.kind()
            && let Some(ownership) = self.async_frame_string_array_ownership(name)
            && let Some(ref array_type @ Type::Array(ref element)) = self.expr_type_hint(object)
            && matches!(transparent_codegen_type(element.as_ref()), Type::Str)
        {
            let slot = self
                .load_var(name)
                .map(|(slot, _)| slot)
                .ok_or_else(|| format!("internal: array `{name}` was not found"))?;
            let function = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
                .ok_or("internal: async string-array sort outside function")?;
            self.normalize_async_string_array_for_mutation(
                function, name, slot, ownership, array_type,
            )?;
        }
        let view = self.functional_array_view(object)?;
        match method {
            "map" => self.emit_map(&view, args, expected),
            "filter" => self.emit_filter(&view, args),
            "partition" => self.emit_partition(&view, args),
            "windows" => self.emit_windows(&view, args),
            "scan" => self.emit_scan(&view, args),
            "unique" => self.emit_unique(&view),
            "uniqueBy" => self.emit_unique_by(&view, args),
            "groupBy" => self.emit_group_by(&view, args),
            "cartesianProduct" => self.emit_cartesian_product(&view, args),
            "zip" => self.emit_zip(&view, args),
            "transpose" => self.emit_transpose(&view),
            "some" => self.emit_predicate_terminal(&view, args, false, true, "some"),
            "every" => self.emit_predicate_terminal(&view, args, true, false, "every"),
            "find" => self.emit_find_variant(&view, args, false, false),
            "findLast" => self.emit_find_variant(&view, args, true, false),
            "findIndex" => self.emit_find_variant(&view, args, false, true),
            "findLastIndex" => self.emit_find_variant(&view, args, true, true),
            "forEach" => self.emit_for_each(&view, args),
            "reduce" => self.emit_reduce(&view, args, expected, false),
            "reduceRight" => self.emit_reduce(&view, args, expected, true),
            "sort" => self.emit_sort(&view, args, false, None),
            "toSorted" => self.emit_sort(&view, args, true, None),
            "flat" => self.emit_flat(&view, args),
            "flatMap" => self.emit_flat_map(&view, args, expected),
            "count" => self.emit_count(&view, args),
            _ => Err(format!(
                "error: unsupported functional array method `{method}`"
            )),
        }
    }

    pub(super) fn borrowed_array_view(
        &mut self,
        slice: crate::arrays::TupleSlice<'ctx>,
        name: &str,
    ) -> Result<ArrayView<'ctx>, String> {
        let array_type = Type::Array(Box::new(slice.element_type.clone()));
        let llvm_array_type = self.llvm_type(&array_type);
        let struct_type = llvm_array_type.into_struct_type();
        let mut value = struct_type.get_undef();
        let stored_length = self.truncate_to_usize(slice.length);
        let fields: [BasicValueEnum<'ctx>; 3] = [
            slice.data.into(),
            stored_length.into(),
            stored_length.into(),
        ];
        for (index, field) in fields.into_iter().enumerate() {
            value = self
                .builder
                .build_insert_value(value, field, index as u32, name)
                .map_err(|error| error.to_string())?
                .into_struct_value();
        }
        let slot = self.entry_alloca(llvm_array_type, name).unwrap();
        self.builder.build_store(slot, value).unwrap();
        Ok(ArrayView {
            slot,
            value,
            data: slice.data,
            length: slice.length,
            array_type,
            element_type: slice.element_type,
            llvm_array_type,
        })
    }

    pub(super) fn functional_array_view(
        &mut self,
        object: &Expr,
    ) -> Result<ArrayView<'ctx>, String> {
        if let Some(slice) = self.tuple_slice(object, "functional_tuple")? {
            return self.borrowed_array_view(slice, "functional_tuple_view");
        }
        let array_type = self
            .expr_type_hint(object)
            .ok_or_else(|| "error: cannot infer array receiver type".to_string())?;
        let Type::Array(element) = transparent_codegen_type(&array_type) else {
            return Err(format!(
                "error: functional method requires an array, found `{array_type}`"
            ));
        };
        let element_type = element.as_ref().clone();
        let llvm_array_type = self.llvm_type(&array_type);
        let slot = if let ExprKind::Ident(name) = object.kind() {
            self.load_var(name)
                .map(|(slot, _)| slot)
                .ok_or_else(|| format!("internal: array `{name}` was not found"))?
        } else {
            let value = self.emit_expr(object)?;
            let slot = self
                .entry_alloca(llvm_array_type, "functional_array_receiver")
                .unwrap();
            self.builder.build_store(slot, value).unwrap();
            slot
        };
        let value = self
            .builder
            .build_load(llvm_array_type, slot, "functional_array")
            .unwrap()
            .into_struct_value();
        let data = self
            .builder
            .build_extract_value(value, 0, "functional_array_data")
            .unwrap()
            .into_pointer_value();
        let length = self
            .builder
            .build_extract_value(value, 1, "functional_array_length")
            .unwrap()
            .into_int_value();
        let length = self.extend_to_i64(length);
        Ok(ArrayView {
            slot,
            value,
            data,
            length,
            array_type,
            element_type,
            llvm_array_type,
        })
    }

    fn callback_arity(&self, callback: &Expr) -> Result<usize, String> {
        match callback.kind() {
            ExprKind::ArrowFn { params, .. } => Ok(params.len()),
            ExprKind::Ident(name) => self
                .expr_type_hint(callback)
                .and_then(|ty| match ty {
                    Type::Function { params, .. } => Some(params.len()),
                    _ => None,
                })
                .ok_or_else(|| format!("error: `{name}` is not a callback function")),
            _ => Err("error: array callback must be a function value".to_string()),
        }
    }

    pub(super) fn infer_callback_return_type(
        &self,
        callback: &Expr,
        available_params: &[Type],
    ) -> Option<Type> {
        match callback.kind() {
            ExprKind::Ident(_) => match self.expr_type_hint(callback)? {
                Type::Function { ret, .. } => Some(*ret),
                _ => None,
            },
            ExprKind::ArrowFn {
                params,
                ret_type,
                body,
                ..
            } => {
                if !matches!(ret_type, Type::Infer) {
                    return Some(ret_type.clone());
                }
                let bindings = params
                    .iter()
                    .enumerate()
                    .filter_map(|(index, parameter)| {
                        available_params.get(index).map(|ty| {
                            let name = parameter
                                .pattern
                                .binding_names()
                                .first()
                                .copied()
                                .unwrap_or("param")
                                .to_string();
                            (name, ty.clone())
                        })
                    })
                    .collect::<HashMap<_, _>>();
                body.iter().find_map(|statement| match statement {
                    Stmt {
                        kind: StmtKind::Return(Some(expression)),
                        ..
                    } => self.contextual_expr_type_hint(expression, &bindings),
                    _ => None,
                })
            }
            _ => None,
        }
    }

    pub(super) fn contextual_expr_type_hint(
        &self,
        expression: &Expr,
        bindings: &HashMap<String, Type>,
    ) -> Option<Type> {
        match expression.kind() {
            ExprKind::Ident(name) => bindings
                .get(name)
                .cloned()
                .or_else(|| self.expr_type_hint(expression)),
            ExprKind::IntLit(_) => Some(Type::I32),
            ExprKind::UIntLit(_) => Some(Type::U128),
            ExprKind::FloatLit(_) => Some(Type::F64),
            ExprKind::StrLit(_) | ExprKind::TemplateLit(_) => Some(Type::Str),
            ExprKind::BoolLit(_) | ExprKind::Not(_) => Some(Type::Bool),
            ExprKind::ArrayLit(values) => values.first().and_then(|value| {
                self.contextual_expr_type_hint(value, bindings)
                    .map(|element| Type::Array(Box::new(element)))
            }),
            ExprKind::ArrayRepeat { value, .. } => self
                .contextual_expr_type_hint(value, bindings)
                .map(|element| Type::Array(Box::new(element))),
            ExprKind::BinOp { op, left, right } => {
                if matches!(
                    op,
                    BinOp::Eq
                        | BinOp::Neq
                        | BinOp::Gt
                        | BinOp::Lt
                        | BinOp::Gte
                        | BinOp::Lte
                        | BinOp::And
                        | BinOp::Or
                ) {
                    Some(Type::Bool)
                } else {
                    self.contextual_expr_type_hint(left, bindings)
                        .or_else(|| self.contextual_expr_type_hint(right, bindings))
                }
            }
            _ => self.expr_type_hint(expression),
        }
    }

    pub(super) fn emit_callback(
        &mut self,
        callback: &Expr,
        available_params: &[Type],
        return_type: &Type,
    ) -> Result<(BasicValueEnum<'ctx>, Vec<Type>), String> {
        let arity = self.callback_arity(callback)?;
        if arity > available_params.len() {
            return Err(format!(
                "error: callback accepts {arity} parameters, but at most {} are available",
                available_params.len()
            ));
        }
        let params = available_params[..arity].to_vec();
        let callback_type = Type::Function {
            params: params.clone(),
            ret: Box::new(return_type.clone()),
        };
        let closure = match callback.kind() {
            ExprKind::ArrowFn {
                is_async,
                params: fn_params,
                ret_type,
                body,
                ..
            } => self.emit_contextual_arrow_fn(
                fn_params,
                ret_type,
                body,
                &params,
                return_type,
                *is_async,
                false,
            )?,
            _ => self.emit_expr_as(callback, &callback_type)?,
        };
        Ok((closure, params))
    }

    fn callback_arguments(
        &self,
        arity: usize,
        element: BasicValueEnum<'ctx>,
        index: IntValue<'ctx>,
        array: inkwell::values::StructValue<'ctx>,
    ) -> Vec<BasicValueEnum<'ctx>> {
        [element, index.into(), array.into()]
            .into_iter()
            .take(arity)
            .collect()
    }

    fn load_array_element(
        &self,
        view: &ArrayView<'ctx>,
        index: IntValue<'ctx>,
    ) -> BasicValueEnum<'ctx> {
        let slot = unsafe {
            self.builder
                .build_gep(
                    self.llvm_type(&view.element_type),
                    view.data,
                    &[index],
                    "functional_array_index",
                )
                .unwrap()
        };
        self.builder
            .build_load(
                self.llvm_type(&view.element_type),
                slot,
                "functional_array_value",
            )
            .unwrap()
    }

    fn empty_array_slot(
        &self,
        element: &Type,
        name: &str,
    ) -> (PointerValue<'ctx>, BasicTypeEnum<'ctx>) {
        let array_type = self.llvm_type(&Type::Array(Box::new(element.clone())));
        let slot = self
            .entry_alloca(array_type, name)
            .expect("a stack slot needs an enclosing function");
        self.builder
            .build_store(slot, array_type.const_zero())
            .unwrap();
        (slot, array_type)
    }

    fn push_array_value(
        &mut self,
        array: PointerValue<'ctx>,
        element_type: &Type,
        value: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        if matches!(transparent_codegen_type(element_type), Type::Str) {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let push = self.declare_libc_fn(
                "tyrus_array_push_owned_string",
                &[pointer.into(), self.string_abi_type()],
                Some(self.context.i64_type().into()),
            );
            self.builder
                .build_call(
                    push,
                    &[array.into(), value.into()],
                    "functional_array_push_string",
                )
                .unwrap();
            return Ok(());
        }
        let value = if !self.resolve_type(element_type).is_copy() {
            let source = self.entry_alloca(
                self.llvm_type(element_type),
                "functional_array_owned_source",
            )?;
            let cloned =
                self.entry_alloca(self.llvm_type(element_type), "functional_array_owned_clone")?;
            self.builder.build_store(source, value).unwrap();
            self.builder
                .build_store(cloned, self.llvm_type(element_type).const_zero())
                .unwrap();
            let clone_element = self.emit_array_element_clone(element_type)?;
            self.builder
                .build_call(
                    clone_element,
                    &[cloned.into(), source.into()],
                    "functional_array_clone_element",
                )
                .unwrap();
            self.builder
                .build_load(
                    self.llvm_type(element_type),
                    cloned,
                    "functional_array_cloned_element",
                )
                .map_err(|error| error.to_string())?
        } else {
            value
        };
        self.push_owned_array_value(array, element_type, value)
    }

    fn push_owned_array_value(
        &mut self,
        array: PointerValue<'ctx>,
        element_type: &Type,
        value: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let value_slot = self.entry_alloca(
            self.llvm_type(element_type),
            "functional_array_output_value",
        )?;
        self.builder.build_store(value_slot, value).unwrap();
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();
        let push = self.declare_libc_fn(
            "tyrus_array_push",
            &[pointer.into(), pointer.into(), i64_type.into()],
            Some(i64_type.into()),
        );
        let size = self
            .llvm_type(element_type)
            .size_of()
            .ok_or_else(|| format!("error: type `{element_type}` has no native size"))?;
        self.builder
            .build_call(push, &[array.into(), value_slot.into(), size.into()], "")
            .unwrap();
        Ok(())
    }

    fn reserve_array(
        &mut self,
        array: PointerValue<'ctx>,
        count: IntValue<'ctx>,
        element: &Type,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let size = self
            .llvm_type(element)
            .size_of()
            .ok_or_else(|| format!("error: type `{element}` has no native size"))?;
        let reserve = self.declare_libc_fn(
            "tyrus_array_reserve",
            &[
                pointer.into(),
                self.context.i64_type().into(),
                self.context.i64_type().into(),
            ],
            Some(self.context.i32_type().into()),
        );
        let status = self
            .builder
            .build_call(
                reserve,
                &[array.into(), count.into(), size.into()],
                "array_reserve",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        self.emit_abort_if_negative(status, "reserve", "array.reserve")
    }

    pub(super) fn emit_forward_loop<F>(
        &mut self,
        length: IntValue<'ctx>,
        label: &str,
        mut body: F,
    ) -> Result<(), String>
    where
        F: FnMut(&mut Self, IntValue<'ctx>) -> Result<(), String>,
    {
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: array method outside function")?;
        let index_type = self.context.i64_type();
        let index_slot = self
            .entry_alloca(index_type, &format!("{label}_index"))
            .expect("a stack slot needs an enclosing function");
        self.builder
            .build_store(index_slot, index_type.const_zero())
            .unwrap();
        let condition = self
            .context
            .append_basic_block(function, &format!("{label}.condition"));
        let loop_body = self
            .context
            .append_basic_block(function, &format!("{label}.body"));
        let done = self
            .context
            .append_basic_block(function, &format!("{label}.done"));
        self.builder.build_unconditional_branch(condition).unwrap();
        self.builder.position_at_end(condition);
        let index = self
            .builder
            .build_load(index_type, index_slot, "index")
            .unwrap()
            .into_int_value();
        let continues = self
            .builder
            .build_int_compare(
                IntPredicate::ULT,
                index,
                length,
                "array_iteration_continues",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(continues, loop_body, done)
            .unwrap();
        self.builder.position_at_end(loop_body);
        body(self, index)?;
        if !self.current_block_terminated() {
            let next = self
                .builder
                .build_int_add(index, index_type.const_int(1, false), "next_index")
                .unwrap();
            self.builder.build_store(index_slot, next).unwrap();
            self.builder.build_unconditional_branch(condition).unwrap();
        }
        self.builder.position_at_end(done);
        Ok(())
    }

    fn emit_scan(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let accumulator_type = self
            .expr_type_hint(&args[0])
            .ok_or("error: cannot infer scan accumulator type")?;
        let available = [accumulator_type.clone(), view.element_type.clone()];
        let (callback, callback_params) =
            self.emit_callback(&args[1], &available, &accumulator_type)?;
        let (output, output_type) = self.empty_array_slot(&accumulator_type, "scan_output");
        let accumulator = self
            .entry_alloca(self.llvm_type(&accumulator_type), "scan_accumulator")
            .unwrap();
        let initial = self.emit_expr_as(&args[0], &accumulator_type)?;
        self.builder.build_store(accumulator, initial).unwrap();
        let output_length = self
            .builder
            .build_int_add(
                view.length,
                self.context.i64_type().const_int(1, false),
                "scan_output_length",
            )
            .unwrap();
        self.reserve_array(output, output_length, &accumulator_type)?;
        self.push_array_value(output, &accumulator_type, initial)?;
        self.emit_forward_loop(view.length, "array.scan", |this, index| {
            let current = this
                .builder
                .build_load(
                    this.llvm_type(&accumulator_type),
                    accumulator,
                    "scan_current",
                )
                .unwrap();
            let element = this.load_array_element(view, index);
            let arguments = [current, element]
                .into_iter()
                .take(callback_params.len())
                .collect::<Vec<_>>();
            let next = this
                .emit_closure_call(callback, &callback_params, &accumulator_type, &arguments)?
                .ok_or("internal: scan callback returned void")?;
            this.builder.build_store(accumulator, next).unwrap();
            this.push_array_value(output, &accumulator_type, next)
        })?;
        self.release_synchronous_closure(callback, false)?;
        self.builder
            .build_load(output_type, output, "scan_result")
            .map_err(|error| error.to_string())
    }

    fn array_hash_field(
        &self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
        method: &str,
    ) -> Result<(usize, Type), String> {
        let key = match args.first().map(tyrus_core::ast::Expr::kind) {
            Some(ExprKind::StrLit(key)) => key,
            _ => {
                return Err(format!(
                    "error: `{method}` requires a string literal field name"
                ));
            }
        };
        let is_object = matches!(
            transparent_codegen_type(&view.element_type),
            Type::Object(_)
        ) || matches!(transparent_codegen_type(&view.element_type), Type::Named(name) if self.interfaces.contains_key(name));
        if !is_object {
            return Err(format!("error: `{method}` requires object elements"));
        }
        let index = self
            .struct_field_index(&view.element_type, key)
            .ok_or_else(|| format!("error: `{method}` key `{key}` is not a field"))?;
        let field_type = self
            .struct_field_type(&view.element_type, key)
            .expect("field index and field type must resolve together");
        Ok((index as usize, field_type))
    }

    fn hash_callback_expr(&self, expression: &Expr) -> Expr {
        if matches!(
            expression.kind(),
            ExprKind::ArrowFn { .. } | ExprKind::Ident(_)
        ) {
            return expression.clone();
        }
        let Some(name) = self.count_free_variable(expression) else {
            return expression.clone();
        };
        implicit_count_condition(&name, expression.clone())
    }

    fn emit_transpose(&mut self, view: &ArrayView<'ctx>) -> Result<BasicValueEnum<'ctx>, String> {
        let Type::Array(inner) = transparent_codegen_type(&view.element_type) else {
            return Err("error: `transpose` requires a two-dimensional array".to_string());
        };
        let output_element = Type::Array(inner.clone());
        let (output, output_type) = self.empty_array_slot(&output_element, "transpose_output");
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: transpose outside function")?;
        let no_rows = self.context.append_basic_block(function, "transpose.empty");
        let rows_body = self.context.append_basic_block(function, "transpose.rows");
        let done = self.context.append_basic_block(function, "transpose.done");
        let has_rows = self
            .builder
            .build_int_compare(
                IntPredicate::UGT,
                view.length,
                self.context.i64_type().const_zero(),
                "transpose_has_rows",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(has_rows, rows_body, no_rows)
            .unwrap();
        self.builder.position_at_end(no_rows);
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(rows_body);
        let first_row = self
            .load_array_element(view, self.context.i64_type().const_zero())
            .into_struct_value();
        let columns = self
            .builder
            .build_extract_value(first_row, 1, "transpose_columns")
            .map_err(|error| error.to_string())?
            .into_int_value();
        self.reserve_array(output, columns, &output_element)?;
        self.emit_forward_loop(view.length, "array.transpose.validate", |this, row| {
            let row_value = this.load_array_element(view, row).into_struct_value();
            let row_length = this
                .builder
                .build_extract_value(row_value, 1, "transpose_row_length")
                .map_err(|error| error.to_string())?
                .into_int_value();
            let rectangular = this
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    row_length,
                    columns,
                    "transpose_rectangular",
                )
                .unwrap();
            let function = this
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
                .ok_or("internal: transpose validation outside function")?;
            let error = this
                .context
                .append_basic_block(function, "transpose.invalid");
            let valid = this.context.append_basic_block(function, "transpose.valid");
            this.builder
                .build_conditional_branch(rectangular, valid, error)
                .unwrap();
            this.builder.position_at_end(error);
            let message = this
                .builder
                .build_global_string_ptr(
                    "error: transpose requires rows with equal length\n",
                    "transpose_error",
                )
                .unwrap();
            this.builder
                .build_call(
                    this.declare_dprintf(),
                    &[
                        this.context.i32_type().const_int(2, false).into(),
                        message.as_pointer_value().into(),
                    ],
                    "",
                )
                .unwrap();
            this.emit_exit_failure();
            this.builder.position_at_end(valid);
            Ok(())
        })?;
        self.emit_forward_loop(columns, "array.transpose.columns", |this, column| {
            let (column_output, column_type) = this.empty_array_slot(inner, "transpose_column");
            this.reserve_array(column_output, view.length, inner)?;
            this.emit_forward_loop(view.length, "array.transpose.rows", |this, row| {
                let row_value = this.load_array_element(view, row).into_struct_value();
                let row_data = this
                    .builder
                    .build_extract_value(row_value, 0, "transpose_row_data")
                    .map_err(|error| error.to_string())?
                    .into_pointer_value();
                let element_slot = unsafe {
                    this.builder
                        .build_gep(
                            this.llvm_type(inner),
                            row_data,
                            &[column],
                            "transpose_element_slot",
                        )
                        .unwrap()
                };
                let element = this
                    .builder
                    .build_load(this.llvm_type(inner), element_slot, "transpose_element")
                    .unwrap();
                this.push_array_value(column_output, inner, element)
            })?;
            let column_value = this
                .builder
                .build_load(column_type, column_output, "transpose_column_value")
                .map_err(|error| error.to_string())?;
            this.push_owned_array_value(output, &output_element, column_value)
        })?;
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(done);
        self.builder
            .build_load(output_type, output, "transpose_result")
            .map_err(|error| error.to_string())
    }

    fn emit_for_each(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let available = [
            view.element_type.clone(),
            Type::Usize,
            view.array_type.clone(),
        ];
        let (callback, callback_params) = self.emit_callback(&args[0], &available, &Type::Void)?;
        self.emit_forward_loop(view.length, "array.forEach", |this, index| {
            let element = this.load_array_element(view, index);
            let arguments =
                this.callback_arguments(callback_params.len(), element, index, view.value);
            this.emit_closure_call(callback, &callback_params, &Type::Void, &arguments)?;
            Ok(())
        })?;
        self.release_synchronous_closure(callback, false)?;
        Ok(self.context.i32_type().const_zero().into())
    }

    fn emit_find_variant(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
        reverse: bool,
        return_index: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let available = [
            view.element_type.clone(),
            Type::Usize,
            view.array_type.clone(),
        ];
        let predicate = self.hash_callback_expr(&args[0]);
        let (callback, callback_params) =
            self.emit_callback(&predicate, &available, &Type::Bool)?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();
        let result_type: BasicTypeEnum<'ctx> = if return_index {
            i64_type.into()
        } else {
            pointer.into()
        };
        let result = self.entry_alloca(result_type, "find_result").unwrap();
        let initial_result: BasicValueEnum<'ctx> = if return_index {
            i64_type.const_int(u64::MAX, true).into()
        } else {
            pointer.const_null().into()
        };
        self.builder.build_store(result, initial_result).unwrap();
        let function = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();
        let done = self.context.append_basic_block(function, "array.find.done");
        let condition = self.context.append_basic_block(function, "find.condition");
        let body = self.context.append_basic_block(function, "find.body");
        let found = self.context.append_basic_block(function, "find.found");
        let advance = self.context.append_basic_block(function, "find.advance");
        let index_slot = self.entry_alloca(i64_type, "find_index").unwrap();
        let start = if reverse {
            self.builder
                .build_int_sub(view.length, i64_type.const_int(1, false), "find_last_start")
                .unwrap()
        } else {
            i64_type.const_zero()
        };
        self.builder.build_store(index_slot, start).unwrap();
        self.builder.build_unconditional_branch(condition).unwrap();
        self.builder.position_at_end(condition);
        let index = self
            .builder
            .build_load(i64_type, index_slot, "find_index")
            .unwrap()
            .into_int_value();
        let continues = if reverse {
            self.builder
                .build_int_compare(
                    IntPredicate::SGE,
                    index,
                    i64_type.const_zero(),
                    "find_continues",
                )
                .unwrap()
        } else {
            self.builder
                .build_int_compare(IntPredicate::ULT, index, view.length, "find_continues")
                .unwrap()
        };
        self.builder
            .build_conditional_branch(continues, body, done)
            .unwrap();
        self.builder.position_at_end(body);
        let element = self.load_array_element(view, index);
        let arguments = self.callback_arguments(callback_params.len(), element, index, view.value);
        let matches = self
            .emit_closure_call(callback, &callback_params, &Type::Bool, &arguments)?
            .unwrap()
            .into_int_value();
        self.builder
            .build_conditional_branch(matches, found, advance)
            .unwrap();
        self.builder.position_at_end(found);
        if return_index {
            self.builder.build_store(result, index).unwrap();
        } else {
            let size = self.llvm_type(&view.element_type).size_of().unwrap();
            let (malloc, size) = self.declare_malloc(size);
            let storage = self
                .builder
                .build_call(malloc, &[size.into()], "find_storage")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_pointer_value();
            self.builder.build_store(storage, element).unwrap();
            self.builder.build_store(result, storage).unwrap();
        }
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(advance);
        let next = if reverse {
            self.builder
                .build_int_sub(index, i64_type.const_int(1, false), "find_previous")
                .unwrap()
        } else {
            self.builder
                .build_int_add(index, i64_type.const_int(1, false), "find_next")
                .unwrap()
        };
        self.builder.build_store(index_slot, next).unwrap();
        self.builder.build_unconditional_branch(condition).unwrap();
        self.builder.position_at_end(done);
        self.release_synchronous_closure(callback, false)?;
        self.builder
            .build_load(result_type, result, "find_result")
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_keyed_aggregate(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
        average: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let key = match args.first().map(tyrus_core::ast::Expr::kind) {
            Some(ExprKind::StrLit(key)) => key,
            _ => {
                return Err(
                    "error: keyed array aggregation requires a string literal field name"
                        .to_string(),
                );
            }
        };
        let is_object = matches!(
            transparent_codegen_type(&view.element_type),
            Type::Object(_)
        ) || matches!(transparent_codegen_type(&view.element_type), Type::Named(name) if self.interfaces.contains_key(name));
        if !is_object {
            return Err(format!(
                "error: keyed array aggregation requires object elements, found `{}`",
                view.element_type
            ));
        }
        let field_index = self
            .struct_field_index(&view.element_type, key)
            .ok_or_else(|| {
                format!(
                    "error: aggregate key `{key}` is not a field of `{}`",
                    view.element_type
                )
            })?;
        let field_type = self
            .struct_field_type(&view.element_type, key)
            .expect("field index and field type must resolve together");
        let field_llvm = self.llvm_type(&field_type);

        let data = view.data;
        let length = view.length;

        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: keyed aggregate outside function")?;

        let accumulator = self.builder.build_alloca(field_llvm, "agg_acc").unwrap();
        let initial: BasicValueEnum<'ctx> = if field_llvm.is_float_type() {
            field_llvm.into_float_type().const_float(0.0).into()
        } else {
            field_llvm.into_int_type().const_int(0, false).into()
        };
        self.builder.build_store(accumulator, initial).unwrap();

        let index = self
            .builder
            .build_alloca(self.context.i64_type(), "agg_index")
            .unwrap();
        self.builder
            .build_store(index, self.context.i64_type().const_zero())
            .unwrap();

        let cond_block = self.context.append_basic_block(function, "agg_cond");
        let body_block = self.context.append_basic_block(function, "agg_body");
        let done_block = self.context.append_basic_block(function, "agg_done");

        self.builder.build_unconditional_branch(cond_block).unwrap();
        self.builder.position_at_end(cond_block);
        let cur = self
            .builder
            .build_load(self.context.i64_type(), index, "agg_idx")
            .unwrap()
            .into_int_value();
        let more = self
            .builder
            .build_int_compare(IntPredicate::ULT, cur, length, "agg_more")
            .unwrap();
        self.builder
            .build_conditional_branch(more, body_block, done_block)
            .unwrap();

        self.builder.position_at_end(body_block);
        let elem_ptr = unsafe {
            self.builder.build_gep(
                self.llvm_type(&view.element_type),
                data,
                &[cur],
                "agg_elem_ptr",
            )
        }
        .map_err(|e| e.to_string())?;
        let elem = self
            .builder
            .build_load(self.llvm_type(&view.element_type), elem_ptr, "agg_elem")
            .map_err(|e| e.to_string())?;
        let field_val = self
            .builder
            .build_extract_value(elem.into_struct_value(), field_index, "agg_field")
            .map_err(|e| e.to_string())?;

        let cur_acc = self
            .builder
            .build_load(field_llvm, accumulator, "agg_cur_acc")
            .map_err(|e| e.to_string())?;
        let next_acc: BasicValueEnum<'ctx> = if field_llvm.is_float_type() {
            self.builder
                .build_float_add(
                    cur_acc.into_float_value(),
                    field_val.into_float_value(),
                    "agg_next",
                )
                .unwrap()
                .into()
        } else {
            self.builder
                .build_int_add(
                    cur_acc.into_int_value(),
                    field_val.into_int_value(),
                    "agg_next",
                )
                .unwrap()
                .into()
        };
        self.builder.build_store(accumulator, next_acc).unwrap();

        let next_idx = self
            .builder
            .build_int_add(
                cur,
                self.context.i64_type().const_int(1, false),
                "agg_next_idx",
            )
            .unwrap();
        self.builder.build_store(index, next_idx).unwrap();
        self.builder.build_unconditional_branch(cond_block).unwrap();

        self.builder.position_at_end(done_block);
        let final_acc = self
            .builder
            .build_load(field_llvm, accumulator, "agg_final")
            .map_err(|e| e.to_string())?;

        if average {
            let float_acc = if field_llvm.is_float_type() {
                final_acc.into_float_value()
            } else {
                self.builder
                    .build_signed_int_to_float(
                        final_acc.into_int_value(),
                        self.context.f64_type(),
                        "agg_to_float",
                    )
                    .unwrap()
            };
            let len_f64 = self
                .builder
                .build_signed_int_to_float(length, self.context.f64_type(), "agg_len_f64")
                .unwrap();
            let avg = self
                .builder
                .build_float_div(float_acc, len_f64, "agg_avg")
                .unwrap();
            let boxed = self.emit_optional_box(avg.into(), &Type::F64)?;
            return Ok(boxed.into());
        }

        Ok(final_acc)
    }
}

fn implicit_count_condition(free_name: &str, body: Expr) -> Expr {
    let param = Param {
        decorators: Vec::new(),
        pattern: ParamPattern::Name(free_name.to_string()),
        ty: Type::Infer,
        mutable: false,
        optional: false,
        default_value: None,
        rest: false,
        span: tyrus_core::span::Span::DUMMY,
    };
    Expr::synthetic(ExprKind::ArrowFn {
        is_async: false,
        type_params: Vec::new(),
        params: vec![param],
        ret_type: Type::Infer,
        throws: Vec::new(),
        body: vec![Stmt::synthetic(StmtKind::Return(Some(body)))],
    })
}

fn flatten_array_element(element: &Type, depth: usize) -> Type {
    if depth == 0 {
        return element.clone();
    }
    match element {
        Type::Array(inner) => flatten_array_element(inner, depth - 1),
        other => other.clone(),
    }
}
