use super::*;
use tyrus_core::ast::{Expr, ExprKind, OnFail, OnFailHandler, Stmt, StmtKind};
use tyrus_core::type_checker::TupleSequence;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ThrownValue {
    Str,
    Validation,
    Coded(&'static str),
    ErrorMessage,
    Other,
}

pub(super) fn native_coded_error(name: &str) -> Option<&'static str> {
    Some(match name {
        "AlgorithmError" => "AlgorithmError",
        "EncodingError" => "EncodingError",
        "FsError" => "FsError",
        "MemoryError" => "MemoryError",
        "OsError" => "OsError",
        "TemporalError" => "TemporalError",
        "ZipError" => "ZipError",
        _ => return None,
    })
}

#[derive(Debug, Clone)]
pub(super) struct CatchBindingRelease<'ctx> {
    function: FunctionValue<'ctx>,
    name: String,
    owned_slot: PointerValue<'ctx>,
    error_slot: PointerValue<'ctx>,
    catch_type: Type,
}

#[derive(Clone)]
pub(crate) struct ErrorSubtypeStorage<'ctx> {
    pub(crate) binding: String,
    pub(crate) tag_slot: PointerValue<'ctx>,
    pub(crate) slots: Vec<(String, PointerValue<'ctx>)>,
    pub(crate) scope_depth: usize,
    pub(crate) function: FunctionValue<'ctx>,
}

pub(crate) struct ThrowFrame {
    pub(crate) throws: Vec<Type>,
    pub(crate) catches: Vec<Type>,
}

pub(crate) fn thrown_type_tag_of(spelling: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in spelling.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub(crate) fn first_thrown_tag_collision<'a>(
    spellings: impl IntoIterator<Item = &'a str>,
    tag: impl Fn(&str) -> u64,
) -> Option<(String, String)> {
    let mut seen = std::collections::HashMap::new();
    for spelling in spellings {
        if let Some(previous) = seen.insert(tag(spelling), spelling)
            && previous != spelling
        {
            return Some((previous.to_string(), spelling.to_string()));
        }
    }
    None
}

#[derive(Debug, Clone, Copy)]
pub(super) enum AsyncRejection<'ctx> {
    Message(BasicValueEnum<'ctx>),
    Structured { code_index: u32, message_index: u32 },
}

#[derive(Debug, Clone)]
struct DiscriminantNarrowing {
    name: String,
    field: String,
    literal: Expr,
    matches_in_then: bool,
}

#[path = "assignments/assignments.rs"]
mod assignments;
#[path = "control_flow/control_flow.rs"]
mod control_flow;
#[path = "declarations/declarations.rs"]
mod declarations;
#[path = "destructuring/destructuring.rs"]
mod destructuring;
#[path = "errors/errors.rs"]
mod errors;
#[path = "flow_exits/flow_exits.rs"]
mod flow_exits;
#[path = "match_stmt/match_stmt.rs"]
mod match_stmt;
#[path = "switch_stmt/switch_stmt.rs"]
mod switch_stmt;

impl<'ctx> Codegen<'ctx> {
    pub(super) fn emit_stmt(
        &mut self,
        stmt: &Stmt,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let tracked = !self.async_results.is_empty()
            && crate::core::first_await_in_block(std::slice::from_ref(stmt)).is_none();
        if tracked {
            self.async_temporaries_tracked += 1;
        }
        if matches!(stmt.kind(), StmtKind::Throw(_)) {
            self.emit_async_throw_region_marker(stmt.span, fn_val);
        }
        let result = self.emit_stmt_kind(stmt, fn_val);
        if tracked {
            self.async_temporaries_tracked -= 1;
        }
        result
    }

    fn emit_stmt_kind(&mut self, stmt: &Stmt, fn_val: FunctionValue<'ctx>) -> Result<(), String> {
        match stmt.kind() {
            StmtKind::Defer { kind, body, .. } => {
                if !self.async_results.is_empty() {
                    return Err("error: `defer` and `errdefer` inside async functions are not supported yet".to_string());
                }
                self.deferred_scopes
                    .last_mut()
                    .expect("internal: deferred finalizer registered without a scope")
                    .push(DeferredFinalizer {
                        kind: *kind,
                        body: body.clone(),
                    });
                Ok(())
            }
            StmtKind::VarDecl { .. } => self.emit_var_decl_stmt(stmt, fn_val),
            StmtKind::Assign { .. }
            | StmtKind::IndexAssign { .. }
            | StmtKind::FieldAssign { .. } => self.emit_assignment_stmt(stmt, fn_val),
            StmtKind::Return { .. } | StmtKind::Break { .. } | StmtKind::Continue => {
                self.emit_flow_exit_stmt(stmt, fn_val)
            }

            StmtKind::While { cond, body } => self.emit_while(cond, body, fn_val),
            StmtKind::DoWhile { body, cond } => self.emit_do_while(cond, body, fn_val),

            StmtKind::For {
                init,
                cond,
                update,
                body,
            } => self.emit_for(init, cond, update, body, fn_val),

            StmtKind::ForOf {
                name,
                iterable,
                body,
                ..
            } => self.emit_for_of(name, iterable, body, fn_val),

            StmtKind::ForIn { name, object, body } => self.emit_for_in(name, object, body, fn_val),
            StmtKind::ArrayDestructure { .. } | StmtKind::ObjectDestructure { .. } => {
                self.emit_destructure_stmt(stmt, fn_val)
            }
            StmtKind::Expr(expr) => {
                if let ExprKind::FnCall {
                    callee,
                    type_args,
                    args,
                } = expr.kind()
                {
                    let is_user_defined = self.fn_signatures.contains_key(callee)
                        || self.generic_fns.contains_key(callee)
                        || self.load_var(callee).is_some();
                    if callee == "panic" && type_args.is_empty() && !is_user_defined {
                        return self.emit_panic_builtin(args);
                    }
                }
                self.emit_expr_for_side_effects(expr, fn_val)?;
                self.discard_owned_native_result();
                Ok(())
            }
            StmtKind::Switch { .. } | StmtKind::Throw { .. } | StmtKind::TryCatch { .. } => {
                self.emit_switch_stmt(stmt, fn_val)
            }
            StmtKind::Match { .. } => self.emit_match_stmt(stmt, fn_val),
        }
    }

    pub(super) fn replace_async_string_local(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        slot: PointerValue<'ctx>,
        ownership: PointerValue<'ctx>,
        value: StructValue<'ctx>,
        new_owned: bool,
    ) {
        let old_owned = self
            .builder
            .build_load(
                self.context.bool_type(),
                ownership,
                &format!("async.local.{name}.old.owned"),
            )
            .unwrap()
            .into_int_value();
        let release = self
            .context
            .append_basic_block(function, "release.reassigned.async.string");
        let store = self
            .context
            .append_basic_block(function, "store.reassigned.async.string");
        self.builder
            .build_conditional_branch(old_owned, release, store)
            .unwrap();
        self.builder.position_at_end(release);
        let old_value = self
            .builder
            .build_load(
                self.string_abi_type(),
                slot,
                &format!("async.local.{name}.old"),
            )
            .unwrap()
            .into_struct_value();
        self.emit_string_release(old_value)
            .expect("releasing a superseded string is always emittable");
        self.builder.build_unconditional_branch(store).unwrap();
        self.builder.position_at_end(store);
        self.builder.build_store(slot, value).unwrap();
        self.builder
            .build_store(
                ownership,
                self.context.bool_type().const_int(new_owned as u64, false),
            )
            .unwrap();
    }

    pub(super) fn replace_async_copy_array_local(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        slot: PointerValue<'ctx>,
        value: StructValue<'ctx>,
    ) {
        let array_type = value.get_type();
        let old_value = self
            .builder
            .build_load(array_type, slot, &format!("async.local.{name}.old"))
            .unwrap()
            .into_struct_value();
        let old_data = self
            .builder
            .build_extract_value(old_value, 0, &format!("async.local.{name}.old.data"))
            .unwrap()
            .into_pointer_value();
        let release = self
            .context
            .append_basic_block(function, "release.reassigned.async.array");
        let store = self
            .context
            .append_basic_block(function, "store.reassigned.async.array");
        let present = self
            .builder
            .build_is_not_null(old_data, &format!("async.local.{name}.old.present"))
            .unwrap();
        self.builder
            .build_conditional_branch(present, release, store)
            .unwrap();
        self.builder.position_at_end(release);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[old_data.into()], "")
            .unwrap();
        self.builder.build_unconditional_branch(store).unwrap();
        self.builder.position_at_end(store);
        self.builder.build_store(slot, value).unwrap();
    }

    pub(super) fn normalize_async_string_array_for_mutation(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        slot: PointerValue<'ctx>,
        ownership: PointerValue<'ctx>,
        ty: &Type,
    ) -> Result<(), String> {
        let mode = self
            .builder
            .build_load(
                self.context.i8_type(),
                ownership,
                &format!("async.local.{name}.array.mutation.ownership"),
            )
            .unwrap()
            .into_int_value();
        let shallow = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                mode,
                self.context.i8_type().const_int(1, false),
                &format!("async.local.{name}.array.mutation.shallow"),
            )
            .unwrap();
        let normalize = self
            .context
            .append_basic_block(function, "normalize.async.string.array.for.mutation");
        let ready = self
            .context
            .append_basic_block(function, "mutate.async.string.array");
        self.builder
            .build_conditional_branch(shallow, normalize, ready)
            .unwrap();

        self.builder.position_at_end(normalize);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let array_type = self.llvm_type(ty);
        let cloned = self
            .builder
            .build_alloca(array_type, "async.string.array.mutation.clone")
            .unwrap();
        let clone = self.declare_libc_fn(
            "tyrus_array_clone_strings",
            &[pointer.into(), pointer.into()],
            Some(self.context.i32_type().into()),
        );
        let status = self
            .builder
            .build_call(
                clone,
                &[slot.into(), cloned.into()],
                "async.string.array.mutation.clone.status",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        self.emit_abort_if_negative(status, "async string-array mutation", "async.string.array")?;
        let previous = self
            .builder
            .build_load(array_type, slot, "async.string.array.mutation.previous")
            .unwrap()
            .into_struct_value();
        let previous_data = self
            .builder
            .build_extract_value(previous, 0, "async.string.array.mutation.previous.data")
            .unwrap()
            .into_pointer_value();
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[previous_data.into()], "")
            .unwrap();
        let cloned_value = self
            .builder
            .build_load(array_type, cloned, "async.string.array.mutation.cloned")
            .unwrap();
        self.builder.build_store(slot, cloned_value).unwrap();
        self.builder
            .build_store(ownership, self.context.i8_type().const_int(2, false))
            .unwrap();
        self.builder.build_unconditional_branch(ready).unwrap();
        self.builder.position_at_end(ready);
        Ok(())
    }

    pub(super) fn replace_async_string_array_local(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        slot: PointerValue<'ctx>,
        ownership: PointerValue<'ctx>,
        value: StructValue<'ctx>,
        new_mode: u64,
    ) {
        let old_mode = self
            .builder
            .build_load(
                self.context.i8_type(),
                ownership,
                &format!("async.local.{name}.array.old.ownership"),
            )
            .unwrap()
            .into_int_value();
        let release_deep = self
            .context
            .append_basic_block(function, "release.reassigned.async.string.array.deep");
        let check_shallow = self
            .context
            .append_basic_block(function, "check.reassigned.async.string.array.shallow");
        let release_shallow = self
            .context
            .append_basic_block(function, "release.reassigned.async.string.array.shallow");
        let store = self
            .context
            .append_basic_block(function, "store.reassigned.async.string.array");
        let deep = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                old_mode,
                self.context.i8_type().const_int(2, false),
                &format!("async.local.{name}.array.old.deep"),
            )
            .unwrap();
        self.builder
            .build_conditional_branch(deep, release_deep, check_shallow)
            .unwrap();
        self.builder.position_at_end(release_deep);
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let drop_owned =
            self.declare_libc_fn("tyrus_array_drop_string_contents", &[pointer.into()], None);
        self.builder
            .build_call(drop_owned, &[slot.into()], "")
            .unwrap();
        self.builder.build_unconditional_branch(store).unwrap();
        self.builder.position_at_end(check_shallow);
        let shallow = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                old_mode,
                self.context.i8_type().const_int(1, false),
                &format!("async.local.{name}.array.old.shallow"),
            )
            .unwrap();
        self.builder
            .build_conditional_branch(shallow, release_shallow, store)
            .unwrap();
        self.builder.position_at_end(release_shallow);
        let old_value = self
            .builder
            .build_load(
                value.get_type(),
                slot,
                &format!("async.local.{name}.array.old"),
            )
            .unwrap()
            .into_struct_value();
        let old_data = self
            .builder
            .build_extract_value(old_value, 0, &format!("async.local.{name}.array.old.data"))
            .unwrap()
            .into_pointer_value();
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[old_data.into()], "")
            .unwrap();
        self.builder.build_unconditional_branch(store).unwrap();
        self.builder.position_at_end(store);
        self.builder.build_store(slot, value).unwrap();
        self.builder
            .build_store(ownership, self.context.i8_type().const_int(new_mode, false))
            .unwrap();
    }

    pub(super) fn emit_logical_assignment(
        &mut self,
        name: &str,
        op: &AssignOp,
        value: &Expr,
        function: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let (pointer, target_type) = self
            .load_var(name)
            .ok_or_else(|| format!("internal: variable `{name}` not found in codegen"))?;
        let current = self
            .builder
            .build_load(self.llvm_type(&target_type), pointer, name)
            .unwrap();
        let assign_block = self.context.append_basic_block(function, "logical.assign");
        let continue_block = self
            .context
            .append_basic_block(function, "logical.continue");

        let assign_when_true = match op {
            AssignOp::AndAssign => current.into_int_value(),
            AssignOp::OrAssign => self
                .builder
                .build_not(current.into_int_value(), "logical.not")
                .unwrap(),
            AssignOp::NullCoalesceAssign => self
                .builder
                .build_is_null(current.into_pointer_value(), "is_null")
                .unwrap(),
            _ => unreachable!(),
        };
        self.builder
            .build_conditional_branch(assign_when_true, assign_block, continue_block)
            .unwrap();

        self.builder.position_at_end(assign_block);
        let assigned = self.emit_expr_as(value, &target_type)?;
        self.builder.build_store(pointer, assigned).unwrap();
        self.builder
            .build_unconditional_branch(continue_block)
            .unwrap();

        self.builder.position_at_end(continue_block);
        Ok(())
    }

    pub(super) fn current_block_terminated(&self) -> bool {
        self.builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_terminator)
            .is_some()
    }

    pub(super) fn emit_do_while(
        &mut self,
        cond: &Expr,
        body: &[Stmt],
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let body_bb = self.context.append_basic_block(fn_val, "dowhile.body");
        let cond_bb = self.context.append_basic_block(fn_val, "dowhile.cond");
        let end_bb = self.context.append_basic_block(fn_val, "dowhile.end");

        self.builder.build_unconditional_branch(body_bb).unwrap();

        self.builder.position_at_end(body_bb);
        self.push_scope();
        self.push_loop_control(cond_bb, end_bb);
        for stmt in body {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(stmt, fn_val)?;
        }
        self.pop_loop_control();
        self.pop_scope();
        if !self.current_block_terminated() {
            self.builder.build_unconditional_branch(cond_bb).unwrap();
        }

        self.builder.position_at_end(cond_bb);
        let cond_val = self.emit_condition_value(cond)?;
        self.discard_owned_native_result();
        self.builder
            .build_conditional_branch(cond_val, body_bb, end_bb)
            .unwrap();

        self.builder.position_at_end(end_bb);
        Ok(())
    }

    fn for_index_hint(&self, init: &Stmt, cond: &Expr) -> Option<Type> {
        let StmtKind::VarDecl {
            name,
            ty: Type::Infer,
            value: Some(value),
            ..
        } = init.kind()
        else {
            return None;
        };
        if !matches!(value.kind(), ExprKind::IntLit(start) if *start >= 0) {
            return None;
        }
        let ExprKind::BinOp { op, left, right } = cond.kind() else {
            return None;
        };
        if !matches!(
            op,
            BinOp::Lt | BinOp::Lte | BinOp::Gt | BinOp::Gte | BinOp::Eq | BinOp::Neq
        ) {
            return None;
        }
        let bound = match (left.kind(), right.kind()) {
            (ExprKind::Ident(counter), _) if counter == name => right.as_ref(),
            (_, ExprKind::Ident(counter)) if counter == name => left.as_ref(),
            _ => return None,
        };
        let bound_ty = self.for_bound_type(bound)?;
        is_integer_type(&bound_ty).then_some(bound_ty)
    }

    fn for_bound_type(&self, expr: &Expr) -> Option<Type> {
        match expr.kind() {
            ExprKind::Ident(name) => {
                let (_, ty) = self.load_var(name)?;
                Some(transparent_codegen_type(&ty).clone())
            }
            ExprKind::FieldAccess { object, field }
                if matches!(field.as_str(), "length" | "capacity") =>
            {
                let object_ty = self.for_bound_type(object)?;
                matches!(object_ty, Type::Array(_) | Type::Tuple(_) | Type::Str)
                    .then_some(Type::Usize)
            }
            ExprKind::BinOp {
                op: BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod,
                left,
                right,
            } => self
                .for_bound_type(left)
                .or_else(|| self.for_bound_type(right)),
            _ => None,
        }
    }

    pub(super) fn tuple_iteration_element_type(&self, elements: &[Type]) -> Result<Type, String> {
        let resolved = elements
            .iter()
            .map(|element| self.resolve_type(element))
            .collect::<Vec<_>>();
        match tyrus_core::type_checker::tuple_sequence(&resolved) {
            TupleSequence::Uniform { element } => Ok(element),
            TupleSequence::Mixed { .. } => Err(
                "error: iterating a tuple whose positions have different types is not supported yet"
                    .to_string(),
            ),
            TupleSequence::Empty => Err("error: cannot iterate an empty tuple".to_string()),
        }
    }

    pub(super) fn emit_if_stmt(
        &mut self,
        cond: &Expr,
        then_block: &[Stmt],
        else_block: Option<&[Stmt]>,
        fn_val: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let typeof_narrowing = Self::detect_typeof_narrowing(cond);
        let is_narrowing = Self::detect_is_narrowing(cond);
        let optional_narrowing = Self::detect_optional_presence_narrowing(cond);
        let optional_path_narrowing =
            codegen_expression_path(cond).filter(|path| path.contains('.'));
        let result_narrowing = Self::detect_result_narrowing(cond);
        let discriminant_narrowing = self.detect_discriminant_narrowing(cond);
        let cond_val = if let Some(narrowing) = &discriminant_narrowing {
            self.emit_discriminant_condition(narrowing)?
        } else {
            self.emit_condition_value(cond)?
        };
        // `pending_owned_native_results` is a single flat, compile-time list:
        // whichever statement's own end-of-statement discard happens to run
        // *next* is the one that emits the `free` for anything still on it,
        // wherever that statement's own codegen lands. Left un-flushed here,
        // a temporary the condition alone produced (`(x ?? "z").length > 99`
        // clones the fallback/present branch into a fresh owned string) is
        // "freed" by the first statement inside `then`/`else` that happens to
        // discard, landing the `free` inside a block only one runtime branch
        // ever reaches, every other branch leaks it every time it is taken,
        // confirmed by dumping IR: the `free` existed only in the `then`
        // block, and `else` (this project's `??`-in-`if` leak corpus entry,
        // was `2` B/iter, always took that path in the reproduction) branched
        // straight to `merge` with nothing releasing the phi result at all.
        // Flushing right here, before `then_bb`/`else_bb` exist, puts the
        // `free` (if any) in the block both branches originate from, it runs
        // exactly once, regardless of which one is taken.
        self.discard_owned_native_result();

        let then_bb = self.context.append_basic_block(fn_val, "then");
        let else_bb = self.context.append_basic_block(fn_val, "else");
        let merge_bb = self.context.append_basic_block(fn_val, "merge");

        self.builder
            .build_conditional_branch(cond_val, then_bb, else_bb)
            .unwrap();

        self.builder.position_at_end(then_bb);
        self.push_scope();
        if let Some((name, type_name, type_in_then)) = &typeof_narrowing
            && let Some(concrete) = self.typeof_branch_type(name, type_name, *type_in_then)
        {
            self.store_concrete_type(name, concrete);
        }
        if let Some((name, target, target_in_then)) = &is_narrowing
            && !self.apply_error_subtype_narrowing(name, target, *target_in_then)
            && let Some(concrete) = self.is_branch_type(name, target, *target_in_then)
        {
            self.store_concrete_type(name, concrete);
        }
        if let Some(name) = &optional_narrowing
            && let Some(concrete) = self.optional_present_type(name)
        {
            self.store_concrete_type(name, concrete);
        }
        if let Some(path) = &optional_path_narrowing
            && let Some(scope) = self.narrowed_paths.last_mut()
        {
            scope.insert(path.clone());
        }
        if let Some((name, success_in_then)) = &result_narrowing {
            if *success_in_then {
                if let Some(concrete) = self.result_success_type(name) {
                    self.store_concrete_type(name, concrete);
                }
            } else if let Some(concrete) = self.result_error_type(name) {
                self.store_concrete_type(name, concrete);
            }
        }
        if let Some(narrowing) = &discriminant_narrowing
            && let Some(concrete) = self.discriminant_branch_type(
                &narrowing.name,
                &narrowing.field,
                &narrowing.literal,
                narrowing.matches_in_then,
            )
        {
            self.store_concrete_type(&narrowing.name, concrete);
        }
        for stmt in then_block {
            self.emit_stmt(stmt, fn_val)?;
        }
        self.pop_scope();
        if self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_terminator)
            .is_none()
        {
            self.builder.build_unconditional_branch(merge_bb).unwrap();
        }

        self.builder.position_at_end(else_bb);
        if let Some(else_stmts) = else_block {
            self.push_scope();
            if let Some((name, success_in_then)) = &result_narrowing {
                if !*success_in_then {
                    if let Some(concrete) = self.result_success_type(name) {
                        self.store_concrete_type(name, concrete);
                    }
                } else if let Some(concrete) = self.result_error_type(name) {
                    self.store_concrete_type(name, concrete);
                }
            }
            if let Some((name, type_name, type_in_then)) = &typeof_narrowing
                && let Some(concrete) = self.typeof_branch_type(name, type_name, !*type_in_then)
            {
                self.store_concrete_type(name, concrete);
            }
            if let Some((name, target, target_in_then)) = &is_narrowing
                && !self.apply_error_subtype_narrowing(name, target, !*target_in_then)
                && let Some(concrete) = self.is_branch_type(name, target, !*target_in_then)
            {
                self.store_concrete_type(name, concrete);
            }
            if let Some(narrowing) = &discriminant_narrowing
                && let Some(concrete) = self.discriminant_branch_type(
                    &narrowing.name,
                    &narrowing.field,
                    &narrowing.literal,
                    !narrowing.matches_in_then,
                )
            {
                self.store_concrete_type(&narrowing.name, concrete);
            }
            for stmt in else_stmts {
                self.emit_stmt(stmt, fn_val)?;
            }
            self.pop_scope();
        }
        if self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_terminator)
            .is_none()
        {
            self.builder.build_unconditional_branch(merge_bb).unwrap();
        }

        self.builder.position_at_end(merge_bb);
        Ok(())
    }

    pub(super) fn emit_condition_value(&mut self, cond: &Expr) -> Result<IntValue<'ctx>, String> {
        let value = self.emit_expr(cond)?;
        self.discard_owned_native_result();
        if matches!(self.expr_type_hint(cond), Some(Type::Optional(_))) {
            return Ok(self
                .builder
                .build_is_not_null(value.into_pointer_value(), "optional.is_present")
                .unwrap());
        }
        if matches!(self.expr_type_hint(cond), Some(Type::Result { .. })) {
            let tag = self
                .builder
                .build_extract_value(value.into_struct_value(), 0, "result.condition.tag")
                .unwrap()
                .into_int_value();
            return Ok(self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    self.context.i8_type().const_zero(),
                    "result.is_success",
                )
                .unwrap());
        }
        Ok(value.into_int_value())
    }

    pub(super) fn apply_condition_narrowing(&mut self, cond: &Expr, in_then: bool) {
        if in_then
            && let Some(name) = Self::detect_optional_presence_narrowing(cond)
            && let Some(concrete) = self.optional_present_type(&name)
        {
            self.store_concrete_type(&name, concrete);
        }
        if let Some((name, success_in_then)) = Self::detect_result_narrowing(cond) {
            if success_in_then == in_then {
                if let Some(concrete) = self.result_success_type(&name) {
                    self.store_concrete_type(&name, concrete);
                }
            } else if let Some(concrete) = self.result_error_type(&name) {
                self.store_concrete_type(&name, concrete);
            }
        }
        if let Some((name, type_name, type_in_then)) = Self::detect_typeof_narrowing(cond)
            && let Some(concrete) =
                self.typeof_branch_type(&name, &type_name, type_in_then == in_then)
        {
            self.store_concrete_type(&name, concrete);
        }
        if let Some((name, target, target_in_then)) = Self::detect_is_narrowing(cond)
            && !self.apply_error_subtype_narrowing(&name, &target, target_in_then == in_then)
            && let Some(concrete) = self.is_branch_type(&name, &target, target_in_then == in_then)
        {
            self.store_concrete_type(&name, concrete);
        }
        if let Some(narrowing) = self.detect_discriminant_narrowing(cond)
            && let Some(concrete) = self.discriminant_branch_type(
                &narrowing.name,
                &narrowing.field,
                &narrowing.literal,
                narrowing.matches_in_then == in_then,
            )
        {
            self.store_concrete_type(&narrowing.name, concrete);
        }
    }

    pub(super) fn detect_optional_presence_narrowing(cond: &Expr) -> Option<String> {
        match cond.kind() {
            ExprKind::Ident(name) => Some(name.clone()),
            _ => None,
        }
    }

    pub(super) fn detect_result_narrowing(cond: &Expr) -> Option<(String, bool)> {
        match cond.kind() {
            ExprKind::Ident(name) => Some((name.clone(), true)),
            ExprKind::Not(inner) => match inner.kind() {
                ExprKind::Ident(name) => Some((name.clone(), false)),
                _ => None,
            },
            _ => None,
        }
    }

    pub(super) fn path_is_narrowed(&self, path: &str) -> bool {
        self.narrowed_paths.iter().any(|scope| scope.contains(path))
    }

    pub(super) fn optional_present_type(&self, name: &str) -> Option<Type> {
        let (_, ty) = self.load_var(name)?;
        match ty {
            Type::Optional(inner) => Some(*inner),
            _ => None,
        }
    }

    pub(super) fn result_success_type(&self, name: &str) -> Option<Type> {
        let (_, ty) = self.load_var(name)?;
        match self.resolve_type(&ty) {
            Type::Result { success, .. } => Some(*success),
            _ => None,
        }
    }

    fn detect_typeof_narrowing(cond: &Expr) -> Option<(String, String, bool)> {
        let Expr {
            kind: ExprKind::BinOp {
                op, left, right, ..
            },
            ..
        } = cond
        else {
            return None;
        };
        let (typeof_expr, type_name) = match (left.as_ref(), right.as_ref()) {
            (
                Expr {
                    kind: ExprKind::Typeof(inner),
                    ..
                },
                Expr {
                    kind: ExprKind::StrLit(name),
                    ..
                },
            ) => (inner, name.clone()),
            (
                Expr {
                    kind: ExprKind::StrLit(name),
                    ..
                },
                Expr {
                    kind: ExprKind::Typeof(inner),
                    ..
                },
            ) => (inner, name.clone()),
            _ => return None,
        };
        let var_name = match typeof_expr.as_ref().kind() {
            ExprKind::Ident(name) => name.clone(),
            _ => return None,
        };
        match op {
            BinOp::Eq => Some((var_name, type_name, true)),
            BinOp::Neq => Some((var_name, type_name, false)),
            _ => None,
        }
    }

    fn detect_discriminant_narrowing(&self, cond: &Expr) -> Option<DiscriminantNarrowing> {
        let Expr {
            kind: ExprKind::BinOp {
                op, left, right, ..
            },
            ..
        } = cond
        else {
            return None;
        };
        let (object, field, literal) = match (left.as_ref(), right.as_ref()) {
            (
                Expr {
                    kind: ExprKind::FieldAccess { object, field },
                    ..
                },
                literal,
            ) if Self::is_narrowing_literal(literal) => (object, field, literal),
            (
                literal,
                Expr {
                    kind: ExprKind::FieldAccess { object, field },
                    ..
                },
            ) if Self::is_narrowing_literal(literal) => (object, field, literal),
            _ => return None,
        };
        let name = match object.as_ref().kind() {
            ExprKind::Ident(name) => name.clone(),
            _ => return None,
        };
        let is_union = self
            .load_var(&name)
            .is_some_and(|(_, ty)| matches!(transparent_codegen_type(&ty), Type::Union(_)));
        if !is_union {
            return None;
        }
        match op {
            BinOp::Eq => Some(DiscriminantNarrowing {
                name,
                field: field.clone(),
                literal: literal.clone(),
                matches_in_then: true,
            }),
            BinOp::Neq => Some(DiscriminantNarrowing {
                name,
                field: field.clone(),
                literal: literal.clone(),
                matches_in_then: false,
            }),
            _ => None,
        }
    }

    fn is_narrowing_literal(expr: &Expr) -> bool {
        matches!(
            expr,
            Expr {
                kind: ExprKind::StrLit(_),
                ..
            } | Expr {
                kind: ExprKind::IntLit(_),
                ..
            } | Expr {
                kind: ExprKind::UIntLit(_),
                ..
            } | Expr {
                kind: ExprKind::FloatLit(_),
                ..
            } | Expr {
                kind: ExprKind::BoolLit(_),
                ..
            } | Expr {
                kind: ExprKind::Null,
                ..
            }
        )
    }

    fn emit_discriminant_condition(
        &self,
        narrowing: &DiscriminantNarrowing,
    ) -> Result<IntValue<'ctx>, String> {
        let (ptr, declared_ty) = self
            .load_var(&narrowing.name)
            .ok_or_else(|| format!("internal: `{}` not in codegen scope", narrowing.name))?;
        let Type::Union(declared_variants) = transparent_codegen_type(&declared_ty) else {
            return Err(format!(
                "error: discriminant narrowing requires a union value, found `{declared_ty}`"
            ));
        };
        let current_ty = self
            .current_union_flow_type(&narrowing.name)
            .unwrap_or_else(|| declared_ty.clone());
        let current_variants = match transparent_codegen_type(&current_ty) {
            Type::Union(variants) => variants.clone(),
            Type::Never => Vec::new(),
            ty => vec![ty.clone()],
        };
        let matching_tags = declared_variants
            .iter()
            .enumerate()
            .filter(|(_, variant)| {
                current_variants.iter().any(|current| {
                    transparent_codegen_type(current) == transparent_codegen_type(variant)
                })
            })
            .filter(|(_, variant)| {
                Self::variant_discriminant_matches(variant, &narrowing.field, &narrowing.literal)
                    == narrowing.matches_in_then
            })
            .map(|(index, _)| index as u64)
            .collect::<Vec<_>>();

        let bool_ty = self.context.bool_type();
        if matching_tags.is_empty() {
            return Ok(bool_ty.const_zero());
        }

        let union_ty = self.union_llvm_type(declared_variants);
        let tag_ptr = self
            .builder
            .build_struct_gep(union_ty, ptr, 0, "union.tag")
            .map_err(|error| format!("union tag gep: {error}"))?;
        let tag = self
            .builder
            .build_load(self.context.i8_type(), tag_ptr, "union.tag.load")
            .map_err(|error| format!("union tag load: {error}"))?
            .into_int_value();

        let mut comparisons = matching_tags.into_iter().map(|tag_index| {
            self.builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    self.context.i8_type().const_int(tag_index, false),
                    "union.tag.matches",
                )
                .map_err(|error| format!("union tag compare: {error}"))
        });
        let mut condition = comparisons
            .next()
            .transpose()?
            .unwrap_or_else(|| bool_ty.const_zero());
        for comparison in comparisons {
            condition = self
                .builder
                .build_or(condition, comparison?, "union.tag.any")
                .map_err(|error| format!("union tag or: {error}"))?;
        }
        Ok(condition)
    }

    fn detect_is_narrowing(cond: &Expr) -> Option<(String, Type, bool)> {
        let (inner, holds_in_then) = match cond.kind() {
            ExprKind::Not(inner) => (inner.as_ref(), false),
            _ => (cond, true),
        };
        let ExprKind::Is { expr, ty } = inner.kind() else {
            return None;
        };
        let ExprKind::Ident(name) = expr.kind() else {
            return None;
        };
        Some((name.clone(), ty.clone(), holds_in_then))
    }

    fn is_branch_type(&self, name: &str, target: &Type, holds_in_branch: bool) -> Option<Type> {
        let current_ty = self.current_union_flow_type(name)?;
        match transparent_codegen_type(&current_ty) {
            Type::Union(variants) => {
                let narrowed = variants
                    .iter()
                    .filter(|variant| self.types_assignable(variant, target) == holds_in_branch)
                    .cloned()
                    .collect::<Vec<_>>();
                Self::type_from_narrowed_variants(narrowed)
            }
            _ => None,
        }
    }

    fn typeof_branch_type(
        &self,
        name: &str,
        type_name: &str,
        type_in_branch: bool,
    ) -> Option<Type> {
        let current_ty = self.current_union_flow_type(name)?;
        match transparent_codegen_type(&current_ty) {
            Type::Union(variants) => {
                let narrowed = variants
                    .iter()
                    .filter(|variant| {
                        (Self::type_string_for_variant(variant) == type_name) == type_in_branch
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                Self::type_from_narrowed_variants(narrowed).filter(|ty| !matches!(ty, Type::Never))
            }
            ty if type_in_branch && Self::type_string_for_variant(ty) == type_name => {
                Some(ty.clone())
            }
            _ => None,
        }
    }

    fn discriminant_branch_type(
        &self,
        name: &str,
        field: &str,
        literal: &Expr,
        matches_branch: bool,
    ) -> Option<Type> {
        let current_ty = self.current_union_flow_type(name)?;
        let Type::Union(variants) = transparent_codegen_type(&current_ty) else {
            return None;
        };
        let narrowed = variants
            .iter()
            .filter(|variant| {
                Self::variant_discriminant_matches(variant, field, literal) == matches_branch
            })
            .cloned()
            .collect::<Vec<_>>();
        Self::type_from_narrowed_variants(narrowed)
    }

    fn current_union_flow_type(&self, name: &str) -> Option<Type> {
        self.load_concrete_type(name)
            .or_else(|| self.load_var(name).map(|(_, ty)| ty))
    }

    fn variant_discriminant_matches(variant: &Type, field: &str, literal: &Expr) -> bool {
        let Type::Object(fields) = transparent_codegen_type(variant) else {
            return false;
        };
        let Some(field_ty) = fields
            .iter()
            .find(|candidate| candidate.name == field)
            .map(field_storage_type)
        else {
            return false;
        };
        matches!(transparent_codegen_type(&field_ty), Type::Lit(expected) if Self::literal_expr_matches(expected, literal))
    }

    fn type_from_narrowed_variants(variants: Vec<Type>) -> Option<Type> {
        match variants.len() {
            0 => Some(Type::Never),
            1 => variants.into_iter().next(),
            _ => Some(Type::Union(variants)),
        }
    }

    fn literal_expr_matches(expected: &Expr, actual: &Expr) -> bool {
        match (expected, actual) {
            (
                Expr {
                    kind: ExprKind::StrLit(expected),
                    ..
                },
                Expr {
                    kind: ExprKind::StrLit(actual),
                    ..
                },
            ) => expected == actual,
            (
                Expr {
                    kind: ExprKind::IntLit(expected),
                    ..
                },
                Expr {
                    kind: ExprKind::IntLit(actual),
                    ..
                },
            ) => expected == actual,
            (
                Expr {
                    kind: ExprKind::UIntLit(expected),
                    ..
                },
                Expr {
                    kind: ExprKind::UIntLit(actual),
                    ..
                },
            ) => expected == actual,
            (
                Expr {
                    kind: ExprKind::FloatLit(expected),
                    ..
                },
                Expr {
                    kind: ExprKind::FloatLit(actual),
                    ..
                },
            ) => expected == actual,
            (
                Expr {
                    kind: ExprKind::BoolLit(expected),
                    ..
                },
                Expr {
                    kind: ExprKind::BoolLit(actual),
                    ..
                },
            ) => expected == actual,
            (
                Expr {
                    kind: ExprKind::Null,
                    ..
                },
                Expr {
                    kind: ExprKind::Null,
                    ..
                },
            ) => true,
            _ => false,
        }
    }

    pub(super) fn union_variant_for_expr(&self, expr: &Expr, variants: &[Type]) -> Type {
        if matches!(
            expr.kind(),
            ExprKind::ArrayLit(_) | ExprKind::ArrayRepeat { .. }
        ) {
            let mut array_variants = variants
                .iter()
                .filter(|variant| matches!(transparent_codegen_type(variant), Type::Array(_)));
            if let (Some(array), None) = (array_variants.next(), array_variants.next()) {
                return array.clone();
            }
        }
        if let ExprKind::ObjectLit(fields) = expr.kind()
            && let Some(discriminated) = variants.iter().find(|variant| {
                fields.iter().any(|(field, value)| {
                    Self::is_narrowing_literal(value)
                        && Self::variant_discriminant_matches(variant, field, value)
                })
            })
        {
            return discriminated.clone();
        }
        if matches!(
            expr.kind(),
            ExprKind::StrLit(_)
                | ExprKind::IntLit(_)
                | ExprKind::UIntLit(_)
                | ExprKind::FloatLit(_)
                | ExprKind::BoolLit(_)
        ) && let Some(literal) = variants
            .iter()
            .find(|variant| matches!(variant, Type::Lit(value) if value.kind() == expr.kind()))
        {
            return literal.clone();
        }
        let Some(hint) = self.expr_type_hint(expr) else {
            return variants[0].clone();
        };
        if let Some(exact) = variants
            .iter()
            .find(|variant| transparent_codegen_type(variant) == transparent_codegen_type(&hint))
        {
            return exact.clone();
        }
        if let Some(assignable) = variants
            .iter()
            .find(|variant| can_codegen_assign_to_variant(expr, &hint, variant))
        {
            return assignable.clone();
        }
        variants[0].clone()
    }

    pub(super) fn emit_if_expr(
        &mut self,
        cond: &Expr,
        then_block: &[Stmt],
        else_block: Option<&[Stmt]>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: if expression outside function")?;

        self.emit_if_stmt(cond, then_block, else_block, current_fn)?;
        Ok(self.context.i32_type().const_int(0, false).into())
    }

    fn emit_conditional_return_owned_bit(&mut self, fn_val: FunctionValue<'ctx>, expr: &Expr) {
        let Ok(name) = fn_val.get_name().to_str() else {
            return;
        };
        if !self.fn_conditionally_owned_returns.contains(name) {
            return;
        }
        let owned_bit = match expr.kind() {
            ExprKind::NewMap { .. } | ExprKind::NewSet { .. } => {
                self.context.bool_type().const_int(1, false)
            }
            ExprKind::Ident(local) => {
                if let Some(bit) = self
                    .conditionally_owned_vars
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(local))
                    .copied()
                {
                    bit
                } else {
                    let owned = self
                        .owned_native_vars
                        .iter()
                        .rev()
                        .find_map(|scope| scope.get(local))
                        .is_some_and(|plan| *plan == ReleasePlan::Deep);
                    self.context.bool_type().const_int(owned as u64, false)
                }
            }
            _ => self.context.bool_type().const_int(0, false),
        };
        let global = self.get_or_declare_return_owned();
        self.builder
            .build_store(global.as_pointer_value(), owned_bit)
            .unwrap();
    }

    pub(super) fn get_or_declare_return_owned(&self) -> GlobalValue<'ctx> {
        if let Some(g) = self.module.get_global("tyrus.return.owned") {
            return g;
        }
        let bool_ty = self.context.bool_type();
        let g = self.module.add_global(bool_ty, None, "tyrus.return.owned");
        g.set_initializer(&bool_ty.const_zero());
        g.set_thread_local(true);
        g
    }

    pub(super) fn declare_setjmp(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("_setjmp") {
            return f;
        }
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let args = if cfg!(windows) {
            vec![ptr_ty.into(), ptr_ty.into()]
        } else {
            vec![ptr_ty.into()]
        };
        let fn_ty = self.context.i32_type().fn_type(&args, false);
        let f = self.module.add_function("_setjmp", fn_ty, None);
        let kind_id = Attribute::get_named_enum_kind_id("returns_twice");
        let attr = self.context.create_enum_attribute(kind_id, 0);
        f.add_attribute(AttributeLoc::Function, attr);
        f
    }

    pub(super) fn setjmp_arguments(
        &self,
        jump_buffer: PointerValue<'ctx>,
    ) -> Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> {
        #[cfg(not(windows))]
        {
            vec![jump_buffer.into()]
        }
        #[cfg(windows)]
        {
            let mut arguments = vec![jump_buffer.into()];
            // The x64 CRT needs the current frame to unwind back through longjmp.
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let frame_address = self
                .module
                .get_function("llvm.frameaddress.p0")
                .unwrap_or_else(|| {
                    self.module.add_function(
                        "llvm.frameaddress.p0",
                        pointer.fn_type(&[self.context.i32_type().into()], false),
                        None,
                    )
                });
            let frame = self
                .builder
                .build_call(
                    frame_address,
                    &[self.context.i32_type().const_zero().into()],
                    "setjmp.frame",
                )
                .unwrap()
                .try_as_basic_value()
                .basic()
                .unwrap();
            arguments.push(frame.into());
            arguments
        }
    }

    pub(super) fn emit_code_has_prefix(
        &mut self,
        code: StructValue<'ctx>,
        prefix: &str,
    ) -> IntValue<'ctx> {
        let string_type = self.string_abi_type();
        let starts_with = self.declare_libc_fn(
            "tyrus_string_starts_with",
            &[string_type, string_type],
            Some(self.context.i64_type().into()),
        );
        let prefix_value = self.const_string_abi(prefix, "error.code.prefix");
        let answer = self
            .builder
            .build_call(
                starts_with,
                &[code.into(), prefix_value.into()],
                "error.code.starts",
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let has_prefix = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                answer,
                self.context.i64_type().const_zero(),
                "error.code.has_prefix",
            )
            .unwrap();
        let generic = self
            .const_string_abi(prefix.trim_end_matches('_'), "error.code.generic")
            .into_struct_value();
        let is_generic = self
            .emit_string_equals(code, generic, "error.code.is_generic")
            .expect("comparing two strings is always emittable");
        self.builder
            .build_or(has_prefix, is_generic, "error.code.names_module")
            .unwrap()
    }

    pub(super) fn emit_async_reject_call(
        &mut self,
        promise: PointerValue<'ctx>,
        rejection: AsyncRejection<'ctx>,
        value: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_ty = self.context.i32_type();
        match rejection {
            AsyncRejection::Message(message) => {
                let reject = self.declare_libc_fn(
                    "tyrus_promise_reject_pending",
                    &[ptr_ty.into(), self.string_abi_type()],
                    Some(i32_ty.into()),
                );
                self.builder
                    .build_call(
                        reject,
                        &[promise.into(), message.into()],
                        "async.throw.reject",
                    )
                    .unwrap();
            }
            AsyncRejection::Structured {
                code_index,
                message_index,
            } => {
                let error = value.into_struct_value();
                let code = self
                    .builder
                    .build_extract_value(error, code_index, "async.throw.code")
                    .map_err(|error| error.to_string())?;
                let message = self
                    .builder
                    .build_extract_value(error, message_index, "async.throw.message")
                    .map_err(|error| error.to_string())?;
                let reject = self.declare_libc_fn(
                    "tyrus_promise_reject_pending_structured",
                    &[
                        ptr_ty.into(),
                        self.string_abi_type(),
                        self.string_abi_type(),
                    ],
                    Some(i32_ty.into()),
                );
                self.builder
                    .build_call(
                        reject,
                        &[promise.into(), code.into(), message.into()],
                        "async.throw.reject.structured",
                    )
                    .unwrap();
            }
        }
        Ok(())
    }

    fn take_field_moved_by_declaration(
        &mut self,
        binding: &str,
        value: &Expr,
        val: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> Result<(), String> {
        let ExprKind::FieldAccess { object, field } = value.kind() else {
            return Ok(());
        };
        let declared = self.resolve_type(ty);
        if transparent_codegen_type(&declared).is_copy() {
            return Ok(());
        }
        let Some(root) = field_move_root(value) else {
            return Ok(());
        };
        if self.load_var(root).is_none() {
            return Ok(());
        }
        if self.async_frame_owns_value(binding) {
            return Ok(());
        }
        let Some(owner_type) = self.expr_type_hint(object).map(|ty| self.resolve_type(&ty)) else {
            return Ok(());
        };
        let Some(storage) = self
            .struct_field_type(&owner_type, field)
            .map(|ty| self.resolve_type(&ty))
        else {
            return Ok(());
        };
        if transparent_codegen_type(&storage) != transparent_codegen_type(&declared) {
            return Ok(());
        }
        if let Some(plan) = self.field_release_plan(value) {
            self.record_owned_native_result_with(val, declared.clone(), plan);
        }
        let (slot, _) = self.emit_place_ptr(value)?;
        let zero = self.llvm_type(&storage).const_zero();
        self.builder
            .build_store(slot, zero)
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn field_release_plan(&self, place: &Expr) -> Option<ReleasePlan> {
        match place.kind() {
            ExprKind::Ident(name) => {
                if self.async_frame_owns_value(name) {
                    Some(ReleasePlan::Deep)
                } else {
                    self.owned_native_plan_for(name)
                }
            }
            ExprKind::FieldAccess { object, field } => {
                let parent = self.field_release_plan(object)?;
                let owner_type = self
                    .expr_type_hint(object)
                    .map(|ty| self.resolve_type(&ty))?;
                let index = self.struct_field_index(&owner_type, field)? as usize;
                match parent {
                    ReleasePlan::Deep => Some(ReleasePlan::Deep),
                    ReleasePlan::Fields(plans) => plans
                        .into_iter()
                        .find(|(candidate, _)| *candidate == index)
                        .map(|(_, plan)| plan),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn emit_pending_finally_since(
        &mut self,
        fn_val: FunctionValue<'ctx>,
        depth: usize,
    ) -> Result<(), String> {
        let saved = self.finally_scopes.clone();
        let result = (|| {
            while self.finally_scopes.len() > depth {
                let scope = self
                    .finally_scopes
                    .pop()
                    .expect("internal: missing finally scope");
                for stmt in &scope.body {
                    if self.current_block_terminated() {
                        break;
                    }
                    self.emit_stmt(stmt, fn_val)?;
                }
                if self.current_block_terminated() {
                    break;
                }
            }
            Ok(())
        })();
        self.finally_scopes = saved;
        result
    }

    fn return_exit_reason(&self, expr: Option<&Expr>) -> ScopeExitReason {
        match expr.map(Expr::kind) {
            Some(ExprKind::FnCall { callee, .. }) if callee == "Err" => {
                ScopeExitReason::ErrorReturn
            }
            Some(ExprKind::None) if matches!(self.return_types.last(), Some(Type::Optional(_))) => {
                ScopeExitReason::ErrorReturn
            }
            _ => ScopeExitReason::Return,
        }
    }

    fn emit_pending_validation_cleanups_since(&self, depth: usize) {
        for scope in self.validation_cleanup_scopes[depth..].iter().rev() {
            if !scope.transferred {
                self.emit_validation_error_cleanup_slot(scope.error_slot);
            }
        }
    }

    pub(super) fn emit_pending_finally(
        &mut self,
        fn_val: FunctionValue<'ctx>,
        for_throw: bool,
        exiting_loop_depth: Option<usize>,
    ) -> Result<(), String> {
        if self.async_catch_cleanup_scopes.last().is_some_and(|scope| {
            for_throw
                || exiting_loop_depth.is_none()
                || exiting_loop_depth.is_some_and(|depth| depth <= scope.loop_depth)
        }) {
            self.emit_active_async_catch_cleanup(fn_val)?;
        }
        let saved = self.finally_scopes.clone();
        while let Some(scope) = self.finally_scopes.pop() {
            if for_throw && !scope.catches_throw {
                break;
            }
            if exiting_loop_depth.is_some_and(|depth| scope.loop_depth < depth) {
                continue;
            }
            for stmt in &scope.body {
                if self.current_block_terminated() {
                    break;
                }
                self.emit_stmt(stmt, fn_val)?;
            }
            if self.current_block_terminated() {
                break;
            }
        }
        self.finally_scopes = saved;
        Ok(())
    }

    pub(super) fn emit_pending_validation_cleanups(&self, exiting_loop_depth: Option<usize>) {
        for scope in self.validation_cleanup_scopes.iter().rev() {
            if scope.transferred {
                continue;
            }
            if exiting_loop_depth.is_some_and(|depth| scope.loop_depth < depth) {
                continue;
            }
            self.emit_validation_error_cleanup_slot(scope.error_slot);
        }
    }
}

fn can_codegen_assign_to_variant(expr: &Expr, source: &Type, variant: &Type) -> bool {
    let source = transparent_codegen_type(source);
    let variant = transparent_codegen_type(variant);
    if source == variant {
        return true;
    }
    match expr.kind() {
        ExprKind::IntLit(_) => is_integer_type(variant) || is_float_type(variant),
        ExprKind::UIntLit(_) => matches!(variant, Type::U128),
        ExprKind::FloatLit(_) => is_float_type(variant) || is_integer_type(variant),
        _ => {
            (is_integer_type(source) && (is_integer_type(variant) || is_float_type(variant)))
                || (is_float_type(source) && (is_float_type(variant) || is_integer_type(variant)))
        }
    }
}

fn assign_op_binop(op: &AssignOp) -> Option<BinOp> {
    Some(match op {
        AssignOp::AddAssign => BinOp::Add,
        AssignOp::SubAssign => BinOp::Sub,
        AssignOp::MulAssign => BinOp::Mul,
        AssignOp::DivAssign => BinOp::Div,
        AssignOp::ModAssign => BinOp::Mod,
        AssignOp::PowAssign => BinOp::Pow,
        AssignOp::BitAndAssign => BinOp::BitAnd,
        AssignOp::BitOrAssign => BinOp::BitOr,
        AssignOp::BitXorAssign => BinOp::BitXor,
        AssignOp::ShlAssign => BinOp::Shl,
        AssignOp::ShrAssign => BinOp::Shr,
        AssignOp::UShrAssign => BinOp::UShr,
        AssignOp::Assign
        | AssignOp::AndAssign
        | AssignOp::OrAssign
        | AssignOp::NullCoalesceAssign => return None,
    })
}

pub(super) fn codegen_expression_path(expr: &Expr) -> Option<String> {
    match expr.kind() {
        ExprKind::Ident(name) => Some(name.clone()),
        ExprKind::FieldAccess { object, field } => {
            Some(format!("{}.{field}", codegen_expression_path(object)?))
        }
        _ => None,
    }
}

fn field_move_root(expr: &Expr) -> Option<&str> {
    match expr.kind() {
        ExprKind::FieldAccess { object, .. } => match object.kind() {
            ExprKind::Ident(name) => Some(name),
            ExprKind::FieldAccess { .. } => field_move_root(object),
            _ => None,
        },
        _ => None,
    }
}
