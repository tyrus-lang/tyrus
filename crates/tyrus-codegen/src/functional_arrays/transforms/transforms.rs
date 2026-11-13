use super::*;

#[path = "merge_sort/merge_sort.rs"]
mod merge_sort;
use merge_sort::{MergeSort, MergeSortBlocks};

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_map(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
        expected: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let inferred_output;
        let result_type = if let Some(expected) = expected {
            expected
        } else {
            inferred_output = Type::Array(Box::new(
                self.infer_callback_return_type(
                    &args[0],
                    &[
                        view.element_type.clone(),
                        Type::Usize,
                        view.array_type.clone(),
                    ],
                )
                .ok_or("error: cannot infer `map` callback return type")?,
            ));
            &inferred_output
        };
        let Type::Array(output_element) = result_type else {
            return Err("error: `map` result must be an array".to_string());
        };
        let available = [
            view.element_type.clone(),
            Type::Usize,
            view.array_type.clone(),
        ];
        let (callback, callback_params) =
            self.emit_callback(&args[0], &available, output_element)?;
        let (output, output_type) = self.empty_array_slot(output_element, "map_output");
        self.emit_forward_loop(view.length, "array.map", |this, index| {
            let element = this.load_array_element(view, index);
            let arguments =
                this.callback_arguments(callback_params.len(), element, index, view.value);
            let mapped = this
                .emit_closure_call(callback, &callback_params, output_element, &arguments)?
                .ok_or("internal: map callback returned void")?;
            this.push_array_value(output, output_element, mapped)?;
            if !this.resolve_type(output_element).is_copy() {
                this.emit_owned_native_release(mapped, output_element, &crate::ReleasePlan::Deep);
            }
            Ok(())
        })?;
        self.release_synchronous_closure(callback, false)?;
        self.builder
            .build_load(output_type, output, "map_result")
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_filter(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let available = [
            view.element_type.clone(),
            Type::Usize,
            view.array_type.clone(),
        ];
        let predicate = self.hash_callback_expr(&args[0]);
        let (callback, callback_params) =
            self.emit_callback(&predicate, &available, &Type::Bool)?;
        let (output, output_type) = self.empty_array_slot(&view.element_type, "filter_output");
        self.emit_forward_loop(view.length, "array.filter", |this, index| {
            let element = this.load_array_element(view, index);
            let arguments =
                this.callback_arguments(callback_params.len(), element, index, view.value);
            let keep = this
                .emit_closure_call(callback, &callback_params, &Type::Bool, &arguments)?
                .unwrap()
                .into_int_value();
            let function = this
                .builder
                .get_insert_block()
                .unwrap()
                .get_parent()
                .unwrap();
            let append = this.context.append_basic_block(function, "filter.append");
            let resume = this.context.append_basic_block(function, "filter.resume");
            this.builder
                .build_conditional_branch(keep, append, resume)
                .unwrap();
            this.builder.position_at_end(append);
            this.push_array_value(output, &view.element_type, element)?;
            this.builder.build_unconditional_branch(resume).unwrap();
            this.builder.position_at_end(resume);
            Ok(())
        })?;
        self.release_synchronous_closure(callback, false)?;
        self.builder
            .build_load(output_type, output, "filter_result")
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_partition(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let field = matches!(
            args.first().map(tyrus_core::ast::Expr::kind),
            Some(ExprKind::StrLit(_))
        )
        .then(|| self.array_hash_field(view, args, "partition"))
        .transpose()?;
        let match_values = field
            .as_ref()
            .filter(|_| args.len() > 1)
            .map(|(_, field_type)| {
                args[1..]
                    .iter()
                    .map(|value| self.emit_expr_as(value, field_type))
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        let available = [
            view.element_type.clone(),
            Type::Usize,
            view.array_type.clone(),
        ];
        let callback = field
            .is_none()
            .then(|| {
                let predicate = self.count_free_variable(&args[0]).map_or_else(
                    || args[0].clone(),
                    |name| implicit_count_condition(&name, args[0].clone()),
                );
                self.emit_callback(&predicate, &available, &Type::Bool)
            })
            .transpose()?;
        let (accepted, accepted_type) = self.empty_array_slot(&view.element_type, "partition_true");
        let (rejected, rejected_type) =
            self.empty_array_slot(&view.element_type, "partition_false");
        self.emit_forward_loop(view.length, "array.partition", |this, index| {
            let element = this.load_array_element(view, index);
            let keep = if let Some((field_index, _)) = &field {
                let field_value = this
                    .builder
                    .build_extract_value(
                        element.into_struct_value(),
                        *field_index as u32,
                        "partition_field",
                    )
                    .map_err(|error| error.to_string())?;
                if match_values.is_empty() {
                    field_value.into_int_value()
                } else {
                    let mut matches = this.context.bool_type().const_zero();
                    for (offset, expected) in match_values.iter().enumerate() {
                        let equal = this
                            .emit_value_equality(
                                field_value,
                                *expected,
                                &args[0],
                                &args[offset + 1],
                            )?
                            .into_int_value();
                        matches = this
                            .builder
                            .build_or(matches, equal, "partition_match")
                            .unwrap();
                    }
                    matches
                }
            } else {
                let (callback, callback_params) = callback.as_ref().unwrap();
                let arguments =
                    this.callback_arguments(callback_params.len(), element, index, view.value);
                this.emit_closure_call(*callback, callback_params, &Type::Bool, &arguments)?
                    .unwrap()
                    .into_int_value()
            };
            let function = this
                .builder
                .get_insert_block()
                .unwrap()
                .get_parent()
                .unwrap();
            let accepted_block = this
                .context
                .append_basic_block(function, "partition.accepted");
            let rejected_block = this
                .context
                .append_basic_block(function, "partition.rejected");
            let resume = this
                .context
                .append_basic_block(function, "partition.resume");
            this.builder
                .build_conditional_branch(keep, accepted_block, rejected_block)
                .unwrap();
            this.builder.position_at_end(accepted_block);
            this.push_array_value(accepted, &view.element_type, element)?;
            this.builder.build_unconditional_branch(resume).unwrap();
            this.builder.position_at_end(rejected_block);
            this.push_array_value(rejected, &view.element_type, element)?;
            this.builder.build_unconditional_branch(resume).unwrap();
            this.builder.position_at_end(resume);
            Ok(())
        })?;
        if let Some((callback, _)) = callback {
            self.release_synchronous_closure(callback, false)?;
        }
        let accepted_value = self
            .builder
            .build_load(accepted_type, accepted, "partition_true_result")
            .map_err(|error| error.to_string())?;
        let rejected_value = self
            .builder
            .build_load(rejected_type, rejected, "partition_false_result")
            .map_err(|error| error.to_string())?;
        let array_type = Type::Array(Box::new(view.element_type.clone()));
        let tuple_type = Type::Tuple(vec![array_type.clone(), array_type]);
        let tuple_llvm = self.llvm_type(&tuple_type).into_struct_type();
        let tuple = tuple_llvm.get_undef();
        let tuple = self
            .builder
            .build_insert_value(tuple, accepted_value, 0, "partition_tuple")
            .map_err(|error| error.to_string())?;
        let tuple = self
            .builder
            .build_insert_value(tuple, rejected_value, 1, "partition_tuple")
            .map_err(|error| error.to_string())?;
        Ok(tuple.into_struct_value().into())
    }

    pub(super) fn emit_windows(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let size = self.emit_expr_as(&args[0], &Type::Usize)?.into_int_value();
        let output_element = Type::Array(Box::new(view.element_type.clone()));
        let (output, output_type) = self.empty_array_slot(&output_element, "windows_output");
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: windows outside function")?;
        let invalid = self.context.append_basic_block(function, "windows.invalid");
        let check_length = self
            .context
            .append_basic_block(function, "windows.check.length");
        let too_large = self
            .context
            .append_basic_block(function, "windows.too.large");
        let body = self.context.append_basic_block(function, "windows.body");
        let done = self.context.append_basic_block(function, "windows.done");
        let nonzero = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                size,
                self.context.i64_type().const_zero(),
                "windows_size_nonzero",
            )
            .unwrap();
        self.builder
            .build_conditional_branch(nonzero, check_length, invalid)
            .unwrap();
        self.builder.position_at_end(invalid);
        let message = self
            .builder
            .build_global_string_ptr(
                "error: windows size must be greater than zero\n",
                "windows_error",
            )
            .unwrap();
        self.builder
            .build_call(
                self.declare_dprintf(),
                &[
                    self.context.i32_type().const_int(2, false).into(),
                    message.as_pointer_value().into(),
                ],
                "",
            )
            .unwrap();
        self.emit_exit_failure();
        self.builder.position_at_end(check_length);
        let valid = self
            .builder
            .build_int_compare(IntPredicate::ULE, size, view.length, "windows_size_valid")
            .unwrap();
        self.builder
            .build_conditional_branch(valid, body, too_large)
            .unwrap();
        self.builder.position_at_end(too_large);
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(body);
        let count = self
            .builder
            .build_int_sub(view.length, size, "windows_count_base")
            .unwrap();
        let count = self
            .builder
            .build_int_add(
                count,
                self.context.i64_type().const_int(1, false),
                "windows_count",
            )
            .unwrap();
        self.reserve_array(output, count, &output_element)?;
        // One slot reused by every iteration. An `alloca` inside the loop body
        // runs once per window, so the stack grew with the input instead of
        // staying flat. Nothing reads the slot before the slice below fills it:
        // `tyrus_array_slice` overwrites the whole `TyrusArray` and allocates
        // the window's buffer itself. Reserving capacity here first was both
        // unsound, `tyrus_array_reserve` reads `data`/`capacity`, and a fresh
        // `alloca` holds whatever the stack left behind, so it called `realloc`
        // on a garbage pointer, and a leak, since the slice replaced the
        // buffer it had just allocated.
        let window_type = self.llvm_type(&output_element);
        let window = self.entry_alloca(window_type, "windows_window").unwrap();
        self.emit_forward_loop(count, "array.windows", |this, index| {
            let end = this
                .builder
                .build_int_add(index, size, "windows_end")
                .unwrap();
            let pointer = this.context.ptr_type(inkwell::AddressSpace::default());
            let i64_type = this.context.i64_type();
            let i32_type = this.context.i32_type();
            let owned_strings = matches!(transparent_codegen_type(&view.element_type), Type::Str);
            let element_size = this.llvm_type(&view.element_type).size_of().unwrap();
            let (slice, arguments) = if owned_strings {
                let slice = this.declare_libc_fn(
                    "tyrus_array_slice_owned_strings",
                    &[
                        pointer.into(),
                        i64_type.into(),
                        i64_type.into(),
                        pointer.into(),
                    ],
                    Some(i32_type.into()),
                );
                (
                    slice,
                    vec![view.slot.into(), index.into(), end.into(), window.into()],
                )
            } else {
                let slice = this.declare_libc_fn(
                    "tyrus_array_slice",
                    &[
                        pointer.into(),
                        i64_type.into(),
                        i64_type.into(),
                        i64_type.into(),
                        pointer.into(),
                    ],
                    Some(i32_type.into()),
                );
                (
                    slice,
                    vec![
                        view.slot.into(),
                        index.into(),
                        end.into(),
                        element_size.into(),
                        window.into(),
                    ],
                )
            };
            let status = this
                .builder
                .build_call(slice, &arguments, "windows_slice")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_int_value();
            this.emit_abort_if_negative(status, "windows", "array.windows")?;
            let value = this
                .builder
                .build_load(window_type, window, "windows_value")
                .unwrap();
            this.push_owned_array_value(output, &output_element, value)
        })?;
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(done);
        self.builder
            .build_load(output_type, output, "windows_result")
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_unique(
        &mut self,
        view: &ArrayView<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let output_type = self.llvm_type(&Type::Array(Box::new(view.element_type.clone())));
        let output = self.entry_alloca(output_type, "unique_output").unwrap();
        self.builder
            .build_store(output, output_type.const_zero())
            .unwrap();
        let function = self.declare_libc_fn(
            "tyrus_array_unique_values",
            &[
                pointer.into(),
                pointer.into(),
                self.context.i64_type().into(),
                self.context.i32_type().into(),
            ],
            Some(self.context.i32_type().into()),
        );
        let element_size = self
            .llvm_type(&view.element_type)
            .size_of()
            .ok_or_else(|| format!("error: type `{}` has no native size", view.element_type))?;
        let status = self
            .builder
            .build_call(
                function,
                &[
                    view.slot.into(),
                    output.into(),
                    element_size.into(),
                    self.context
                        .i32_type()
                        .const_int(self.array_element_kind(&view.element_type), false)
                        .into(),
                ],
                "unique_status",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        self.emit_abort_if_negative(status, "unique", "array.unique")?;
        self.builder
            .build_load(output_type, output, "unique_result")
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_unique_by(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let field_index = matches!(
            args.first().map(tyrus_core::ast::Expr::kind),
            Some(ExprKind::StrLit(_))
        )
        .then(|| self.array_hash_field(view, args, "uniqueBy"))
        .transpose()?;
        let callback_expr = field_index
            .is_none()
            .then(|| self.hash_callback_expr(&args[0]));
        let available = [
            view.element_type.clone(),
            Type::Usize,
            view.array_type.clone(),
        ];
        let key_type = field_index
            .as_ref()
            .map(|(_, key_type)| key_type.clone())
            .or_else(|| {
                callback_expr
                    .as_ref()
                    .and_then(|callback| self.infer_callback_return_type(callback, &available))
            })
            .ok_or("error: cannot infer uniqueBy key type")?;
        let callback = callback_expr
            .as_ref()
            .map(|callback| self.emit_callback(callback, &available, &key_type))
            .transpose()?;
        let map = self.emit_map_new(&key_type, &Type::Bool)?;
        let (output, output_type) = self.empty_array_slot(&view.element_type, "unique_by_output");
        self.reserve_array(output, view.length, &view.element_type)?;
        let pointer = self.collection_ptr_type();
        let has = self.declare_libc_fn(
            "tyrus_map_has",
            &[pointer.into(), pointer.into()],
            Some(self.context.bool_type().into()),
        );
        let insert = self.declare_libc_fn(
            "tyrus_map_insert",
            &[pointer.into(), pointer.into(), pointer.into()],
            None,
        );
        self.emit_forward_loop(view.length, "array.unique_by", |this, index| {
            let element = this.load_array_element(view, index);
            let key = if let Some((callback, callback_params)) = &callback {
                let arguments =
                    this.callback_arguments(callback_params.len(), element, index, view.value);
                this.emit_closure_call(*callback, callback_params, &key_type, &arguments)?
                    .ok_or("internal: uniqueBy callback returned void")?
            } else {
                this.builder
                    .build_extract_value(
                        element.into_struct_value(),
                        field_index.as_ref().unwrap().0 as u32,
                        "unique_by_key",
                    )
                    .map_err(|error| error.to_string())?
            };
            let key_ptr = this.emit_value_slot(key, &key_type);
            let present = this
                .builder
                .build_call(has, &[map.into(), key_ptr.into()], "unique_by_present")
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_int_value();
            let function = this
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
                .ok_or("internal: uniqueBy outside function")?;
            let skip = this.context.append_basic_block(function, "unique_by.skip");
            let take = this.context.append_basic_block(function, "unique_by.take");
            this.builder
                .build_conditional_branch(present, skip, take)
                .unwrap();
            this.builder.position_at_end(take);
            this.push_array_value(output, &view.element_type, element)?;
            let value = this.context.bool_type().const_int(1, false);
            let value_ptr = this.emit_value_slot(value.into(), &Type::Bool);
            this.builder
                .build_call(
                    insert,
                    &[map.into(), key_ptr.into(), value_ptr.into()],
                    "unique_by_insert",
                )
                .unwrap();
            this.builder.build_unconditional_branch(skip).unwrap();
            this.builder.position_at_end(skip);
            Ok(())
        })?;
        if let Some((callback, _)) = callback {
            self.release_synchronous_closure(callback, false)?;
        }
        let free = self.declare_libc_fn("tyrus_map_free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[map.into()], "unique_by_free")
            .unwrap();
        self.builder
            .build_load(output_type, output, "unique_by_result")
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_group_by(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let field_index = matches!(
            args.first().map(tyrus_core::ast::Expr::kind),
            Some(ExprKind::StrLit(_))
        )
        .then(|| self.array_hash_field(view, args, "groupBy"))
        .transpose()?;
        let callback_expr = field_index
            .is_none()
            .then(|| self.hash_callback_expr(&args[0]));
        let available = [
            view.element_type.clone(),
            Type::Usize,
            view.array_type.clone(),
        ];
        let key_type = field_index
            .as_ref()
            .map(|(_, key_type)| key_type.clone())
            .or_else(|| {
                callback_expr
                    .as_ref()
                    .and_then(|callback| self.infer_callback_return_type(callback, &available))
            })
            .ok_or("error: cannot infer groupBy key type")?;
        let callback = callback_expr
            .as_ref()
            .map(|callback| self.emit_callback(callback, &available, &key_type))
            .transpose()?;
        let pointer = self.collection_ptr_type();
        let key_size = self
            .llvm_type(&key_type)
            .size_of()
            .map(|size| self.truncate_to_usize(size))
            .ok_or("error: groupBy key has no native size")?;
        let element_size = self
            .llvm_type(&view.element_type)
            .size_of()
            .map(|size| self.truncate_to_usize(size))
            .ok_or("error: groupBy element has no native size")?;
        let usize_type = self.usize_type();
        let new = self.declare_libc_fn(
            "tyrus_map_new_grouped",
            &[
                usize_type.into(),
                self.context.i32_type().into(),
                usize_type.into(),
            ],
            Some(pointer.into()),
        );
        let map = self
            .builder
            .build_call(
                new,
                &[
                    key_size.into(),
                    self.context
                        .i32_type()
                        .const_int(self.array_element_kind(&key_type), false)
                        .into(),
                    element_size.into(),
                ],
                "group_by_map",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_pointer_value();
        let push = self.declare_libc_fn(
            "tyrus_map_group_push",
            &[pointer.into(), pointer.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        self.emit_forward_loop(view.length, "array.group_by", |this, index| {
            let element = this.load_array_element(view, index);
            let key = if let Some((callback, callback_params)) = &callback {
                let arguments =
                    this.callback_arguments(callback_params.len(), element, index, view.value);
                this.emit_closure_call(*callback, callback_params, &key_type, &arguments)?
                    .ok_or("internal: groupBy callback returned void")?
            } else {
                this.builder
                    .build_extract_value(
                        element.into_struct_value(),
                        field_index.as_ref().unwrap().0 as u32,
                        "group_by_key",
                    )
                    .map_err(|error| error.to_string())?
            };
            let key_ptr = this.emit_value_slot(key, &key_type);
            let value_ptr = this.emit_value_slot(element, &view.element_type);
            let status = this
                .builder
                .build_call(
                    push,
                    &[map.into(), key_ptr.into(), value_ptr.into()],
                    "group_by_push",
                )
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap()
                .into_int_value();
            this.emit_abort_if_negative(status, "groupBy", "array.groupBy")
        })?;
        if let Some((callback, _)) = callback {
            self.release_synchronous_closure(callback, false)?;
        }
        self.record_owned_native_result(
            map.into(),
            Type::Map(
                Box::new(key_type),
                Box::new(Type::Array(Box::new(view.element_type.clone()))),
            ),
        );
        Ok(map.into())
    }

    pub(super) fn emit_zip(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let other = self.functional_array_view(&args[0])?;
        let tuple_element =
            Type::Tuple(vec![view.element_type.clone(), other.element_type.clone()]);
        let (output, output_type) = self.empty_array_slot(&tuple_element, "zip_output");
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: zip outside function")?;
        let same_length = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                view.length,
                other.length,
                "zip.same_length",
            )
            .map_err(|error| error.to_string())?;
        let mismatch = self.context.append_basic_block(function, "zip.mismatch");
        let proceed = self.context.append_basic_block(function, "zip.proceed");
        self.builder
            .build_conditional_branch(same_length, proceed, mismatch)
            .map_err(|error| error.to_string())?;

        self.builder.position_at_end(mismatch);
        self.emit_zip_mismatch_error(function)?;

        self.builder.position_at_end(proceed);
        self.reserve_array(output, view.length, &tuple_element)?;
        self.emit_forward_loop(view.length, "array.zip", |this, index| {
            let left = this.load_array_element(view, index);
            let right = this.load_array_element(&other, index);
            let tuple_type = this.llvm_type(&tuple_element).into_struct_type();
            let tuple = tuple_type.get_undef();
            let tuple = this
                .builder
                .build_insert_value(tuple, left, 0, "zip_left")
                .map_err(|error| error.to_string())?;
            let tuple = this
                .builder
                .build_insert_value(tuple, right, 1, "zip_right")
                .map_err(|error| error.to_string())?;
            this.push_array_value(output, &tuple_element, tuple.into_struct_value().into())
        })?;
        self.builder
            .build_load(output_type, output, "zip_result")
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_zip_mismatch_error(
        &mut self,
        function: inkwell::values::FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let code = self.const_string_abi("ERR_ARRAY_ZIP_MISMATCH", "zip.error.code");
        let message = self.const_string_abi(
            "zip: arrays passed to zip must have the same length",
            "zip.error.message",
        );
        let error_type = Type::Named("ZipError".to_string());
        let error = self
            .builder
            .build_insert_value(
                self.llvm_type(&error_type).into_struct_type().const_zero(),
                code,
                0,
                "zip.error.value.code",
            )
            .map_err(|error| error.to_string())?;
        let error = self
            .builder
            .build_insert_value(error, message, 1, "zip.error.value.message")
            .map_err(|error| error.to_string())?
            .into_struct_value();
        let async_throw = self.async_results.last().copied().map(|promise| {
            (
                promise,
                super::statements::AsyncRejection::Structured {
                    code_index: 0,
                    message_index: 1,
                },
            )
        });
        self.emit_recoverable_runtime_error_cleanups()?;
        self.emit_throw_value_in_async(
            error.into(),
            super::statements::ThrownValue::Coded("ZipError"),
            function,
            async_throw,
            self.context.bool_type().const_zero(),
            &Type::Named("ZipError".to_string()),
        )?;
        Ok(())
    }

    pub(super) fn emit_sort(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
        copy: bool,
        keyed: Option<(usize, Type, bool)>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        self.emit_stable_merge_sort(view, args, copy, keyed)
    }

    pub(super) fn emit_stable_merge_sort(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
        copy: bool,
        keyed: Option<(usize, Type, bool)>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let target = if copy {
            let output = self
                .entry_alloca(view.llvm_array_type, "sorted_output")
                .unwrap();
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let i64_type = self.context.i64_type();
            let i32_type = self.context.i32_type();
            let slice = self.declare_libc_fn(
                "tyrus_array_slice",
                &[
                    pointer.into(),
                    i64_type.into(),
                    i64_type.into(),
                    i64_type.into(),
                    pointer.into(),
                ],
                Some(i32_type.into()),
            );
            let size = self.llvm_type(&view.element_type).size_of().unwrap();
            self.builder
                .build_call(
                    slice,
                    &[
                        view.slot.into(),
                        i64_type.const_zero().into(),
                        view.length.into(),
                        size.into(),
                        output.into(),
                    ],
                    "array_to_sorted_copy",
                )
                .unwrap();
            output
        } else {
            view.slot
        };
        let target_value = self
            .builder
            .build_load(view.llvm_array_type, target, "stable_sort_array")
            .unwrap()
            .into_struct_value();
        let data = self
            .builder
            .build_extract_value(target_value, 0, "stable_sort_data")
            .unwrap()
            .into_pointer_value();
        let element_type = self.llvm_type(&view.element_type);
        let element_size = element_type
            .size_of()
            .ok_or_else(|| format!("error: type `{}` has no native size", view.element_type))?;
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let allocation_size = self
            .builder
            .build_int_mul(view.length, element_size, "stable_sort_buffer_size")
            .unwrap();
        let (malloc, allocation_size) = self.declare_malloc(allocation_size);
        let scratch = self
            .builder
            .build_call(malloc, &[allocation_size.into()], "stable_sort_scratch")
            .unwrap()
            .try_as_basic_value()
            .basic()
            .ok_or("internal: stable sort malloc returned void")?
            .into_pointer_value();
        let callback = if let Some(callback) = args.first() {
            Some(self.emit_callback(
                callback,
                &[view.element_type.clone(), view.element_type.clone()],
                &Type::I32,
            )?)
        } else {
            None
        };
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: stable sort outside function")?;
        let i64_type = self.context.i64_type();
        let zero = i64_type.const_zero();
        let one = i64_type.const_int(1, false);
        let width_slot = self.entry_alloca(i64_type, "stable_sort_width").unwrap();
        let left_slot = self.entry_alloca(i64_type, "stable_sort_left").unwrap();
        let left_cursor = self.entry_alloca(i64_type, "stable_sort_i").unwrap();
        let right_cursor = self.entry_alloca(i64_type, "stable_sort_j").unwrap();
        let output_cursor = self.entry_alloca(i64_type, "stable_sort_k").unwrap();
        self.builder.build_store(width_slot, one).unwrap();

        let pass_condition = self
            .context
            .append_basic_block(function, "stable_sort.pass.condition");
        let pass_body = self
            .context
            .append_basic_block(function, "stable_sort.pass.body");
        let left_condition = self
            .context
            .append_basic_block(function, "stable_sort.left.condition");
        let merge_setup = self
            .context
            .append_basic_block(function, "stable_sort.merge.setup");
        let merge_condition = self
            .context
            .append_basic_block(function, "stable_sort.merge.condition");
        let compare = self
            .context
            .append_basic_block(function, "stable_sort.compare");
        let take_left = self
            .context
            .append_basic_block(function, "stable_sort.take.left");
        let take_right = self
            .context
            .append_basic_block(function, "stable_sort.take.right");
        let drain_left_condition = self
            .context
            .append_basic_block(function, "stable_sort.drain.left.condition");
        let drain_left = self
            .context
            .append_basic_block(function, "stable_sort.drain.left");
        let drain_right_condition = self
            .context
            .append_basic_block(function, "stable_sort.drain.right.condition");
        let drain_right = self
            .context
            .append_basic_block(function, "stable_sort.drain.right");
        let copy_condition = self
            .context
            .append_basic_block(function, "stable_sort.copy.condition");
        let copy_body = self
            .context
            .append_basic_block(function, "stable_sort.copy.body");
        let next_left = self
            .context
            .append_basic_block(function, "stable_sort.next.left");
        let next_width = self
            .context
            .append_basic_block(function, "stable_sort.next.width");
        let width_to_length = self
            .context
            .append_basic_block(function, "stable_sort.width.to.length");
        let width_double = self
            .context
            .append_basic_block(function, "stable_sort.width.double");
        let done = self
            .context
            .append_basic_block(function, "stable_sort.done");
        self.builder
            .build_unconditional_branch(pass_condition)
            .unwrap();

        let sort = MergeSort {
            view,
            data,
            scratch,
            element_type,
            i64_type,
            zero,
            one,
            width_slot,
            left_slot,
            left_cursor,
            right_cursor,
            output_cursor,
            function,
            blocks: MergeSortBlocks {
                pass_condition,
                pass_body,
                left_condition,
                merge_setup,
                merge_condition,
                compare,
                take_left,
                take_right,
                drain_left_condition,
                drain_left,
                drain_right_condition,
                drain_right,
                copy_condition,
                copy_body,
                next_left,
                next_width,
                width_to_length,
                width_double,
                done,
            },
        };
        let range = self.emit_merge_sort_passes(sort)?;
        self.emit_merge_sort_merge(sort, range, &callback, &keyed)?;
        self.emit_merge_sort_drain(sort, range)?;
        self.emit_merge_sort_copy_back(sort, range)?;
        self.builder.position_at_end(done);
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[scratch.into()], "stable_sort_free_scratch")
            .unwrap();
        if let Some((closure, _)) = callback {
            self.release_synchronous_closure(closure, false)?;
        }
        self.builder
            .build_load(view.llvm_array_type, target, "stable_sort_result")
            .map_err(|error| error.to_string())
    }

    pub(crate) fn emit_keyed_sort(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
        copy: bool,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let key = match args.first().map(tyrus_core::ast::Expr::kind) {
            Some(ExprKind::StrLit(key)) => key,
            _ => {
                return Err(
                    "error: keyed array sorting requires a string literal field name".to_string(),
                );
            }
        };
        let is_object = matches!(
            transparent_codegen_type(&view.element_type),
            Type::Object(_)
        ) || matches!(transparent_codegen_type(&view.element_type), Type::Named(name) if self.interfaces.contains_key(name));
        if !is_object {
            return Err(format!(
                "error: keyed array sorting requires object elements, found `{}`",
                view.element_type
            ));
        }
        let field_index = self
            .struct_field_index(&view.element_type, key)
            .ok_or_else(|| {
                format!(
                    "error: sort key `{key}` is not a field of `{}`",
                    view.element_type
                )
            })?;
        let field_type = self
            .struct_field_type(&view.element_type, key)
            .expect("field index and field type must resolve together");
        let descending = matches!(
            args.get(1).map(tyrus_core::ast::Expr::kind),
            Some(ExprKind::StrLit(value)) if value == "desc"
        );
        self.emit_sort(
            view,
            &[],
            copy,
            Some((field_index as usize, field_type, descending)),
        )
    }

    pub(super) fn emit_default_sort_comparison(
        &self,
        element_type: &Type,
        left: BasicValueEnum<'ctx>,
        right: BasicValueEnum<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let greater = match element_type {
            Type::I32 | Type::I64 | Type::I128 | Type::Isize | Type::Bool => self
                .builder
                .build_int_compare(
                    IntPredicate::SGT,
                    left.into_int_value(),
                    right.into_int_value(),
                    "sort_greater",
                )
                .unwrap(),
            Type::U32 | Type::U64 | Type::U128 | Type::Usize => self
                .builder
                .build_int_compare(
                    IntPredicate::UGT,
                    left.into_int_value(),
                    right.into_int_value(),
                    "sort_greater",
                )
                .unwrap(),
            Type::F32 | Type::F64 | Type::F128 => self
                .builder
                .build_float_compare(
                    inkwell::FloatPredicate::OGT,
                    left.into_float_value(),
                    right.into_float_value(),
                    "sort_greater",
                )
                .unwrap(),
            Type::Str => {
                return self.emit_string_compare(
                    left.into_struct_value(),
                    right.into_struct_value(),
                    "sort_compare",
                );
            }
            _ => {
                return Err(format!(
                    "error: default sort is not available for `{element_type}`; provide a comparator"
                ));
            }
        };
        Ok(self
            .builder
            .build_int_z_extend(greater, self.context.i32_type(), "sort_comparison")
            .unwrap())
    }

    pub(super) fn emit_flat(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let depth = match args.first() {
            None => 1,
            Some(Expr {
                kind: ExprKind::IntLit(depth),
                ..
            }) if *depth >= 0 => *depth as usize,
            Some(_) => {
                return Err(
                    "error: `flat` depth must be a non-negative integer literal in native tyrus"
                        .to_string(),
                );
            }
        };
        let output_element = flatten_array_element(&view.element_type, depth);
        let (output, output_type) = self.empty_array_slot(&output_element, "flat_output");
        self.emit_forward_loop(view.length, "array.flat", |this, index| {
            let value = this.load_array_element(view, index);
            this.emit_flattened_value(output, &output_element, &view.element_type, value, depth)
        })?;
        self.builder
            .build_load(output_type, output, "flat_result")
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_flat_map(
        &mut self,
        view: &ArrayView<'ctx>,
        args: &[Expr],
        expected: Option<&Type>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let inferred_output;
        let result_type = if let Some(expected) = expected {
            expected
        } else {
            let callback_return = self
                .infer_callback_return_type(
                    &args[0],
                    &[
                        view.element_type.clone(),
                        Type::Usize,
                        view.array_type.clone(),
                    ],
                )
                .ok_or("error: cannot infer `flatMap` callback return type")?;
            let Type::Array(element) = callback_return else {
                return Err("error: `flatMap` callback must return an array".to_string());
            };
            inferred_output = Type::Array(element);
            &inferred_output
        };
        let Type::Array(output_element) = result_type else {
            return Err("error: `flatMap` result must be an array".to_string());
        };
        let callback_return = Type::Array(output_element.clone());
        let available = [
            view.element_type.clone(),
            Type::Usize,
            view.array_type.clone(),
        ];
        let (callback, callback_params) =
            self.emit_callback(&args[0], &available, &callback_return)?;
        let (output, output_type) = self.empty_array_slot(output_element, "flat_map_output");
        self.emit_forward_loop(view.length, "array.flatMap", |this, index| {
            let element = this.load_array_element(view, index);
            let arguments =
                this.callback_arguments(callback_params.len(), element, index, view.value);
            let mapped = this
                .emit_closure_call(callback, &callback_params, &callback_return, &arguments)?
                .ok_or("internal: flatMap callback returned void")?;
            this.emit_flattened_value(output, output_element, &callback_return, mapped, 1)?;
            let data = this
                .builder
                .build_extract_value(mapped.into_struct_value(), 0, "flat_map.mapped.data")
                .unwrap();
            let free = this.declare_libc_fn(
                "free",
                &[this
                    .context
                    .ptr_type(inkwell::AddressSpace::default())
                    .into()],
                None,
            );
            this.builder.build_call(free, &[data.into()], "").unwrap();
            Ok(())
        })?;
        self.release_synchronous_closure(callback, false)?;
        self.builder
            .build_load(output_type, output, "flat_map_result")
            .map_err(|error| error.to_string())
    }

    pub(super) fn emit_flattened_value(
        &mut self,
        output: PointerValue<'ctx>,
        output_element: &Type,
        value_type: &Type,
        value: BasicValueEnum<'ctx>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 0
            && let Type::Array(inner) = value_type
        {
            let array = value.into_struct_value();
            let data = self
                .builder
                .build_extract_value(array, 0, "flat_nested_data")
                .unwrap()
                .into_pointer_value();
            let length = self
                .builder
                .build_extract_value(array, 1, "flat_nested_length")
                .unwrap()
                .into_int_value();
            return self.emit_forward_loop(length, "array.flat.nested", |this, index| {
                let slot = unsafe {
                    this.builder
                        .build_gep(this.llvm_type(inner), data, &[index], "flat_nested_index")
                        .unwrap()
                };
                let nested = this
                    .builder
                    .build_load(this.llvm_type(inner), slot, "flat_nested_value")
                    .unwrap();
                this.emit_flattened_value(output, output_element, inner, nested, depth - 1)
            });
        }
        if value_type != output_element {
            return Err(format!(
                "error: cannot flatten `{value_type}` into `{output_element}[]`"
            ));
        }
        self.push_array_value(output, output_element, value)
    }
}
