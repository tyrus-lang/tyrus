#![allow(
    clippy::obfuscated_if_else,
    reason = "boolean `then` keeps async continuation construction aligned with its suspension-site metadata"
)]

use super::*;
use crate::statements::ErrorSubtypeStorage;
use tyrus_core::ast::{Expr, ExprKind, OnFailHandler, Stmt, StmtKind};
use tyrus_core::async_shapes::{
    async_expression_children, async_prefix_requires_storage, collect_await_expressions,
    collect_prefixes_before_await, expression_contains_await_span, first_await_expression,
    guarded_nested_condition_await, nested_condition_await,
};
use tyrus_core::async_types::{
    owned_copy_array_frame_method, owned_string_array_frame_method, owned_string_frame_method,
    static_string_array_frame_literal,
};
use tyrus_core::span::Span;

#[path = "throw_regions/throw_regions.rs"]
pub(crate) mod throw_regions;

#[path = "layout/layout.rs"]
mod layout;

#[path = "validation/validation.rs"]
mod validation;
use validation::validate_codegen_item;

#[path = "async_analysis/async_analysis.rs"]
mod async_analysis;
#[path = "async_errors/async_errors.rs"]
mod async_errors;
#[path = "async_frame/async_frame.rs"]
mod async_frame;
#[path = "async_segments/async_segments.rs"]
mod async_segments;
#[path = "async_support/async_support.rs"]
mod async_support;
#[path = "async_values/async_values.rs"]
mod async_values;

#[cfg(test)]
#[path = "db_async_returning_tests.rs"]
mod db_async_returning_tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AsyncLocalStorage {
    None,
    OwnedString,
    TrackedString,
    CopyArray,
    StaticStringArray,
    OwnedStringArray,
    TrackedCopyArray,
    TrackedStringArray,
    StaticObject,
    OwnedValue,
}

const UNION_PAYLOAD_ALIGNMENT: u32 = 8;

impl<'ctx> Codegen<'ctx> {
    pub fn new(
        context: &'ctx Context,
        module_name: &str,
        _optimize: bool,
        target: CompileTarget,
    ) -> Self {
        let module = context.create_module(module_name);
        let triple = if target.triple().is_empty() {
            TargetMachine::get_default_triple()
        } else {
            inkwell::targets::TargetTriple::create(target.triple())
        };
        module.set_triple(&triple);
        // Without a data layout LLVM falls back to defaults that disagree with
        // the platform ABI the runtime is compiled for: `i128` gets alignment
        // 8, so a `decimal` slot handed to a runtime function that writes a
        // 16-aligned `TyrusDecimal` through it is misaligned. That is undefined
        // behaviour, and the debug runtime aborts on it.
        // `Target::from_triple` answers only for a target that has been
        // registered, and nothing else in the compiler registers one: native
        // code is produced by handing the `.ll` file to clang, so no
        // `TargetMachine` was ever needed.
        let _ = inkwell::targets::Target::initialize_native(
            &inkwell::targets::InitializationConfig::default(),
        );
        inkwell::targets::Target::initialize_webassembly(
            &inkwell::targets::InitializationConfig::default(),
        );
        if let Ok(resolved) = Target::from_triple(&triple)
            && let Some(machine) = resolved.create_target_machine(
                &triple,
                "",
                "",
                inkwell::OptimizationLevel::None,
                RelocMode::Default,
                CodeModel::Default,
            )
        {
            module.set_data_layout(&machine.get_target_data().get_data_layout());
        }

        let mut interfaces = tyrus_core::builtin_interfaces::builtin_interfaces();
        for interface in tyrus_core::cli::option_interfaces() {
            interfaces.insert(interface.name.to_string(), interface.fields);
        }
        interfaces.extend(tyrus_core::async_types::error_interfaces());
        for (ns_name, fields) in tyrus_core::crypto::crypto_namespace_interface_fields() {
            interfaces.entry(ns_name).or_insert(fields);
        }
        interfaces
            .entry("DigestStream".to_string())
            .or_insert_with(tyrus_core::crypto::digest_stream_methods);
        Self {
            context,
            module,
            builder: context.create_builder(),
            vars: vec![HashMap::default()],
            var_concrete_types: vec![HashMap::default()],
            non_zero_vars: vec![HashSet::default()],
            narrowed_paths: vec![HashSet::default()],
            owned_closure_vars: vec![HashSet::default()],
            owned_closure_drop_fns: vec![HashMap::default()],
            owned_native_vars: vec![HashMap::default()],
            optional_chain_depth: 0,
            for_index_hint: None,
            pending_owned_native_results: Vec::new(),
            async_capture_param_count: 0,
            pending_owned_native_guard: Vec::new(),
            pending_conditionally_owned_results: Vec::new(),
            conditionally_owned_vars: vec![HashMap::default()],
            mutable_string_ownership: vec![HashMap::default()],
            string_array_ownership: vec![HashMap::default()],
            pending_closure_drop_fn: None,
            capture_test_assertions: false,
            contained_arrays: tyrus_core::escape_analysis::ContainedArrays::default(),
            constant_global_arrays: tyrus_core::escape_analysis::ConstantGlobalArrays::default(),
            checked_indexes: tyrus_core::bounds_check::CheckedIndexes::default(),
            confined_closures: tyrus_core::escape_analysis::ConfinedClosures::default(),
            confined_closure_pending: false,
            closure_env_on_stack: false,
            stack_array_pending: false,
            array_literal_on_stack: false,
            fns: HashMap::default(),
            auto_route_exports: Vec::new(),
            fn_signatures: HashMap::default(),
            user_validator_names: HashSet::default(),
            extension_methods: Vec::new(),
            behavior_targets: HashMap::default(),
            fn_owned_returns: HashSet::default(),
            fn_db_async_returns: HashSet::default(),
            for_of_iterable_cleanups: Vec::new(),
            interned_strings: std::cell::RefCell::new(HashMap::default()),
            fn_conditionally_owned_returns: HashSet::default(),
            fn_throws: HashMap::default(),
            throwing_closures: HashSet::default(),
            throw_frames: Vec::new(),
            error_subtypes: std::collections::HashSet::default(),
            catch_error_subtypes: Vec::new(),
            next_assert_throws_id: 0,
            async_functions: HashSet::default(),
            fn_param_metadata: HashMap::default(),
            generic_fns: HashMap::default(),
            generic_specializations: HashMap::default(),
            type_substitutions: Vec::new(),
            return_types: Vec::new(),
            async_results: Vec::new(),
            async_temporaries_tracked: 0,
            async_frame_slots: Vec::new(),
            async_frame_string_ownership: Vec::new(),
            async_frame_tracked_copy_arrays: Vec::new(),
            async_frame_string_array_ownership: Vec::new(),
            async_frame_owned_values: Vec::new(),
            async_frame_owned_initialization: Vec::new(),
            async_frame_local_storage: Vec::new(),
            async_await_values: Vec::new(),
            async_prescan_types: HashMap::default(),
            type_aliases: HashMap::default(),
            native_layout_interfaces: interfaces.keys().cloned().collect(),
            interfaces,
            declared_interfaces: HashMap::default(),
            db_models: HashMap::default(),
            enums: tyrus_core::builtin_interfaces::builtin_enums(),
            arrow_fn_count: 0,
            main_block: None,
            validation_depth: 0,
            loop_stack: Vec::new(),
            break_targets: Vec::new(),
            deferred_scopes: vec![Vec::new()],
            finally_scopes: Vec::new(),
            async_finally_completion_scopes: Vec::new(),
            async_catch_cleanup_scopes: Vec::new(),
            catch_context_restores: Vec::new(),
            catch_binding_types: Vec::new(),
            async_throw_regions: Vec::new(),
            catch_error_owned_slots: Vec::new(),
            match_result_bindings: Vec::new(),
            catch_binding_releases: Vec::new(),
            async_error_cleanups: Vec::new(),
            validation_cleanup_scopes: Vec::new(),
            env_startup: None,
            worker_threads: None,
            development_https: false,
            database_path: None,
            env_var_types: HashMap::default(),
            env_validation_mode: false,
            validation_target_path: Vec::new(),
            topic_values: Vec::new(),
            target,
        }
    }

    pub fn set_env_schema(&mut self, schema: &tyrus_core::env::EnvSchema) {
        self.set_env_schema_with_init(
            schema,
            super::EnvInit::File {
                path: schema.env_file.clone(),
            },
        );
    }

    pub fn set_worker_threads(&mut self, threads: u32) {
        self.worker_threads = Some(threads);
    }

    pub fn set_database_path(&mut self, path: &str) {
        self.database_path = Some(path.to_string());
    }

    pub fn set_development_https(&mut self) {
        self.development_https = true;
    }

    pub fn set_sealed_env_schema(
        &mut self,
        schema: &tyrus_core::env::EnvSchema,
        sealed: &super::SealedEnvMetadata,
    ) {
        self.set_env_schema_with_init(
            schema,
            super::EnvInit::Sealed {
                project_name: sealed.project_name.clone(),
                fingerprint: sealed.fingerprint.clone(),
            },
        );
    }

    fn set_env_schema_with_init(
        &mut self,
        schema: &tyrus_core::env::EnvSchema,
        init: super::EnvInit,
    ) {
        self.env_var_types = schema
            .vars
            .iter()
            .map(|var| (var.name.clone(), var.declared_type.clone()))
            .collect();
        self.env_startup = Some(super::EnvStartup {
            init,
            vars: schema
                .vars
                .iter()
                .map(|var| super::EnvStartupVar {
                    name: var.name.clone(),
                    ty: var.declared_type.clone(),
                    message: var.type_message.clone(),
                    decorators: var.decorators.clone(),
                })
                .collect(),
        });
    }

    pub(super) fn push_scope(&mut self) {
        self.vars.push(HashMap::default());
        self.var_concrete_types.push(HashMap::default());
        self.non_zero_vars.push(HashSet::default());
        self.narrowed_paths.push(HashSet::default());
        self.owned_closure_vars.push(HashSet::default());
        self.owned_closure_drop_fns.push(HashMap::default());
        self.owned_native_vars.push(HashMap::default());
        self.conditionally_owned_vars.push(HashMap::default());
        self.mutable_string_ownership.push(HashMap::default());
        self.string_array_ownership.push(HashMap::default());
        self.deferred_scopes.push(Vec::new());
    }
    pub(super) fn pop_scope(&mut self) {
        if !self.current_block_terminated() {
            self.emit_deferred_finalizers_in_current_scope(ScopeExitReason::Normal)
                .expect("internal: validated deferred finalizer failed during code generation");
            let _ =
                self.emit_owned_closure_cleanups_in_scope(self.owned_closure_vars.len() - 1, None);
            self.emit_owned_native_cleanups_in_scope(self.owned_native_vars.len() - 1, None);
            self.emit_conditionally_owned_cleanups_in_scope(
                self.conditionally_owned_vars.len() - 1,
                None,
            );
            self.emit_mutable_string_ownership_cleanups_in_scope(
                self.mutable_string_ownership.len() - 1,
                None,
            );
            self.emit_string_array_ownership_cleanups_in_scope(
                self.string_array_ownership.len() - 1,
                None,
            );
        }
        self.vars.pop();
        let depth = self.vars.len();
        self.catch_error_subtypes
            .retain(|storage| storage.scope_depth <= depth);
        self.var_concrete_types.pop();
        self.non_zero_vars.pop();
        self.narrowed_paths.pop();
        self.owned_closure_vars.pop();
        self.owned_closure_drop_fns.pop();
        self.owned_native_vars.pop();
        self.conditionally_owned_vars.pop();
        self.mutable_string_ownership.pop();
        self.string_array_ownership.pop();
        self.deferred_scopes.pop();
    }

    pub(super) fn store_var(&mut self, name: &str, ptr: PointerValue<'ctx>, ty: Type) {
        if let Some(scope) = self.vars.last_mut() {
            scope.insert(name.to_string(), (ptr, ty));
        }
    }

    pub(super) fn mark_owned_closure(&mut self, name: &str) {
        if let Some(scope) = self.owned_closure_vars.last_mut() {
            scope.insert(name.to_string());
        }
        if let Some(drop) = self.pending_closure_drop_fn.take()
            && let Some(scope) = self.owned_closure_drop_fns.last_mut()
        {
            scope.insert(name.to_string(), drop);
        }
    }

    pub(super) fn mark_owned_dynamic_closure(&mut self, name: &str) {
        if let Some(scope) = self.owned_closure_vars.last_mut() {
            scope.insert(name.to_string());
        }
    }

    pub(super) fn is_owned_closure_binding(&self, name: &str) -> bool {
        self.owned_closure_vars
            .iter()
            .any(|scope| scope.contains(name))
    }

    pub(super) fn is_temporary_closure_argument(&self, argument: &Expr, param_type: &Type) -> bool {
        matches!(
            transparent_codegen_type(&self.resolve_type(param_type)),
            Type::Function { .. }
        ) && matches!(
            argument.kind(),
            ExprKind::ArrowFn { .. }
                | ExprKind::FnCall { .. }
                | ExprKind::MethodCall { .. }
                | ExprKind::CallValue { .. }
        )
    }

    fn closure_environment(&self, value: BasicValueEnum<'ctx>) -> PointerValue<'ctx> {
        self.builder
            .build_extract_value(value.into_struct_value(), 1, "closure.env")
            .unwrap()
            .into_pointer_value()
    }

    pub(super) fn emit_closure_release(
        &mut self,
        value: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let environment = self.closure_environment(value);
        let release = self.declare_libc_fn("tyrus_closure_release", &[pointer.into()], None);
        self.builder
            .build_call(release, &[environment.into()], "")
            .unwrap();
        Ok(())
    }

    pub(super) fn emit_closure_retain(&mut self, value: BasicValueEnum<'ctx>) {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let environment = self.closure_environment(value);
        let retain = self.declare_libc_fn("tyrus_closure_retain", &[pointer.into()], None);
        self.builder
            .build_call(retain, &[environment.into()], "")
            .unwrap();
    }

    fn emit_owned_closure_cleanups_in_scope(
        &mut self,
        scope_index: usize,
        exclude: Option<&str>,
    ) -> Result<(), String> {
        let mut names = self.owned_closure_vars[scope_index]
            .iter()
            .filter(|name| exclude != Some(name.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        names.sort();
        for name in names {
            let Some((slot, ty)) = self.vars[scope_index].get(&name).cloned() else {
                continue;
            };
            let value = self
                .builder
                .build_load(self.llvm_type(&ty), slot, "closure.cleanup")
                .map_err(|error| error.to_string())?;
            if let Some(drop) = self.owned_closure_drop_fns[scope_index].get(&name).copied() {
                let environment = self.closure_environment(value);
                self.builder
                    .build_call(drop, &[environment.into()], "")
                    .unwrap();
            } else {
                self.emit_closure_release(value)?;
            }
        }
        Ok(())
    }

    pub(super) fn emit_owned_value_cleanups(
        &mut self,
        exclude: Option<&str>,
    ) -> Result<(), String> {
        self.emit_owned_value_cleanups_to_depth(0, exclude)
    }

    pub(crate) fn current_emitting_function_is_main(&self) -> bool {
        self.builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .is_some_and(|function| function.get_name().to_str() == Ok("main"))
    }

    pub(super) fn emit_owned_value_cleanups_to_depth(
        &mut self,
        depth: usize,
        exclude: Option<&str>,
    ) -> Result<(), String> {
        // Scope 0 is the module-global scope: the `main` function owns and
        // releases the module globals there (via `finish_main`). Any other
        // function must only release what its own scopes hold, unwinding to
        // scope 0 from a helper would free module globals it does not own,
        // leaving them dangling for `main` (double-free), and in the REPL
        // that also frees globals the persistent session still needs.
        let floor = if self.current_emitting_function_is_main() {
            depth
        } else {
            depth.max(1)
        };
        for scope_index in (floor..self.owned_closure_vars.len()).rev() {
            self.emit_owned_closure_cleanups_in_scope(scope_index, exclude)?;
        }
        for scope_index in (floor..self.owned_native_vars.len()).rev() {
            self.emit_owned_native_cleanups_in_scope(scope_index, exclude);
        }
        for scope_index in (floor..self.conditionally_owned_vars.len()).rev() {
            self.emit_conditionally_owned_cleanups_in_scope(scope_index, exclude);
        }
        for scope_index in (floor..self.mutable_string_ownership.len()).rev() {
            self.emit_mutable_string_ownership_cleanups_in_scope(scope_index, exclude);
        }
        for scope_index in (floor..self.string_array_ownership.len()).rev() {
            self.emit_string_array_ownership_cleanups_in_scope(scope_index, exclude);
        }
        Ok(())
    }

    pub(super) fn emit_owned_closure_cleanups(
        &mut self,
        exclude: Option<&str>,
    ) -> Result<(), String> {
        self.emit_owned_value_cleanups(exclude)?;
        self.emit_catch_binding_releases_on_exit(exclude)?;
        self.emit_active_async_error_cleanups(exclude);
        for (catch_top, previous, _) in self.catch_context_restores.iter().rev() {
            self.builder.build_store(*catch_top, *previous).unwrap();
        }
        Ok(())
    }

    /// Whether `emit_owned_value_cleanups_to_depth(depth, None)` would release
    /// anything, with the same floor.
    pub(crate) fn has_owned_value_cleanups_to_depth(&self, depth: usize) -> bool {
        let floor = if self.current_emitting_function_is_main() {
            depth
        } else {
            depth.max(1)
        };
        self.owned_closure_vars
            .iter()
            .skip(floor)
            .any(|scope| !scope.is_empty())
            || self
                .owned_native_vars
                .iter()
                .skip(floor)
                .any(|scope| !scope.is_empty())
            || self
                .conditionally_owned_vars
                .iter()
                .skip(floor)
                .any(|scope| !scope.is_empty())
            || self
                .mutable_string_ownership
                .iter()
                .skip(floor)
                .any(|scope| !scope.is_empty())
            || self
                .string_array_ownership
                .iter()
                .skip(floor)
                .any(|scope| !scope.is_empty())
    }

    pub(crate) fn enclosing_catch_scope_depth(&self) -> usize {
        self.catch_context_restores
            .last()
            .map(|(_, _, depth)| *depth)
            .filter(|depth| *depth < self.owned_native_vars.len())
            .unwrap_or(0)
    }

    pub(super) fn emit_recoverable_runtime_error_cleanups(&mut self) -> Result<(), String> {
        self.emit_pending_owned_native_result_cleanup();
        let catch_scope_depth = self
            .catch_context_restores
            .last()
            .map_or(0, |(_, _, depth)| *depth);
        self.emit_owned_value_cleanups_to_depth(catch_scope_depth, None)
    }

    pub(super) fn emit_owned_native_release(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        plan: &ReleasePlan,
    ) {
        if matches!(transparent_codegen_type(ty), Type::Void | Type::Never) {
            return;
        }
        match plan {
            ReleasePlan::Deep => self.emit_owned_native_value_free(value, ty),
            ReleasePlan::Buffer => self.emit_owned_native_buffer_free(value),
            ReleasePlan::Value => {
                if self.is_string_value(value) {
                    self.emit_string_release(value.into_struct_value())
                        .expect("releasing an owned string is always emittable");
                } else {
                    let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                    let free = self.declare_libc_fn("free", &[pointer.into()], None);
                    self.builder
                        .build_call(free, &[value.into_pointer_value().into()], "")
                        .unwrap();
                }
            }
            ReleasePlan::Fields(plans) => {
                let resolved = self.resolve_type(ty);
                let fields = match transparent_codegen_type(&resolved) {
                    Type::Named(name) => self.interfaces.get(name).cloned(),
                    Type::Object(fields) => Some(fields.clone()),
                    _ => None,
                };
                let Some(fields) = fields else {
                    return;
                };
                for (index, field_plan) in plans.clone() {
                    let Some(field) = fields.get(index) else {
                        continue;
                    };
                    let field_type = self.resolve_type(&field_storage_type(field));
                    let field_value = self
                        .builder
                        .build_extract_value(
                            value.into_struct_value(),
                            index as u32,
                            "native.cleanup.field",
                        )
                        .unwrap();
                    self.emit_owned_native_release(field_value, &field_type, &field_plan);
                }
            }
            ReleasePlan::ArrayElements(indices) => {
                let resolved = self.resolve_type(ty);
                let Type::Array(element) = transparent_codegen_type(&resolved) else {
                    return;
                };
                let element = element.clone();
                let element_type = self.llvm_type(&element);
                let usize_type = self.usize_type();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let data = self
                    .builder
                    .build_extract_value(value.into_struct_value(), 0, "native.cleanup.array.data")
                    .unwrap()
                    .into_pointer_value();
                for &index in indices {
                    let offset = usize_type.const_int(index as u64, false);
                    let slot = unsafe {
                        self.builder
                            .build_gep(element_type, data, &[offset], "native.cleanup.array.slot")
                            .unwrap()
                    };
                    let element_value = self
                        .builder
                        .build_load(element_type, slot, "native.cleanup.array.element")
                        .unwrap();
                    self.emit_owned_native_release(element_value, &element, &ReleasePlan::Deep);
                }
                let free = self.declare_libc_fn("free", &[pointer.into()], None);
                self.builder.build_call(free, &[data.into()], "").unwrap();
            }
            ReleasePlan::Box(inner_plan) => {
                let resolved = self.resolve_type(ty);
                let Type::Optional(inner_ty) = transparent_codegen_type(&resolved) else {
                    return;
                };
                let inner_ty = inner_ty.clone();
                let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
                let source = value.into_pointer_value();
                let is_null = self
                    .builder
                    .build_is_null(source, "native.cleanup.box.is_null")
                    .unwrap();
                let function = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
                    .unwrap();
                let free_block = self
                    .context
                    .append_basic_block(function, "native.cleanup.box.present");
                let done_block = self
                    .context
                    .append_basic_block(function, "native.cleanup.box.done");
                self.builder
                    .build_conditional_branch(is_null, done_block, free_block)
                    .unwrap();
                self.builder.position_at_end(free_block);
                let inner_value = self
                    .builder
                    .build_load(
                        self.llvm_type(&inner_ty),
                        source,
                        "native.cleanup.box.inner",
                    )
                    .unwrap();
                self.emit_owned_native_release(inner_value, &inner_ty, inner_plan);
                let free = self.declare_libc_fn("free", &[pointer.into()], None);
                self.builder.build_call(free, &[source.into()], "").unwrap();
                self.builder.build_unconditional_branch(done_block).unwrap();
                self.builder.position_at_end(done_block);
            }
        }
    }

    pub(super) fn emit_pointer_free(&mut self, pointer_value: PointerValue<'ctx>) {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[pointer_value.into()], "")
            .unwrap();
    }

    pub(super) fn emit_owned_native_buffer_free(&mut self, value: BasicValueEnum<'ctx>) {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let data = self
            .builder
            .build_extract_value(value.into_struct_value(), 0, "native.cleanup.buffer")
            .unwrap()
            .into_pointer_value();
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder.build_call(free, &[data.into()], "").unwrap();
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn emit_owned_native_value_free(&mut self, value: BasicValueEnum<'ctx>, ty: &Type) {
        if self.async_owned_value_supported(ty)
            && let Some(function) = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
        {
            let slot = self
                .entry_alloca(self.llvm_type(ty), "native.cleanup.owned")
                .unwrap();
            self.builder.build_store(slot, value).unwrap();
            self.emit_async_drop_value_in_place(function, slot, ty)
                .expect("internal: supported native owned value has no drop glue");
            return;
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let object_fields = match transparent_codegen_type(ty) {
            Type::Named(name) if name != "DigestStream" => self.interfaces.get(name).cloned(),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        };
        if let Some(fields) = object_fields {
            let object = value.into_struct_value();
            for (index, field) in fields.iter().enumerate() {
                let field_type = self.resolve_type(&field_storage_type(field));
                if !matches!(transparent_codegen_type(&field_type), Type::Str) {
                    continue;
                }
                let field_value = self
                    .builder
                    .build_extract_value(object, index as u32, "native.cleanup.object.field")
                    .unwrap()
                    .into_struct_value();
                self.emit_string_release(field_value)
                    .expect("releasing a native object's string field is always emittable");
            }
            return;
        }
        if let Type::Array(element) = transparent_codegen_type(ty)
            && !element.is_copy()
        {
            let slot = self
                .entry_alloca(self.llvm_type(ty), "native.cleanup.owned.array")
                .expect("owned arrays are released inside a function");
            self.builder.build_store(slot, value).unwrap();
            let drop_element = self
                .emit_async_array_element_drop(element)
                .expect("supported owned array elements have drop glue");
            let element_size = self
                .llvm_type(element)
                .size_of()
                .expect("owned array elements are sized");
            let drop_contents = self.declare_libc_fn(
                "tyrus_array_drop_owned_contents",
                &[
                    pointer.into(),
                    self.context.i64_type().into(),
                    pointer.into(),
                ],
                None,
            );
            self.builder
                .build_call(
                    drop_contents,
                    &[
                        slot.into(),
                        element_size.into(),
                        drop_element.as_global_value().as_pointer_value().into(),
                    ],
                    "native.cleanup.owned.array",
                )
                .unwrap();
            return;
        }
        let allocation = match transparent_codegen_type(ty) {
            Type::Str | Type::Regex => {
                self.emit_string_release(value.into_struct_value())
                    .expect("releasing a native string result is always emittable");
                return;
            }
            Type::Array(element) if element.is_copy() => self
                .builder
                .build_extract_value(value.into_struct_value(), 0, "native.cleanup.data")
                .unwrap()
                .into_pointer_value(),
            // The synchronous half of what the async frame's drop glue does for
            // a router. It is the same missing release, and the synchronous
            // form only looked healthy: a router built in `main` sits in a
            // stack slot still live at `exit`, so the pointer stays reachable
            // and no detector calls it a leak. Build one in a function that
            // returns something else and the frame goes away with the only
            // pointer to it, which is when the same missing release starts
            // being reported.
            Type::Named(name) if name == "HttpRouter" => {
                let free = self.declare_libc_fn("tyrus_http_router_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "http.router.drop")
                    .unwrap();
                return;
            }
            Type::Named(name) if name == "DigestStream" => {
                let free = self.declare_libc_fn(
                    "tyrus_crypto_digest_stream_free",
                    &[pointer.into()],
                    None,
                );
                self.builder
                    .build_call(free, &[value.into()], "digest.stream.drop")
                    .unwrap();
                return;
            }
            Type::Named(name) if name == "ChildProcess" => {
                let free =
                    self.declare_libc_fn("tyrus_child_process_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "process.child.drop")
                    .unwrap();
                return;
            }
            Type::Named(name) if name == "WebSocket" => {
                let free = self.declare_libc_fn("tyrus_websocket_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "websocket.drop")
                    .unwrap();
                return;
            }
            Type::Map(_, _) => {
                let free = self.declare_libc_fn("tyrus_map_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "map.drop")
                    .unwrap();
                return;
            }
            Type::Set(_) => {
                let free = self.declare_libc_fn("tyrus_set_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "set.drop")
                    .unwrap();
                return;
            }
            Type::BinaryHeap(_) => {
                let free = self.declare_libc_fn("tyrus_binary_heap_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "binary_heap.drop")
                    .unwrap();
                return;
            }
            Type::BinomialHeap(_) => {
                let free =
                    self.declare_libc_fn("tyrus_binomial_heap_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "binomial_heap.drop")
                    .unwrap();
                return;
            }
            Type::UnionFind(_) => {
                let free = self.declare_libc_fn("tyrus_union_find_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "union_find.drop")
                    .unwrap();
                return;
            }
            Type::Counter(_) => {
                let free = self.declare_libc_fn("tyrus_counter_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "counter.drop")
                    .unwrap();
                return;
            }
            Type::Deque(_) => {
                let free = self.declare_libc_fn("tyrus_deque_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "deque.drop")
                    .unwrap();
                return;
            }
            Type::FenwickTree => {
                let free = self.declare_libc_fn("tyrus_fenwick_tree_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "fenwick_tree.drop")
                    .unwrap();
                return;
            }
            Type::BubbleSort => {
                let free = self.declare_libc_fn("tyrus_bubble_sort_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "bubble_sort.drop")
                    .unwrap();
                return;
            }
            Type::SegmentTree => {
                let free = self.declare_libc_fn("tyrus_segment_tree_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "segment_tree.drop")
                    .unwrap();
                return;
            }
            Type::SparseTable => {
                let free = self.declare_libc_fn("tyrus_sparse_table_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "sparse_table.drop")
                    .unwrap();
                return;
            }
            Type::AvlTree(_) => {
                let free = self.declare_libc_fn("tyrus_avl_tree_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "avl_tree.drop")
                    .unwrap();
                return;
            }
            Type::RedBlackTree(_) => {
                let free =
                    self.declare_libc_fn("tyrus_red_black_tree_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "red_black_tree.drop")
                    .unwrap();
                return;
            }
            Type::SplayTree(_) => {
                let free = self.declare_libc_fn("tyrus_splay_tree_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "splay_tree.drop")
                    .unwrap();
                return;
            }
            Type::Treap(_) => {
                let free = self.declare_libc_fn("tyrus_treap_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "treap.drop")
                    .unwrap();
                return;
            }
            Type::SortedSet(_) => {
                let free = self.declare_libc_fn("tyrus_sorted_set_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "sorted_set.drop")
                    .unwrap();
                return;
            }
            Type::SkipList(_) => {
                let free = self.declare_libc_fn("tyrus_skip_list_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "skip_list.drop")
                    .unwrap();
                return;
            }
            Type::SortedMap(_, _) => {
                let free = self.declare_libc_fn("tyrus_sorted_map_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "sorted_map.drop")
                    .unwrap();
                return;
            }
            Type::Graph(_, _) => {
                let free = self.declare_libc_fn("tyrus_graph_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "graph.drop")
                    .unwrap();
                return;
            }
            Type::SegmentTreeLazy => {
                let free =
                    self.declare_libc_fn("tyrus_segment_tree_lazy_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "segment_tree_lazy.drop")
                    .unwrap();
                return;
            }
            Type::BitSet => {
                let free = self.declare_libc_fn("tyrus_bit_set_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "bitset.drop")
                    .unwrap();
                return;
            }
            Type::LinkedList(_) => {
                let free = self.declare_libc_fn("tyrus_linked_list_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "linked_list.drop")
                    .unwrap();
                return;
            }
            Type::DoubleLinkedList(_) => {
                let free =
                    self.declare_libc_fn("tyrus_double_linked_list_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "double_linked_list.drop")
                    .unwrap();
                return;
            }
            Type::Trie => {
                let free = self.declare_libc_fn("tyrus_trie_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "trie.drop")
                    .unwrap();
                return;
            }
            Type::SuffixTree => {
                let free = self.declare_libc_fn("tyrus_suffix_tree_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "suffix_tree.drop")
                    .unwrap();
                return;
            }
            Type::Lca => {
                let free = self.declare_libc_fn("tyrus_lca_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "lca.drop")
                    .unwrap();
                return;
            }
            Type::Hld => {
                let free = self.declare_libc_fn("tyrus_hld_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "hld.drop")
                    .unwrap();
                return;
            }
            Type::Centroid => {
                let free = self.declare_libc_fn("tyrus_centroid_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "centroid.drop")
                    .unwrap();
                return;
            }
            Type::BloomFilter => {
                let free = self.declare_libc_fn("tyrus_bloom_filter_free", &[pointer.into()], None);
                self.builder
                    .build_call(free, &[value.into()], "bloom_filter.drop")
                    .unwrap();
                return;
            }
            _ => return,
        };
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[allocation.into()], "")
            .unwrap();
    }

    fn emit_owned_native_cleanups_in_scope(&mut self, scope_index: usize, exclude: Option<&str>) {
        let mut names = self.owned_native_vars[scope_index]
            .iter()
            .filter(|(name, _)| exclude != Some(name.as_str()))
            .map(|(name, plan)| (name.clone(), plan.clone()))
            .collect::<Vec<_>>();
        names.sort_by(|left, right| left.0.cmp(&right.0));
        for (name, plan) in names {
            let Some((slot, ty)) = self.vars[scope_index].get(&name).cloned() else {
                continue;
            };
            if plan != ReleasePlan::Deep {
                let value = self
                    .builder
                    .build_load(self.llvm_type(&ty), slot, "native.cleanup")
                    .unwrap();
                self.emit_owned_native_release(value, &ty, &plan);
                continue;
            }
            if self.async_owned_value_supported(&ty)
                && let Some(function) = self
                    .builder
                    .get_insert_block()
                    .and_then(inkwell::basic_block::BasicBlock::get_parent)
            {
                self.emit_async_drop_value_in_place(function, slot, &ty)
                    .expect("internal: supported native owned value has no drop glue");
                continue;
            }
            let value = self
                .builder
                .build_load(self.llvm_type(&ty), slot, "native.cleanup")
                .unwrap();
            self.emit_owned_native_value_free(value, &ty);
        }
    }

    fn emit_owned_native_cleanups_for_names(&mut self, scope_index: usize, names: &[String]) {
        for name in names {
            let Some(plan) = self.owned_native_vars[scope_index].get(name).cloned() else {
                continue;
            };
            let Some((slot, ty)) = self.vars[scope_index].get(name).cloned() else {
                continue;
            };
            let value = self
                .builder
                .build_load(self.llvm_type(&ty), slot, "repl.cleanup")
                .unwrap();
            self.emit_owned_native_release(value, &ty, &plan);
        }
    }

    fn emit_conditionally_owned_cleanups_in_scope(
        &mut self,
        scope_index: usize,
        exclude: Option<&str>,
    ) {
        let mut names = self.conditionally_owned_vars[scope_index]
            .iter()
            .filter(|(name, _)| exclude != Some(name.as_str()))
            .map(|(name, bit)| (name.clone(), *bit))
            .collect::<Vec<_>>();
        names.sort_by(|left, right| left.0.cmp(&right.0));
        for (name, bit) in names {
            let Some((slot, ty)) = self.vars[scope_index].get(&name).cloned() else {
                continue;
            };
            let Some(function) = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
            else {
                continue;
            };
            let value = self
                .builder
                .build_load(self.llvm_type(&ty), slot, "conditional.owned.cleanup")
                .unwrap();
            self.emit_conditionally_owned_release(value, &ty, bit, function);
        }
    }

    fn emit_mutable_string_ownership_cleanups_in_scope(
        &mut self,
        scope_index: usize,
        exclude: Option<&str>,
    ) {
        let mut names = self.mutable_string_ownership[scope_index]
            .iter()
            .filter(|(name, _)| exclude != Some(name.as_str()))
            .map(|(name, bit_slot)| (name.clone(), *bit_slot))
            .collect::<Vec<_>>();
        names.sort_by(|left, right| left.0.cmp(&right.0));
        for (name, bit_slot) in names {
            let Some((slot, _ty)) = self.vars[scope_index].get(&name).cloned() else {
                continue;
            };
            let Some(function) = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
            else {
                continue;
            };
            self.emit_mutable_string_release_if_owned(function, &name, slot, bit_slot);
        }
    }

    fn emit_mutable_string_release_if_owned(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        slot: PointerValue<'ctx>,
        bit_slot: PointerValue<'ctx>,
    ) {
        let owned = self
            .builder
            .build_load(
                self.context.bool_type(),
                bit_slot,
                &format!("mut.str.{name}.owned"),
            )
            .unwrap()
            .into_int_value();
        let release = self
            .context
            .append_basic_block(function, "release.mutable.string");
        let next = self
            .context
            .append_basic_block(function, "release.mutable.string.next");
        self.builder
            .build_conditional_branch(owned, release, next)
            .unwrap();
        self.builder.position_at_end(release);
        let value = self
            .builder
            .build_load(
                self.string_abi_type(),
                slot,
                &format!("mut.str.{name}.value"),
            )
            .unwrap()
            .into_struct_value();
        self.emit_string_release(value)
            .expect("releasing a rebound string is always emittable");
        self.builder.build_unconditional_branch(next).unwrap();
        self.builder.position_at_end(next);
    }

    pub(super) fn mutable_string_ownership_slot(&self, name: &str) -> Option<PointerValue<'ctx>> {
        self.mutable_string_ownership
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    pub(super) fn register_mutable_string_ownership(
        &mut self,
        name: &str,
        bit_slot: PointerValue<'ctx>,
    ) {
        if let Some(scope) = self.mutable_string_ownership.last_mut() {
            scope.insert(name.to_string(), bit_slot);
        }
    }

    pub(super) fn replace_mutable_string_ownership(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        slot: PointerValue<'ctx>,
        bit_slot: PointerValue<'ctx>,
        value: StructValue<'ctx>,
        new_owned: bool,
    ) {
        self.emit_mutable_string_release_if_owned(function, name, slot, bit_slot);
        self.builder.build_store(slot, value).unwrap();
        self.builder
            .build_store(
                bit_slot,
                self.context.bool_type().const_int(new_owned as u64, false),
            )
            .unwrap();
    }

    pub(super) fn replace_mutable_string_ownership_with_bit(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        slot: PointerValue<'ctx>,
        bit_slot: PointerValue<'ctx>,
        value: StructValue<'ctx>,
        new_owned: IntValue<'ctx>,
    ) {
        self.emit_mutable_string_release_if_owned(function, name, slot, bit_slot);
        self.builder.build_store(slot, value).unwrap();
        self.builder.build_store(bit_slot, new_owned).unwrap();
    }

    fn emit_string_array_ownership_cleanups_in_scope(
        &mut self,
        scope_index: usize,
        exclude: Option<&str>,
    ) {
        let mut names = self.string_array_ownership[scope_index]
            .iter()
            .filter(|(name, _)| exclude != Some(name.as_str()))
            .map(|(name, mode_slot)| (name.clone(), *mode_slot))
            .collect::<Vec<_>>();
        names.sort_by(|left, right| left.0.cmp(&right.0));
        for (name, mode_slot) in names {
            let Some((slot, ty)) = self
                .vars
                .get(scope_index)
                .and_then(|scope| scope.get(&name))
                .cloned()
            else {
                continue;
            };
            let Some(function) = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
            else {
                continue;
            };
            self.emit_string_array_release_by_mode(function, &name, slot, mode_slot, &ty);
        }
    }

    pub(super) fn emit_string_array_release_by_mode(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        slot: PointerValue<'ctx>,
        mode_slot: PointerValue<'ctx>,
        ty: &Type,
    ) {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let mode = self
            .builder
            .build_load(
                self.context.i8_type(),
                mode_slot,
                &format!("str.array.{name}.mode"),
            )
            .unwrap()
            .into_int_value();
        let deep = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                mode,
                self.context.i8_type().const_int(2, false),
                &format!("str.array.{name}.deep"),
            )
            .unwrap();
        let release_deep = self
            .context
            .append_basic_block(function, "release.string.array.deep");
        let check_buffer = self
            .context
            .append_basic_block(function, "check.string.array.buffer");
        let release_buffer = self
            .context
            .append_basic_block(function, "release.string.array.buffer");
        let next = self
            .context
            .append_basic_block(function, "release.string.array.next");
        self.builder
            .build_conditional_branch(deep, release_deep, check_buffer)
            .unwrap();
        self.builder.position_at_end(check_buffer);
        let shallow = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                mode,
                self.context.i8_type().const_int(1, false),
                &format!("str.array.{name}.shallow"),
            )
            .unwrap();
        self.builder
            .build_conditional_branch(shallow, release_buffer, next)
            .unwrap();

        self.builder.position_at_end(release_deep);
        let drop_contents =
            self.declare_libc_fn("tyrus_array_drop_string_contents", &[pointer.into()], None);
        self.builder
            .build_call(drop_contents, &[slot.into()], "")
            .unwrap();
        self.builder.build_unconditional_branch(next).unwrap();

        self.builder.position_at_end(release_buffer);
        let array_type = self.llvm_type(ty);
        let array = self
            .builder
            .build_load(array_type, slot, &format!("str.array.{name}.value"))
            .unwrap()
            .into_struct_value();
        let data = self
            .builder
            .build_extract_value(array, 0, &format!("str.array.{name}.data"))
            .unwrap()
            .into_pointer_value();
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder.build_call(free, &[data.into()], "").unwrap();
        self.builder.build_unconditional_branch(next).unwrap();
        self.builder.position_at_end(next);
    }

    pub(super) fn normalize_string_array_for_mutation(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        slot: PointerValue<'ctx>,
        mode_slot: PointerValue<'ctx>,
        ty: &Type,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let mode = self
            .builder
            .build_load(
                self.context.i8_type(),
                mode_slot,
                &format!("str.array.{name}.mutation.mode"),
            )
            .unwrap()
            .into_int_value();
        let shallow = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                mode,
                self.context.i8_type().const_int(1, false),
                &format!("str.array.{name}.mutation.shallow"),
            )
            .unwrap();
        let normalize = self
            .context
            .append_basic_block(function, "normalize.string.array.for.mutation");
        let ready = self
            .context
            .append_basic_block(function, "mutate.string.array");
        self.builder
            .build_conditional_branch(shallow, normalize, ready)
            .unwrap();

        self.builder.position_at_end(normalize);
        let array_type = self.llvm_type(ty);
        let cloned = self
            .builder
            .build_alloca(array_type, &format!("{name}.array.mutation.clone"))
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
                &format!("{name}.array.mutation.clone.status"),
            )
            .unwrap()
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        self.emit_abort_if_negative(status, "string-array mutation", "string.array")?;
        let previous = self
            .builder
            .build_load(array_type, slot, &format!("{name}.array.mutation.previous"))
            .unwrap()
            .into_struct_value();
        let previous_data = self
            .builder
            .build_extract_value(previous, 0, &format!("{name}.array.mutation.previous.data"))
            .unwrap()
            .into_pointer_value();
        let free = self.declare_libc_fn("free", &[pointer.into()], None);
        self.builder
            .build_call(free, &[previous_data.into()], "")
            .unwrap();
        let cloned_value = self
            .builder
            .build_load(array_type, cloned, &format!("{name}.array.mutation.cloned"))
            .unwrap();
        self.builder.build_store(slot, cloned_value).unwrap();
        self.builder
            .build_store(mode_slot, self.context.i8_type().const_int(2, false))
            .unwrap();
        self.builder.build_unconditional_branch(ready).unwrap();
        self.builder.position_at_end(ready);
        Ok(())
    }

    pub(super) fn string_array_ownership_slot(&self, name: &str) -> Option<PointerValue<'ctx>> {
        self.string_array_ownership
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    pub(super) fn register_string_array_ownership(
        &mut self,
        name: &str,
        mode_slot: PointerValue<'ctx>,
    ) {
        if let Some(scope) = self.string_array_ownership.last_mut() {
            scope.insert(name.to_string(), mode_slot);
        }
    }

    pub(super) fn take_string_array_binding_mode(&mut self, value: BasicValueEnum<'ctx>) -> u64 {
        let Some(index) = self
            .pending_owned_native_results
            .iter()
            .position(|(pending, _, _)| pending.as_value_ref() == value.as_value_ref())
        else {
            return 0;
        };
        match &self.pending_owned_native_results[index].2 {
            ReleasePlan::Buffer => {
                self.pending_owned_native_results.remove(index);
                1
            }
            ReleasePlan::Deep => {
                self.pending_owned_native_results.remove(index);
                2
            }
            _ => 0,
        }
    }

    pub(super) fn replace_string_array_ownership(
        &mut self,
        function: FunctionValue<'ctx>,
        name: &str,
        slot: PointerValue<'ctx>,
        mode_slot: PointerValue<'ctx>,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) {
        let new_mode = self.take_string_array_binding_mode(value);
        self.emit_string_array_release_by_mode(function, name, slot, mode_slot, ty);
        self.builder.build_store(slot, value).unwrap();
        self.builder
            .build_store(mode_slot, self.context.i8_type().const_int(new_mode, false))
            .unwrap();
    }

    pub(super) fn record_owned_native_result(&mut self, value: BasicValueEnum<'ctx>, ty: Type) {
        self.record_owned_native_result_with(value, ty, ReleasePlan::Deep);
    }

    pub(super) fn record_owned_native_result_with(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: Type,
        plan: ReleasePlan,
    ) {
        if (!self.async_results.is_empty() && self.async_temporaries_tracked == 0)
            || self.optional_chain_depth > 0
        {
            return;
        }
        self.pending_owned_native_results.push((value, ty, plan));
    }

    pub(super) fn take_owned_native_result(
        &mut self,
        value: BasicValueEnum<'ctx>,
    ) -> Option<ReleasePlan> {
        let index = self
            .pending_owned_native_results
            .iter()
            .position(|(pending, _, _)| pending.as_value_ref() == value.as_value_ref())?;
        let (_, _, plan) = self.pending_owned_native_results.remove(index);
        Some(plan)
    }

    pub(super) fn take_owned_native_result_with_type(
        &mut self,
        value: BasicValueEnum<'ctx>,
    ) -> Option<(Type, ReleasePlan)> {
        let index = self
            .pending_owned_native_results
            .iter()
            .position(|(pending, _, _)| pending.as_value_ref() == value.as_value_ref())?;
        let (_, ty, plan) = self.pending_owned_native_results.remove(index);
        Some((ty, plan))
    }

    pub(super) fn forget_owned_native_result(&mut self, value: BasicValueEnum<'ctx>) -> bool {
        if let Some(index) = self
            .pending_owned_native_results
            .iter()
            .position(|(pending, _, _)| pending.as_value_ref() == value.as_value_ref())
        {
            self.pending_owned_native_results.remove(index);
            true
        } else {
            false
        }
    }

    pub(super) fn forget_conditionally_owned_result(
        &mut self,
        value: BasicValueEnum<'ctx>,
    ) -> bool {
        if let Some(index) = self
            .pending_conditionally_owned_results
            .iter()
            .position(|(pending, _, _)| pending.as_value_ref() == value.as_value_ref())
        {
            self.pending_conditionally_owned_results.remove(index);
            true
        } else {
            false
        }
    }

    pub(super) fn transfer_owned_native_result(&mut self, name: &str, value: BasicValueEnum<'ctx>) {
        let Some(index) = self
            .pending_owned_native_results
            .iter()
            .position(|(pending, _, _)| pending.as_value_ref() == value.as_value_ref())
        else {
            return;
        };
        // The entry leaves the pending list either way: this value now belongs
        // to a binding, and the statement-end sweep must not free it. Only
        // where it goes differs. Synchronously it becomes an `owned_native_vars`
        // entry, so scope exit releases it, the plan travels with the binding,
        // since scope cleanup looks the value up by name and reads its declared
        // type, which cannot say whether the parts were allocated. Inside an
        // async function the heap frame's own drop glue already owns the slot,
        // so the registration is simply dropped.
        //
        // Leaving it pending in the async case was a use-after-free waiting for
        // `async_temporaries_tracked` to start recording anything at all: the
        // frame freed the array at scope exit and the sweep freed it again at
        // the end of the very statement that declared it.
        let (_, _, plan) = self.pending_owned_native_results.remove(index);
        if self.async_results.is_empty()
            && let Some(scope) = self.owned_native_vars.last_mut()
        {
            scope.insert(name.to_string(), plan);
        }
    }

    pub(super) fn record_conditionally_owned_result(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: Type,
        bit: IntValue<'ctx>,
    ) {
        if !self.async_results.is_empty() || self.optional_chain_depth > 0 {
            return;
        }
        self.pending_conditionally_owned_results
            .push((value, ty, bit));
    }

    pub(super) fn transfer_conditionally_owned_result(
        &mut self,
        name: &str,
        value: BasicValueEnum<'ctx>,
    ) {
        let Some(index) = self
            .pending_conditionally_owned_results
            .iter()
            .position(|(pending, _, _)| pending.as_value_ref() == value.as_value_ref())
        else {
            return;
        };
        let (_, _, bit) = self.pending_conditionally_owned_results.remove(index);
        if self.async_results.is_empty()
            && let Some(scope) = self.conditionally_owned_vars.last_mut()
        {
            scope.insert(name.to_string(), bit);
        }
    }

    fn emit_conditionally_owned_release(
        &mut self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
        bit: IntValue<'ctx>,
        function: FunctionValue<'ctx>,
    ) {
        let release = self
            .context
            .append_basic_block(function, "conditional.owned.release");
        let done = self
            .context
            .append_basic_block(function, "conditional.owned.done");
        self.builder
            .build_conditional_branch(bit, release, done)
            .unwrap();
        self.builder.position_at_end(release);
        self.emit_owned_native_value_free(value, ty);
        self.builder.build_unconditional_branch(done).unwrap();
        self.builder.position_at_end(done);
    }

    pub(super) fn has_nested_owned_native_results(
        &self,
        value: BasicValueEnum<'ctx>,
        ty: &Type,
    ) -> bool {
        self.async_results.is_empty()
            && self.async_owned_value_supported(ty)
            && self
                .pending_owned_native_results
                .iter()
                .any(|(pending, _, _)| pending.as_value_ref() != value.as_value_ref())
    }

    pub(super) fn mark_owned_synchronous_value(&mut self, name: &str) {
        self.mark_owned_synchronous_value_with(name, ReleasePlan::Deep);
    }

    pub(super) fn mark_owned_synchronous_value_with(&mut self, name: &str, plan: ReleasePlan) {
        if self.async_results.is_empty()
            && let Some(scope) = self.owned_native_vars.last_mut()
        {
            scope.insert(name.to_string(), plan);
        }
    }

    pub(super) fn promote_owned_synchronous_binding(&mut self, name: &str) {
        if !self.async_results.is_empty() {
            return;
        }
        let Some(scope_index) = (0..self.vars.len())
            .rev()
            .find(|index| self.vars[*index].contains_key(name))
        else {
            return;
        };
        self.owned_native_vars[scope_index].insert(name.to_string(), ReleasePlan::Deep);
    }

    pub(super) fn transfer_owned_binding(&mut self, source: &str, name: &str) {
        if !self.async_results.is_empty() {
            return;
        }
        let Some(scope_index) = (0..self.vars.len())
            .rev()
            .find(|index| self.vars[*index].contains_key(source))
        else {
            return;
        };
        if let Some(plan) = self.owned_native_vars[scope_index].remove(source)
            && let Some(scope) = self.owned_native_vars.last_mut()
        {
            scope.insert(name.to_string(), plan);
        }
        if let Some(mode_slot) = self.string_array_ownership[scope_index].remove(source)
            && let Some(scope) = self.string_array_ownership.last_mut()
        {
            scope.insert(name.to_string(), mode_slot);
        }
        if let Some(bit_slot) = self.mutable_string_ownership[scope_index].remove(source)
            && let Some(scope) = self.mutable_string_ownership.last_mut()
        {
            scope.insert(name.to_string(), bit_slot);
        }
        if let Some(bit) = self.conditionally_owned_vars[scope_index].remove(source)
            && let Some(scope) = self.conditionally_owned_vars.last_mut()
        {
            scope.insert(name.to_string(), bit);
        }
    }

    pub(super) fn dispose_owned_native_binding(&mut self, name: &str) {
        let Some(scope_index) = (0..self.vars.len())
            .rev()
            .find(|index| self.vars[*index].contains_key(name))
        else {
            return;
        };
        self.owned_native_vars[scope_index].remove(name);
        self.string_array_ownership[scope_index].remove(name);
        self.mutable_string_ownership[scope_index].remove(name);
        self.conditionally_owned_vars[scope_index].remove(name);
        let Some((slot, ty)) = self.vars[scope_index].get(name).cloned() else {
            return;
        };
        if matches!(transparent_codegen_type(&ty), Type::Named(value) if value == "DigestStream") {
            let null = self
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .const_null();
            self.builder.build_store(slot, null).unwrap();
        }
    }

    pub(super) fn prepare_owned_native_assignment(
        &mut self,
        name: &str,
        slot: PointerValue<'ctx>,
        ty: &Type,
        value: BasicValueEnum<'ctx>,
    ) {
        if !self.async_results.is_empty() {
            return;
        }
        if self.mutable_string_ownership_slot(name).is_some() {
            return;
        }
        let owns_new_value = self
            .pending_owned_native_results
            .iter()
            .position(|(pending, _, _)| pending.as_value_ref() == value.as_value_ref());
        let Some(pending_index) = owns_new_value else {
            return;
        };
        let scope_index = (0..self.vars.len())
            .rev()
            .find(|index| self.vars[*index].contains_key(name));
        let Some(scope_index) = scope_index else {
            return;
        };
        if let Some(previous_plan) = self.owned_native_vars[scope_index].get(name).cloned() {
            let previous = self
                .builder
                .build_load(self.llvm_type(ty), slot, "native.replace.previous")
                .unwrap();
            self.emit_owned_native_release(previous, ty, &previous_plan);
        }
        let (_, _, plan) = self.pending_owned_native_results.remove(pending_index);
        self.owned_native_vars[scope_index].insert(name.to_string(), plan);
    }

    pub(super) fn owned_native_plan_for(&self, name: &str) -> Option<ReleasePlan> {
        let scope_index = (0..self.vars.len())
            .rev()
            .find(|index| self.vars[*index].contains_key(name))?;
        self.owned_native_vars[scope_index].get(name).cloned()
    }

    pub(super) fn prepare_conditionally_owned_assignment(
        &mut self,
        name: &str,
        slot: PointerValue<'ctx>,
        ty: &Type,
        value: BasicValueEnum<'ctx>,
    ) {
        if !self.async_results.is_empty() {
            return;
        }
        let owns_new_value = self
            .pending_conditionally_owned_results
            .iter()
            .position(|(pending, _, _)| pending.as_value_ref() == value.as_value_ref());
        let Some(pending_index) = owns_new_value else {
            return;
        };
        let scope_index = (0..self.vars.len())
            .rev()
            .find(|index| self.vars[*index].contains_key(name));
        let Some(scope_index) = scope_index else {
            return;
        };
        if let Some(previous_bit) = self.conditionally_owned_vars[scope_index]
            .get(name)
            .copied()
            && let Some(function) = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
        {
            let previous = self
                .builder
                .build_load(self.llvm_type(ty), slot, "conditional.replace.previous")
                .unwrap();
            self.emit_conditionally_owned_release(previous, ty, previous_bit, function);
        }
        let (_, _, bit) = self
            .pending_conditionally_owned_results
            .remove(pending_index);
        self.conditionally_owned_vars[scope_index].insert(name.to_string(), bit);
    }

    pub(super) fn discard_owned_native_result(&mut self) {
        let guard = self
            .pending_owned_native_guard
            .last()
            .map_or(0, |(_, guard)| *guard);
        let guard = guard.min(self.pending_owned_native_results.len());
        let pending = self.pending_owned_native_results.split_off(guard);
        for (value, ty, plan) in pending {
            self.emit_owned_native_release(value, &ty, &plan);
        }
        let conditional_guard = guard.min(self.pending_conditionally_owned_results.len());
        let conditional = self
            .pending_conditionally_owned_results
            .split_off(conditional_guard);
        for (value, ty, bit) in conditional {
            let Some(function) = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
            else {
                continue;
            };
            self.emit_conditionally_owned_release(value, &ty, bit, function);
        }
    }

    pub(super) fn emit_for_of_iterable_cleanups(&mut self) {
        for (value, ty, plan) in self.for_of_iterable_cleanups.clone().into_iter().rev() {
            self.emit_owned_native_release(value, &ty, &plan);
        }
    }

    pub(super) fn discard_all_owned_native_results(&mut self) {
        let pending = std::mem::take(&mut self.pending_owned_native_results);
        for (value, ty, plan) in pending {
            self.emit_owned_native_release(value, &ty, &plan);
        }
        let conditional = std::mem::take(&mut self.pending_conditionally_owned_results);
        for (value, ty, bit) in conditional {
            let Some(function) = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
            else {
                continue;
            };
            self.emit_conditionally_owned_release(value, &ty, bit, function);
        }
    }

    pub(super) fn emit_pending_owned_native_result_cleanup(&mut self) {
        for (value, ty, plan) in self.pending_owned_native_results.clone() {
            self.emit_owned_native_release(value, &ty, &plan);
        }
        let Some(function) = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
        else {
            return;
        };
        for (value, ty, bit) in self.pending_conditionally_owned_results.clone() {
            self.emit_conditionally_owned_release(value, &ty, bit, function);
        }
    }

    pub(super) fn transfer_owned_native_return(&mut self, value: BasicValueEnum<'ctx>) {
        if let Some(index) = self
            .pending_owned_native_results
            .iter()
            .position(|(pending, _, _)| pending.as_value_ref() == value.as_value_ref())
        {
            self.pending_owned_native_results.remove(index);
        }
        self.discard_all_owned_native_results();
    }

    pub(super) fn load_var(&self, name: &str) -> Option<(PointerValue<'ctx>, Type)> {
        for scope in self.vars.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v.clone());
            }
        }
        None
    }

    pub(super) fn numeric_to_string_allocates(
        &self,
        object: &Expr,
        method: &str,
        args: &[Expr],
    ) -> bool {
        if method != "toString" || !args.is_empty() {
            return false;
        }
        self.expr_type_hint(object).is_some_and(|ty| {
            let scalar = transparent_codegen_type(&ty);
            is_integer_type(scalar) || is_float_type(scalar)
        })
    }

    pub(super) fn owned_channel_receive_expression(&self, expression: &Expr) -> bool {
        let ExprKind::MethodCall { object, method, .. } = expression.kind() else {
            return false;
        };
        method == "tryRecv"
            && self
                .expr_type_hint(object)
                .map(|ty| self.resolve_type(&ty))
                .is_some_and(|ty| {
                    matches!(transparent_codegen_type(&ty), Type::Apply { name, args }
                        if name == "Receiver" && args.len() == 1)
                })
    }

    pub(super) fn owned_user_function_call(&self, expression: &Expr) -> bool {
        let ExprKind::FnCall {
            callee,
            type_args,
            args,
        } = expression.kind()
        else {
            return false;
        };
        if self.async_functions.contains(callee) {
            return false;
        }
        if let Some(ty) = self
            .fn_signatures
            .get(callee)
            .map(|(_, ty)| self.resolve_type(ty))
        {
            return self.async_owned_value_supported(&ty);
        }
        if let Some(ty) = self.generic_call_return_type(callee, type_args, args) {
            return self.async_owned_value_supported(&ty);
        }
        false
    }

    pub(super) fn generic_call_return_type(
        &self,
        name: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Option<Type> {
        let definition = self.generic_fns.get(name)?;
        if !type_args.is_empty() && type_args.len() != definition.type_params.len() {
            return None;
        }
        let mut substitutions = HashMap::default();
        for (parameter, concrete) in definition.type_params.iter().zip(type_args) {
            substitutions.insert(parameter.name.clone(), self.resolve_type(concrete));
        }
        for (parameter, argument) in definition.params.iter().zip(args) {
            if let Some(actual) = self.codegen_expr_type(argument) {
                infer_codegen_generics(&parameter.ty, &actual, &mut substitutions).ok()?;
            }
        }
        for parameter in &definition.type_params {
            if !substitutions.contains_key(&parameter.name) {
                return None;
            }
        }
        Some(substitute_type(&definition.ret_type, &substitutions))
    }

    pub(super) fn owned_array_method_call(&self, expression: &Expr) -> bool {
        matches!(
            expression.kind(),
            ExprKind::MethodCall { method, .. } if tyrus_core::async_types::array_method_allocates(method)
        )
    }

    pub(super) fn owned_algorithm_method_call(&self, expression: &Expr) -> bool {
        matches!(
            expression.kind(),
            ExprKind::MethodCall { object, method, .. }
                if crate::methods::algorithm_method_allocates(method)
                    && self.expr_type_hint(object).is_some_and(|ty| {
                        crate::helpers::is_algorithm_value_type(&ty)
                    })
        )
    }

    fn owned_string_array_literal_expression(&self, expression: &Expr) -> bool {
        matches!(
            expression.kind(),
            ExprKind::ArrayLit(values)
                if values.iter().all(|value| {
                    // A literal element is released as a no-op, since its
                    // string has no ownership bit, so it may sit beside owned ones.
                    matches!(value.kind(), ExprKind::StrLit(_))
                        || self.async_owned_string_expression(value)
                })
        )
    }

    pub(super) fn store_concrete_type(&mut self, name: &str, concrete: Type) {
        if let Some(scope) = self.var_concrete_types.last_mut() {
            scope.insert(name.to_string(), concrete);
        }
    }

    pub(super) fn load_concrete_type(&self, name: &str) -> Option<Type> {
        for scope in self.var_concrete_types.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    pub(super) fn clear_concrete_type(&mut self, name: &str) {
        for scope in self.var_concrete_types.iter_mut().rev() {
            if scope.remove(name).is_some() {
                return;
            }
        }
    }

    pub(super) fn store_non_zero_var(&mut self, name: &str) {
        if let Some(scope) = self.non_zero_vars.last_mut() {
            scope.insert(name.to_string());
        }
    }

    pub(super) fn load_non_zero_var(&self, name: &str) -> bool {
        self.non_zero_vars
            .iter()
            .rev()
            .any(|scope| scope.contains(name))
    }

    pub(super) fn lit_underlying_type(&self, expr: &Expr) -> Type {
        match expr.kind() {
            ExprKind::StrLit(_) => Type::Str,
            ExprKind::RegexLit { .. } => Type::Regex,
            ExprKind::IntLit(_) => Type::I32,
            ExprKind::UIntLit(_) => Type::U128,
            ExprKind::FloatLit(_) => Type::F64,
            ExprKind::BoolLit(_) => Type::Bool,
            ExprKind::Null | ExprKind::None => Type::Never,
            _ => Type::Never,
        }
    }

    fn type_query_underlying_type(&self, expr: &Expr) -> Type {
        match expr.kind() {
            ExprKind::Ident(name) => self.load_var(name).map_or(Type::Str, |(_, ty)| ty),
            _ => Type::Str,
        }
    }

    fn union_payload_type(&self, variants: &[Type]) -> BasicTypeEnum<'ctx> {
        if Self::union_is_payload_free(variants) {
            return self.context.i64_type().array_type(0).into();
        }
        let largest_size = variants
            .iter()
            .map(|variant| self.type_layout(variant).0)
            .max()
            .unwrap_or(8);
        let largest_align = variants
            .iter()
            .map(|variant| self.type_layout(variant).1)
            .max()
            .unwrap_or(8);
        if largest_align >= 16 {
            let words = largest_size.div_ceil(16).max(1) as u32;
            return self.context.i128_type().array_type(words).into();
        }
        let words = largest_size.div_ceil(8).max(1) as u32;
        self.context.i64_type().array_type(words).into()
    }

    pub(super) fn union_is_payload_free(variants: &[Type]) -> bool {
        !variants.is_empty()
            && variants
                .iter()
                .all(|variant| matches!(variant, Type::Lit(_)))
    }

    pub(super) fn union_variant_index(variants: &[Type], variant: &Type) -> u8 {
        variants.iter().position(|v| v == variant).unwrap_or(0) as u8
    }

    pub(super) fn store_union_variant(
        &self,
        variants: &[Type],
        union_ptr: PointerValue<'ctx>,
        variant_index: u8,
        value: BasicValueEnum<'ctx>,
    ) -> Result<(), String> {
        let union_ty = self.union_llvm_type(variants);
        self.store_union_variant_tag(union_ty, union_ptr, variant_index)?;

        if Self::union_is_payload_free(variants) {
            return Ok(());
        }

        let payload_ptr = self
            .builder
            .build_struct_gep(union_ty, union_ptr, 1, "union.payload")
            .map_err(|e| format!("union payload gep: {e}"))?;

        let value_llvm_ty = value.get_type();
        let value_ptr_ty = basic_type_to_ptr_type(value_llvm_ty);
        let cast_payload = self
            .builder
            .build_pointer_cast(payload_ptr, value_ptr_ty, "union.payload.cast")
            .map_err(|e| format!("union payload cast: {e}"))?;
        let store = self
            .builder
            .build_store(cast_payload, value)
            .map_err(|e| format!("union payload store: {e}"))?;
        Self::align_to_union_payload(store)?;

        Ok(())
    }

    pub(crate) fn store_union_variant_tag(
        &self,
        union_ty: StructType<'ctx>,
        union_ptr: PointerValue<'ctx>,
        variant_index: u8,
    ) -> Result<(), String> {
        let tag_ptr = self
            .builder
            .build_struct_gep(union_ty, union_ptr, 0, "union.tag")
            .map_err(|error| format!("union tag gep: {error}"))?;
        self.builder
            .build_store(
                tag_ptr,
                self.context
                    .i8_type()
                    .const_int(variant_index as u64, false),
            )
            .map_err(|error| format!("union tag store: {error}"))?;
        Ok(())
    }

    pub(super) fn build_union_value(
        &mut self,
        variants: &[Type],
        expr: &Expr,
        name: &str,
    ) -> Result<(BasicValueEnum<'ctx>, Type), String> {
        if let ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } = expr.kind()
        {
            let function = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent)
                .ok_or("internal: union ternary outside function")?;
            let condition = self.emit_condition_value(cond)?;
            let then_block = self
                .context
                .append_basic_block(function, "union.ternary.then");
            let else_block = self
                .context
                .append_basic_block(function, "union.ternary.else");
            let merge = self
                .context
                .append_basic_block(function, "union.ternary.merge");
            self.builder
                .build_conditional_branch(condition, then_block, else_block)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(then_block);
            let (then_value, _) = self.build_union_value(variants, then_expr, name)?;
            let then_from = self
                .builder
                .get_insert_block()
                .ok_or("internal: union ternary then-branch left no block")?;
            self.builder
                .build_unconditional_branch(merge)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(else_block);
            let (else_value, _) = self.build_union_value(variants, else_expr, name)?;
            let else_from = self
                .builder
                .get_insert_block()
                .ok_or("internal: union ternary else-branch left no block")?;
            self.builder
                .build_unconditional_branch(merge)
                .map_err(|error| error.to_string())?;

            self.builder.position_at_end(merge);
            let phi = self
                .builder
                .build_phi(self.union_llvm_type(variants), name)
                .map_err(|error| error.to_string())?;
            phi.add_incoming(&[
                (&then_value as &dyn inkwell::values::BasicValue, then_from),
                (&else_value as &dyn inkwell::values::BasicValue, else_from),
            ]);
            return Ok((phi.as_basic_value(), Type::Union(variants.to_vec())));
        }
        if self
            .expr_type_hint(expr)
            .map(|hint| self.resolve_type(&hint))
            .is_some_and(|hint| {
                matches!(transparent_codegen_type(&hint), Type::Union(existing) if existing == variants)
            })
        {
            let value = self.emit_expr(expr)?;
            return Ok((value, Type::Union(variants.to_vec())));
        }
        let union_ty = self.union_llvm_type(variants);
        let alloca = self.builder.build_alloca(union_ty, name).unwrap();
        let concrete = self.union_variant_for_expr(expr, variants);
        let tag = Self::union_variant_index(variants, &concrete);
        let value = self.emit_expr_as(expr, &concrete)?;
        self.store_union_variant(variants, alloca, tag, value)?;
        let union_value = self
            .builder
            .build_load(union_ty, alloca, name)
            .map_err(|error| error.to_string())?;
        Ok((union_value, concrete))
    }

    pub(super) fn load_union_payload(
        &self,
        variants: &[Type],
        union_ptr: PointerValue<'ctx>,
        variant_ty: &Type,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if Self::union_is_payload_free(variants) {
            return self.const_literal_type_value(variant_ty, name);
        }

        let union_ty = self.union_llvm_type(variants);
        let variant_llvm_ty = self.llvm_type(variant_ty);

        let payload_ptr = self
            .builder
            .build_struct_gep(union_ty, union_ptr, 1, "union.payload")
            .map_err(|e| format!("union payload gep: {e}"))?;

        let value_ptr_ty = basic_type_to_ptr_type(variant_llvm_ty);
        let cast_payload = self
            .builder
            .build_pointer_cast(payload_ptr, value_ptr_ty, "union.payload.cast")
            .map_err(|e| format!("union payload cast: {e}"))?;

        let value = self
            .builder
            .build_load(variant_llvm_ty, cast_payload, name)
            .map_err(|e| format!("union payload load: {e}"))?;
        if let Some(load) = inkwell::values::BasicValue::as_instruction_value(&value) {
            Self::align_to_union_payload(load)?;
        }
        Ok(value)
    }

    fn const_literal_type_value(
        &self,
        variant_ty: &Type,
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let Type::Lit(literal) = variant_ty else {
            return Err(format!(
                "internal: `{variant_ty}` is not a literal type, so it has no constant value"
            ));
        };
        match literal.kind() {
            ExprKind::StrLit(text) => Ok(self.const_string_abi(text, "str")),
            ExprKind::BoolLit(value) => Ok(self
                .context
                .bool_type()
                .const_int(u64::from(*value), false)
                .into()),
            ExprKind::IntLit(value) => Ok(self
                .llvm_type(variant_ty)
                .into_int_type()
                .const_int(*value as u64, true)
                .into()),
            ExprKind::UIntLit(value) => Ok(self
                .llvm_type(variant_ty)
                .into_int_type()
                .const_int(*value as u64, false)
                .into()),
            ExprKind::FloatLit(value) => Ok(self
                .llvm_type(variant_ty)
                .into_float_type()
                .const_float(value.value())
                .into()),
            other => Err(format!(
                "internal: literal type `{other:?}` has no constant value for `{name}`"
            )),
        }
    }

    fn union_payload_slot(
        &self,
        variants: &[Type],
        union_ptr: PointerValue<'ctx>,
        variant_ty: &Type,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        if Self::union_is_payload_free(variants) {
            return Err("internal: a union of literal types has no payload to address".to_string());
        }
        let union_ty = self.union_llvm_type(variants);
        let payload_ptr = self
            .builder
            .build_struct_gep(union_ty, union_ptr, 1, "union.payload")
            .map_err(|error| format!("union payload gep: {error}"))?;
        self.builder
            .build_pointer_cast(
                payload_ptr,
                basic_type_to_ptr_type(self.llvm_type(variant_ty)),
                name,
            )
            .map_err(|error| format!("union payload cast: {error}"))
    }

    pub(super) fn load_union_tag(
        &self,
        variants: &[Type],
        union_ptr: PointerValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let union_ty = self.union_llvm_type(variants);
        let tag_ptr = self
            .builder
            .build_struct_gep(union_ty, union_ptr, 0, "union.tag")
            .map_err(|e| format!("union tag gep: {e}"))?;
        self.builder
            .build_load(self.context.i8_type(), tag_ptr, "union.tag.value")
            .map(|v| v.into_int_value())
            .map_err(|e| format!("union tag load: {e}"))
    }

    pub(super) fn emit_union_common_field(
        &mut self,
        variants: &[Type],
        union_ptr: PointerValue<'ctx>,
        field: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if Self::union_is_payload_free(variants) {
            return Err("error: field access requires an object value".to_string());
        }
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: union field access outside a function")?;
        let tag = self.load_union_tag(variants, union_ptr)?;
        let merge = self
            .context
            .append_basic_block(function, "union.field.merge");
        let unknown = self
            .context
            .append_basic_block(function, "union.field.unknown");
        let mut cases = Vec::with_capacity(variants.len());
        let mut blocks = Vec::with_capacity(variants.len());
        for index in 0..variants.len() {
            let block = self
                .context
                .append_basic_block(function, &format!("union.field.variant.{index}"));
            cases.push((self.context.i8_type().const_int(index as u64, false), block));
            blocks.push(block);
        }
        self.builder
            .build_switch(tag, unknown, &cases)
            .map_err(|error| format!("union field switch: {error}"))?;
        self.builder.position_at_end(unknown);
        self.builder
            .build_unreachable()
            .map_err(|error| format!("union field unreachable: {error}"))?;

        let mut incoming = Vec::with_capacity(variants.len());
        for (variant, block) in variants.iter().zip(blocks) {
            self.builder.position_at_end(block);
            let resolved = self.resolve_type(variant);
            let index = match transparent_codegen_type(&resolved) {
                Type::Named(name) => self
                    .interfaces
                    .get(name)
                    .and_then(|fields| fields.iter().position(|candidate| candidate.name == field))
                    .map(|index| index as u32)
                    .ok_or_else(|| format!("error: interface `{name}` has no field `{field}`"))?,
                Type::Object(_) => self
                    .struct_field_index(&resolved, field)
                    .ok_or_else(|| format!("error: object has no field `{field}`"))?,
                _ => return Err("error: field access requires an object value".to_string()),
            };
            let payload = self
                .load_union_payload(variants, union_ptr, variant, "union.field.payload")?
                .into_struct_value();
            let value = self
                .builder
                .build_extract_value(payload, index, field)
                .map_err(|error| error.to_string())?;
            let end = self
                .builder
                .get_insert_block()
                .ok_or("internal: union field access lost its block")?;
            self.builder
                .build_unconditional_branch(merge)
                .map_err(|error| error.to_string())?;
            incoming.push((value, end));
        }

        self.builder.position_at_end(merge);
        let first = incoming
            .first()
            .map(|(value, _)| value.get_type())
            .ok_or("internal: union field access on a union with no variants")?;
        let phi = self
            .builder
            .build_phi(first, field)
            .map_err(|error| error.to_string())?;
        for (value, block) in &incoming {
            phi.add_incoming(&[(value, *block)]);
        }
        Ok(phi.as_basic_value())
    }

    pub(super) fn payload_free_union_underlying(&self, ty: &Type) -> Option<(Vec<Type>, Type)> {
        let resolved = self.resolve_type(ty);
        let Type::Union(variants) = transparent_codegen_type(&resolved) else {
            return None;
        };
        if !Self::union_is_payload_free(variants) {
            return None;
        }
        let mut common: Option<Type> = None;
        for variant in variants {
            let Type::Lit(literal) = variant else {
                return None;
            };
            let underlying = match literal.kind() {
                ExprKind::StrLit(_) => Type::Str,
                ExprKind::BoolLit(_) => Type::Bool,
                _ => return None,
            };
            match &common {
                None => common = Some(underlying),
                Some(existing) if *existing == underlying => {}
                Some(_) => return None,
            }
        }
        common.map(|underlying| (variants.clone(), underlying))
    }

    pub(super) fn emit_union_underlying_value(
        &mut self,
        union_value: BasicValueEnum<'ctx>,
        variants: &[Type],
        name: &str,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let (last, rest) = variants
            .split_last()
            .ok_or("internal: a union has at least one variant")?;
        let tag = self
            .builder
            .build_extract_value(union_value.into_struct_value(), 0, "union.underlying.tag")
            .map_err(|error| error.to_string())?
            .into_int_value();
        let mut value = self.const_literal_type_value(last, name)?;
        for (index, variant) in rest.iter().enumerate().rev() {
            let is_variant = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    self.context.i8_type().const_int(index as u64, false),
                    &format!("union.underlying.is.{index}"),
                )
                .map_err(|error| error.to_string())?;
            let candidate = self.const_literal_type_value(variant, name)?;
            value = self
                .builder
                .build_select(is_variant, candidate, value, name)
                .map_err(|error| error.to_string())?;
        }
        Ok(value)
    }

    /// Register a user-declared interface, choosing where its fields are stored.
    ///
    /// The source order is kept in `declared_interfaces` for the paths that
    /// present fields to a human; `interfaces` receives the storage order, which
    /// is what the LLVM struct type is built from. Every site that turns a field
    /// name into a position reads `interfaces`, so it follows the permutation
    /// without knowing one happened.
    ///
    /// An interface the runtime also knows is left exactly as declared: its
    /// field order is an ABI contract with a `#[repr(C)]` struct on the other
    /// side of the call, and permuting it would not fail, it would read the
    /// neighbouring field and carry on.
    pub(super) fn register_interface(&mut self, name: String, fields: Vec<Field>) {
        if self.native_layout_interfaces.contains(&name) {
            self.interfaces.insert(name, fields);
            return;
        }
        let stored = self.storage_order(&fields);
        self.declared_interfaces.insert(name.clone(), fields);
        self.interfaces.insert(name, stored);
    }

    pub(super) fn declared_fields(&self, ty: &Type) -> Option<Vec<Field>> {
        let resolved = self.resolve_type(ty);
        match transparent_codegen_type(&resolved) {
            Type::Named(name) => self
                .declared_interfaces
                .get(name.as_str())
                .or_else(|| self.interfaces.get(name.as_str()))
                .cloned(),
            Type::Object(fields) => Some(fields.clone()),
            _ => None,
        }
    }

    pub(super) fn declared_fields_with_index(&self, ty: &Type) -> Option<Vec<(u32, Field)>> {
        let declared = self.declared_fields(ty)?;
        declared
            .into_iter()
            .map(|field| {
                self.struct_field_index(ty, &field.name)
                    .map(|index| (index, field))
            })
            .collect()
    }

    fn storage_order(&self, fields: &[Field]) -> Vec<Field> {
        let mut stored = fields.to_vec();
        stored.sort_by_key(|field| {
            let mut seen = HashSet::default();
            let (align, size) = self.layout_weight(&field_storage_type(field), &mut seen);
            std::cmp::Reverse((align, size))
        });
        stored
    }

    fn aggregate_weight(&self, fields: &[Field], seen: &mut HashSet<String>) -> (u32, u32) {
        let mut align = 1;
        let mut size = 0;
        for field in fields {
            let (field_align, field_size) = self.layout_weight(&field_storage_type(field), seen);
            align = align.max(field_align);
            size += field_size;
        }
        (align, size)
    }

    pub(super) fn struct_field_index(&self, ty: &Type, field: &str) -> Option<u32> {
        let resolved = self.resolve_type(ty);
        match transparent_codegen_type(&resolved) {
            Type::Named(name) => self
                .interfaces
                .get(name.as_str())?
                .iter()
                .position(|candidate| candidate.name == field)
                .map(|index| index as u32),
            Type::Object(fields) => fields
                .iter()
                .position(|candidate| candidate.name == field)
                .map(|index| index as u32),
            _ => None,
        }
    }

    pub(super) fn struct_field_type(&self, ty: &Type, field: &str) -> Option<Type> {
        let resolved = self.resolve_type(ty);
        let fields = match transparent_codegen_type(&resolved) {
            Type::Named(name) => self.interfaces.get(name.as_str())?,
            Type::Object(fields) => fields,
            _ => return None,
        };
        fields
            .iter()
            .find(|candidate| candidate.name == field)
            .map(|candidate| self.resolve_type(&field_storage_type(candidate)))
    }

    pub(super) fn type_string_for_variant(variant: &Type) -> String {
        tyrus_core::typeof_name::typeof_name(transparent_codegen_type(variant))
    }

    pub(super) fn emit_union_retag(
        &mut self,
        value: BasicValueEnum<'ctx>,
        source_variants: &[Type],
        target_variants: &[Type],
    ) -> Result<BasicValueEnum<'ctx>, String> {
        if source_variants == target_variants {
            return Ok(value);
        }
        let targets = source_variants
            .iter()
            .map(|variant| {
                Self::union_retag_target(variant, target_variants).ok_or_else(|| {
                    format!(
                        "internal: union variant `{variant}` has no counterpart in `{}`",
                        Type::Union(target_variants.to_vec())
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let function = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or("internal: union retag outside a function")?;
        let source_ty = self.union_llvm_type(source_variants);
        let target_ty = self.union_llvm_type(target_variants);
        let source_slot = self.entry_alloca(source_ty, "union.retag.source")?;
        let target_slot = self.entry_alloca(target_ty, "union.retag.target")?;
        self.builder
            .build_store(source_slot, value)
            .map_err(|error| format!("union retag store: {error}"))?;
        self.builder
            .build_store(target_slot, target_ty.const_zero())
            .map_err(|error| format!("union retag store: {error}"))?;
        let tag = self.load_union_tag(source_variants, source_slot)?;
        let merge = self
            .context
            .append_basic_block(function, "union.retag.merge");
        let blocks = (0..source_variants.len())
            .map(|index| {
                self.context
                    .append_basic_block(function, &format!("union.retag.{index}"))
            })
            .collect::<Vec<_>>();
        let cases = blocks
            .iter()
            .enumerate()
            .map(|(index, block)| {
                (
                    self.context.i8_type().const_int(index as u64, false),
                    *block,
                )
            })
            .collect::<Vec<_>>();
        self.builder
            .build_switch(tag, merge, &cases)
            .map_err(|error| format!("union retag switch: {error}"))?;
        for ((variant, target_index), block) in source_variants.iter().zip(targets).zip(blocks) {
            self.builder.position_at_end(block);
            let payload = self.load_union_payload(
                source_variants,
                source_slot,
                variant,
                "union.retag.payload",
            )?;
            self.store_union_variant(target_variants, target_slot, target_index, payload)?;
            self.builder
                .build_unconditional_branch(merge)
                .map_err(|error| format!("union retag branch: {error}"))?;
        }
        self.builder.position_at_end(merge);
        self.builder
            .build_load(target_ty, target_slot, "union.retag.value")
            .map_err(|error| format!("union retag load: {error}"))
    }

    fn union_retag_target(variant: &Type, targets: &[Type]) -> Option<u8> {
        let source = transparent_codegen_type(variant);
        let position = |wanted: &Type| {
            targets
                .iter()
                .position(|target| transparent_codegen_type(target) == wanted)
        };
        position(source)
            .or_else(|| match source {
                Type::Lit(_) => tyrus_core::typeof_category::typeof_category_type(
                    tyrus_core::typeof_category::typeof_category(source),
                )
                .and_then(|underlying| position(&underlying)),
                _ => None,
            })
            .and_then(|index| u8::try_from(index).ok())
    }

    pub(super) fn emit_union_typeof(
        &self,
        union_ptr: PointerValue<'ctx>,
        variants: &[Type],
        fn_val: FunctionValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let tag = self.load_union_tag(variants, union_ptr)?;
        let string_ty = self.string_abi_type();
        let i8_ty = self.context.i8_type();

        let result_alloca = self
            .builder
            .build_alloca(string_ty, "typeof.result")
            .unwrap();
        let unknown_str = self
            .const_string_abi("unknown", "typeof.default")
            .into_struct_value();
        self.builder
            .build_store(result_alloca, unknown_str)
            .unwrap();

        let merge_bb = self.context.append_basic_block(fn_val, "typeof.merge");
        let mut cmp_prev = self.context.append_basic_block(fn_val, "typeof.cmp.0");
        self.builder.build_unconditional_branch(cmp_prev).unwrap();

        for i in 0..variants.len() {
            let store_bb = self
                .context
                .append_basic_block(fn_val, &format!("typeof.set.{i}"));

            let cmp_next = if i < variants.len() - 1 {
                self.context
                    .append_basic_block(fn_val, &format!("typeof.cmp.{}", i + 1))
            } else {
                merge_bb
            };

            self.builder.position_at_end(cmp_prev);
            let cmp_val = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    tag,
                    i8_ty.const_int(i as u64, false),
                    &format!("typeof.eq.{i}"),
                )
                .unwrap();
            self.builder
                .build_conditional_branch(cmp_val, store_bb, cmp_next)
                .unwrap();

            self.builder.position_at_end(store_bb);
            let name = Self::type_string_for_variant(&variants[i]);
            let str_val = self.const_string_named(&name, &format!("typeof.str.{i}"));
            self.builder.build_store(result_alloca, str_val).unwrap();
            self.builder.build_unconditional_branch(merge_bb).unwrap();

            cmp_prev = cmp_next;
        }

        self.builder.position_at_end(merge_bb);
        self.builder
            .build_load(string_ty, result_alloca, "typeof.result.load")
            .map_err(|e| format!("typeof result load: {e}"))
    }

    pub(super) fn usize_type(&self) -> IntType<'ctx> {
        match self.target {
            CompileTarget::Wasm => self.context.i32_type(),
            CompileTarget::Native => self.context.i64_type(),
        }
    }

    pub(super) fn truncate_to_usize(&self, val: IntValue<'ctx>) -> IntValue<'ctx> {
        let target = self.usize_type();
        if val.get_type() == target {
            val
        } else {
            self.builder
                .build_int_truncate(val, target, "usize.trunc")
                .unwrap()
        }
    }

    pub(super) fn extend_to_i64(&self, val: IntValue<'ctx>) -> IntValue<'ctx> {
        let target = self.context.i64_type();
        if val.get_type() == target {
            val
        } else {
            self.builder
                .build_int_z_extend(val, target, "usize.zext")
                .unwrap()
        }
    }

    pub(super) fn declare_malloc(
        &self,
        size: IntValue<'ctx>,
    ) -> (FunctionValue<'ctx>, IntValue<'ctx>) {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let f = self.declare_libc_fn("malloc", &[self.usize_type().into()], Some(pointer.into()));
        (f, self.truncate_to_usize(size))
    }

    pub(super) fn declare_calloc(
        &self,
        nmemb: IntValue<'ctx>,
        size: IntValue<'ctx>,
    ) -> (FunctionValue<'ctx>, IntValue<'ctx>, IntValue<'ctx>) {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let usize_type = self.usize_type();
        let f = self.declare_libc_fn(
            "calloc",
            &[usize_type.into(), usize_type.into()],
            Some(pointer.into()),
        );
        (
            f,
            self.truncate_to_usize(nmemb),
            self.truncate_to_usize(size),
        )
    }

    pub(crate) fn entry_alloca<T: inkwell::types::BasicType<'ctx>>(
        &self,
        ty: T,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let entry = self
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .and_then(inkwell::values::FunctionValue::get_first_basic_block)
            .ok_or("internal: a stack slot needs an enclosing function")?;
        let builder = self.context.create_builder();
        match entry.get_first_instruction() {
            Some(first) => builder.position_before(&first),
            None => builder.position_at_end(entry),
        }
        builder
            .build_alloca(ty, name)
            .map_err(|error| error.to_string())
    }

    pub(super) fn heap_alloc(
        &self,
        ty: BasicTypeEnum<'ctx>,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        let size = ty
            .size_of()
            .ok_or_else(|| format!("error: `{name}` has no native allocation size"))?;
        let (malloc, size) = self.declare_malloc(size);
        Ok(self
            .builder
            .build_call(malloc, &[size.into()], name)
            .map_err(|error| error.to_string())?
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("internal: `{name}` allocation returned no value"))?
            .into_pointer_value())
    }

    pub(super) fn emit_zero(&self, ty: &Type) -> Result<BasicValueEnum<'ctx>, String> {
        Ok(match ty {
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
            | Type::Usize
            | Type::Bool
            | Type::Char => {
                let int_ty = self.llvm_type(ty).into_int_type();
                int_ty.const_int(0, false).into()
            }
            Type::F32 => self.context.f32_type().const_float(0.0).into(),
            Type::F64 => self.context.f64_type().const_float(0.0).into(),
            Type::F128 => self.context.f128_type().const_float(0.0).into(),
            Type::Decimal => self.llvm_type(ty).const_zero(),
            Type::Str => self.null_string().into(),
            Type::Ref(_) | Type::MutRef(_) => self
                .context
                .ptr_type(inkwell::AddressSpace::default())
                .const_null()
                .into(),
            Type::Union(variants) => {
                let struct_ty = self.union_llvm_type(variants);
                struct_ty.const_zero().into()
            }
            _ => {
                let bt: BasicTypeEnum<'ctx> = self.llvm_type(ty);
                bt.const_zero()
            }
        })
    }

    pub(super) fn resolve_type(&self, ty: &Type) -> Type {
        if let Type::Throws { function, .. } = ty {
            return self.resolve_type(function);
        }
        let substituted = match self.type_substitutions.last() {
            Some(substitutions) => substitute_type(ty, substitutions),
            None => ty.clone(),
        };
        match &substituted {
            Type::Named(name) if name == "ObjectId" => Type::Str,
            Type::Named(name) if self.enums.contains_key(name) => {
                let enum_info = &self.enums[name];
                if enum_info.is_algebraic {
                    substituted
                } else {
                    enum_info.repr.clone()
                }
            }
            Type::Named(name) => self.type_aliases.get(name).cloned().unwrap_or(substituted),
            Type::Apply { name, .. } if name == "HttpReply" || name == "__HttpReplyStatus" => {
                substituted
            }
            Type::Apply { name, args } if name == "Page" && args.len() == 1 => {
                let fields = tyrus_core::builtin_interfaces::db_page_fields()
                    .into_iter()
                    .map(|mut field| {
                        if field.name == "items" {
                            field.ty = Type::Array(Box::new(args[0].clone()));
                        }
                        field
                    })
                    .collect();
                Type::Object(fields)
            }
            _ => substituted,
        }
    }

    pub(super) fn llvm_fn_type(&self, params: &[Type], ret_type: &Type) -> FunctionType<'ctx> {
        let param_types: Vec<_> = params.iter().map(|ty| self.llvm_type(ty).into()).collect();

        match ret_type {
            Type::Void | Type::Never => self.context.void_type().fn_type(&param_types, false),
            ty => self.llvm_type(ty).fn_type(&param_types, false),
        }
    }

    pub(super) fn llvm_closure_fn_type(
        &self,
        params: &[Type],
        ret_type: &Type,
    ) -> FunctionType<'ctx> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let mut param_types: Vec<BasicMetadataTypeEnum<'ctx>> = vec![pointer.into()];
        param_types.extend(
            params
                .iter()
                .map(|ty| BasicMetadataTypeEnum::from(self.llvm_type(ty))),
        );
        match ret_type {
            Type::Void | Type::Never => self.context.void_type().fn_type(&param_types, false),
            ty => self.llvm_type(ty).fn_type(&param_types, false),
        }
    }

    pub(super) fn declare_printf(&self) -> FunctionValue<'ctx> {
        if let Some(f) = self.module.get_function("printf") {
            return f;
        }
        let i8_ptr = self.context.ptr_type(inkwell::AddressSpace::default());
        let printf_ty = self.context.i32_type().fn_type(&[i8_ptr.into()], true);
        self.module.add_function("printf", printf_ty, None)
    }

    pub(super) fn declare_dprintf(&self) -> FunctionValue<'ctx> {
        let name = if self.target == CompileTarget::Native {
            "tyrus_dprintf"
        } else {
            "dprintf"
        };
        if let Some(function) = self.module.get_function(name) {
            return function;
        }
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let function_type = self
            .context
            .i32_type()
            .fn_type(&[self.context.i32_type().into(), pointer.into()], true);
        self.module.add_function(name, function_type, None)
    }

    pub(super) fn emit_native_constant(&self, target: &str) -> Option<BasicValueEnum<'ctx>> {
        let value = match target {
            "__tyrus_os_EOL" => {
                if cfg!(windows) {
                    "\r\n"
                } else {
                    "\n"
                }
            }
            _ => return None,
        };
        Some(self.const_string_named(value, "native_const").into())
    }

    pub(super) fn declare_libc_fn(
        &self,
        name: &str,
        params: &[BasicTypeEnum<'ctx>],
        return_type: Option<BasicTypeEnum<'ctx>>,
    ) -> FunctionValue<'ctx> {
        let params: Vec<_> = params.iter().map(|ty| (*ty).into()).collect();
        let function_type = match return_type {
            Some(ty) => ty.fn_type(&params, false),
            None => self.context.void_type().fn_type(&params, false),
        };

        // Rust's wasm32 C ABI passes `TyrusString` indirectly, while LLVM
        // lowers the generated `{ptr, i64}` value as two scalar arguments.
        // Keep that target-specific convention at this one boundary: callers
        // use the ordinary generated-code signature and a private thunk spills
        // direct string arguments (and receives string results through sret)
        // in the runtime's `{ptr, u32, u32}` memory layout.
        let string_type = self.string_type();
        let has_string_parameter = params.iter().any(|parameter| {
            matches!(parameter, BasicMetadataTypeEnum::StructType(kind) if *kind == string_type)
        });
        let returns_string = return_type.is_some_and(
            |kind| matches!(kind, BasicTypeEnum::StructType(value) if value == string_type),
        );
        if self.target == CompileTarget::Wasm && (has_string_parameter || returns_string) {
            return self.declare_wasm_string_abi_thunk(
                name,
                function_type,
                params.as_slice(),
                return_type,
                string_type,
            );
        }

        // A native `string` argument travels in two integer registers. When
        // one starts with a single register left, the runtime's C ABI puts the
        // whole value on the stack while this call splits it, and every
        // argument after it is read from the wrong place. `app.docs()` died on
        // exactly that; pass such options behind a `#[repr(C)]` pointer.
        if cfg!(debug_assertions) && self.target == CompileTarget::Native {
            let mut integer_registers = 0usize;
            for parameter in &params {
                match parameter {
                    BasicMetadataTypeEnum::FloatType(_) => {}
                    BasicMetadataTypeEnum::StructType(kind) if *kind == string_type => {
                        debug_assert!(
                            integer_registers != 5,
                            "`{name}` passes a `string` by value starting at the last integer argument register; pass it behind a `#[repr(C)]` pointer"
                        );
                        integer_registers += 2;
                    }
                    _ => integer_registers += 1,
                }
            }
        }

        if let Some(function) = self.module.get_function(name) {
            debug_assert_eq!(
                function.get_type(),
                function_type,
                "`{name}` is declared with two different signatures; the first one wins and this \
                 call would be emitted against it"
            );
            return function;
        }

        let function = self.module.add_function(name, function_type, None);
        self.describe_allocator(name, function);
        self.describe_abort(name, function);
        self.describe_pure_runtime(name, function);
        function
    }

    /// Tells LLVM what the runtime's scalar helpers do to memory, so it can
    /// treat them as arithmetic rather than as opaque calls.
    ///
    /// The measurement that motivates this: a loop calling `sqrt(seed)` twice
    /// and `log(seed)` once on a *loop-invariant* argument kept all three calls
    /// inside the loop in a release build, three million times over. Neither
    /// common-subexpression elimination nor loop-invariant code motion could
    /// touch them, because a `declare` with no attributes might read or write
    /// anything, might not return, and might unwind, so LLVM has to assume it
    /// does all three.
    ///
    /// Cross-language LTO does not rescue this. It gives LLVM the Rust bodies,
    /// but these are `#[no_mangle] extern "C"` symbols the linker must keep,
    /// and the calls survived to the final assembly anyway. Saying it on the
    /// declaration is what makes the difference, the same way `describe_abort`
    /// says `noreturn` once for the function instead of leaving LLVM to infer
    /// it per call site.
    ///
    /// # What is deliberately *not* here
    ///
    /// `willreturn` is a promise, and a function that aborts the process breaks
    /// it. So the split below is not cosmetic: everything that can reach
    /// `tyrus_panic` on overflow, `factorial`, `permutation`, `combination`,
    /// `fibonacci`, and everything whose own integer arithmetic can trap under
    /// a debug runtime's overflow checks, `lcm` (`a / gcd * b`), `is_prime`
    /// and `next_prime` (`i * i`, `n + 1`), gets `nounwind` and nothing more.
    /// `nounwind` stays true for them because an `extern "C"` function aborts
    /// rather than unwinding.
    ///
    /// `tyrus_math_random_range` is excluded outright: it reads and advances
    /// generator state, so it is neither pure nor repeatable, and describing it
    /// as pure would let LLVM collapse two draws into one.
    fn describe_pure_runtime(&self, name: &str, function: FunctionValue<'ctx>) {
        use inkwell::attributes::AttributeLoc;
        let enum_attribute = |key: &str, value: u64| {
            let id = inkwell::attributes::Attribute::get_named_enum_kind_id(key);
            debug_assert_ne!(id, 0, "`{key}` is not an attribute this LLVM knows");
            self.context.create_enum_attribute(id, value)
        };
        if is_pure_total_runtime_fn(name) {
            function.add_attribute(AttributeLoc::Function, enum_attribute("memory", 0));
            for key in ["nounwind", "willreturn"] {
                function.add_attribute(AttributeLoc::Function, enum_attribute(key, 0));
            }
        } else if is_reading_total_runtime_fn(name) {
            const READS_EVERY_LOCATION: u64 = 0b01_01_01_01_01_01;
            function.add_attribute(
                AttributeLoc::Function,
                enum_attribute("memory", READS_EVERY_LOCATION),
            );
            for key in ["nounwind", "willreturn"] {
                function.add_attribute(AttributeLoc::Function, enum_attribute(key, 0));
            }
        } else if is_pure_aborting_runtime_fn(name) {
            function.add_attribute(AttributeLoc::Function, enum_attribute("nounwind", 0));
        }
    }

    fn describe_abort(&self, name: &str, function: FunctionValue<'ctx>) {
        if !matches!(
            name,
            "tyrus_panic" | "tyrus_panic_user" | "tyrus_panic_validation"
        ) {
            return;
        }
        use inkwell::attributes::AttributeLoc;
        for key in ["noreturn", "cold", "nounwind"] {
            let id = inkwell::attributes::Attribute::get_named_enum_kind_id(key);
            function.add_attribute(
                AttributeLoc::Function,
                self.context.create_enum_attribute(id, 0),
            );
        }
    }

    fn declare_wasm_string_abi_thunk(
        &self,
        name: &str,
        generated_type: FunctionType<'ctx>,
        generated_params: &[BasicMetadataTypeEnum<'ctx>],
        generated_return: Option<BasicTypeEnum<'ctx>>,
        string_type: StructType<'ctx>,
    ) -> FunctionValue<'ctx> {
        let thunk_name = format!("__tyrus_wasm_string_abi_{name}");
        if let Some(thunk) = self.module.get_function(&thunk_name) {
            debug_assert_eq!(
                thunk.get_type(),
                generated_type,
                "`{name}` is declared with two different generated-code signatures"
            );
            return thunk;
        }

        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let i64_type = self.context.i64_type();
        let runtime_string_type = self
            .context
            .struct_type(&[pointer.into(), i32_type.into(), i32_type.into()], false);
        let is_string = |kind: BasicMetadataTypeEnum<'ctx>| matches!(kind, BasicMetadataTypeEnum::StructType(value) if value == string_type);

        let mut runtime_params = Vec::with_capacity(
            generated_params.len()
                + usize::from(generated_return.is_some_and(
                    |kind| matches!(kind, BasicTypeEnum::StructType(value) if value == string_type),
                )),
        );
        let returns_string = generated_return.is_some_and(
            |kind| matches!(kind, BasicTypeEnum::StructType(value) if value == string_type),
        );
        if returns_string {
            runtime_params.push(pointer.into());
        }
        runtime_params.extend(generated_params.iter().map(|kind| {
            if is_string(*kind) {
                BasicMetadataTypeEnum::PointerType(pointer)
            } else {
                *kind
            }
        }));
        let runtime_type = if returns_string {
            self.context.void_type().fn_type(&runtime_params, false)
        } else {
            match generated_return {
                Some(kind) => kind.fn_type(&runtime_params, false),
                None => self.context.void_type().fn_type(&runtime_params, false),
            }
        };
        let runtime = match self.module.get_function(name) {
            Some(function) => {
                debug_assert_eq!(
                    function.get_type(),
                    runtime_type,
                    "`{name}` is declared with two different runtime ABI signatures"
                );
                function
            }
            None => self.module.add_function(name, runtime_type, None),
        };

        let thunk = self.module.add_function(
            &thunk_name,
            generated_type,
            Some(inkwell::module::Linkage::Private),
        );
        let entry = self.context.append_basic_block(thunk, "entry");
        let builder = self.context.create_builder();
        builder.position_at_end(entry);

        let result_slot = returns_string.then(|| {
            builder
                .build_alloca(runtime_string_type, "string.result")
                .expect("the ABI thunk entry block accepts an alloca")
        });
        let mut arguments: Vec<BasicMetadataValueEnum<'ctx>> =
            Vec::with_capacity(runtime_params.len());
        if let Some(slot) = result_slot {
            arguments.push(slot.into());
        }
        for (index, kind) in generated_params.iter().enumerate() {
            let value = thunk
                .get_nth_param(index as u32)
                .expect("the ABI thunk has every declared parameter");
            if is_string(*kind) {
                let value = value.into_struct_value();
                let slot = builder
                    .build_alloca(runtime_string_type, "string.argument")
                    .expect("the ABI thunk entry block accepts an alloca");
                let data = builder
                    .build_extract_value(value, 0, "string.data")
                    .expect("a generated string has a data field");
                let packed = builder
                    .build_extract_value(value, 1, "string.lengths")
                    .expect("a generated string has packed lengths")
                    .into_int_value();
                let bytes = builder
                    .build_int_truncate(packed, i32_type, "string.bytes")
                    .expect("the byte length occupies the low word");
                let chars = builder
                    .build_right_shift(
                        packed,
                        i64_type.const_int(32, false),
                        false,
                        "string.chars.wide",
                    )
                    .and_then(|wide| builder.build_int_truncate(wide, i32_type, "string.chars"))
                    .expect("the character length occupies the high word");
                for (field, field_value) in
                    [data, bytes.into(), chars.into()].into_iter().enumerate()
                {
                    let address = builder
                        .build_struct_gep(runtime_string_type, slot, field as u32, "string.field")
                        .expect("the runtime string has three fields");
                    builder
                        .build_store(address, field_value)
                        .expect("the ABI thunk can initialize a string field");
                }
                arguments.push(slot.into());
            } else {
                arguments.push(value.into());
            }
        }

        let call = builder
            .build_call(runtime, &arguments, "runtime.call")
            .expect("the marshalled runtime call has the declared signature");
        if let Some(slot) = result_slot {
            let data = builder
                .build_load(
                    pointer,
                    builder
                        .build_struct_gep(runtime_string_type, slot, 0, "result.data.addr")
                        .expect("data field exists"),
                    "result.data",
                )
                .expect("the runtime initialized the string result")
                .into_pointer_value();
            let bytes = builder
                .build_load(
                    i32_type,
                    builder
                        .build_struct_gep(runtime_string_type, slot, 1, "result.bytes.addr")
                        .expect("byte field exists"),
                    "result.bytes",
                )
                .expect("the runtime initialized the byte length")
                .into_int_value();
            let chars = builder
                .build_load(
                    i32_type,
                    builder
                        .build_struct_gep(runtime_string_type, slot, 2, "result.chars.addr")
                        .expect("character field exists"),
                    "result.chars",
                )
                .expect("the runtime initialized the character length")
                .into_int_value();
            let bytes = builder
                .build_int_z_extend(bytes, i64_type, "result.bytes.wide")
                .expect("the packed word is i64");
            let chars = builder
                .build_int_z_extend(chars, i64_type, "result.chars.wide")
                .and_then(|wide| {
                    builder.build_left_shift(
                        wide,
                        i64_type.const_int(32, false),
                        "result.chars.packed",
                    )
                })
                .expect("the character length occupies the high word");
            let packed = builder
                .build_or(bytes, chars, "result.lengths")
                .expect("the two lengths form one packed word");
            let value = builder
                .build_insert_value(string_type.get_undef(), data, 0, "result.string")
                .and_then(|value| builder.build_insert_value(value, packed, 1, "result.string"))
                .expect("the generated string has two fields");
            builder
                .build_return(Some(&value))
                .expect("the ABI thunk returns the generated string");
        } else if let Some(value) = call.try_as_basic_value().basic() {
            builder
                .build_return(Some(&value))
                .expect("the ABI thunk forwards the runtime result");
        } else {
            builder
                .build_return(None)
                .expect("the ABI thunk forwards a void runtime call");
        }
        thunk
    }

    fn describe_allocator(&self, name: &str, function: FunctionValue<'ctx>) {
        use inkwell::attributes::AttributeLoc;
        let string_attribute =
            |key: &str, value: &str| self.context.create_string_attribute(key, value);
        let enum_attribute = |key: &str| {
            let id = inkwell::attributes::Attribute::get_named_enum_kind_id(key);
            self.context.create_enum_attribute(id, 0)
        };
        match name {
            "tyrus_string_alloc" => {
                function.add_attribute(
                    AttributeLoc::Function,
                    string_attribute("alloc-family", "tyrus_string"),
                );
                function.add_attribute(
                    AttributeLoc::Function,
                    string_attribute("allockind", "alloc,uninitialized"),
                );
                function.add_attribute(
                    AttributeLoc::Function,
                    self.context.create_enum_attribute(
                        inkwell::attributes::Attribute::get_named_enum_kind_id("allocsize"),
                        0xffff_ffff,
                    ),
                );
                function.add_attribute(AttributeLoc::Return, enum_attribute("noalias"));
                function.add_attribute(AttributeLoc::Function, enum_attribute("nounwind"));
                function.add_attribute(AttributeLoc::Function, enum_attribute("willreturn"));
            }
            "tyrus_string_release_buffer" => {
                function.add_attribute(
                    AttributeLoc::Function,
                    string_attribute("alloc-family", "tyrus_string"),
                );
                function.add_attribute(
                    AttributeLoc::Function,
                    string_attribute("allockind", "free"),
                );
                function.add_attribute(AttributeLoc::Function, enum_attribute("nounwind"));
                function.add_attribute(AttributeLoc::Function, enum_attribute("willreturn"));
            }
            _ => {}
        }
    }

    pub(crate) fn codegen_and_verify(self, program: &Program) -> Result<Module<'ctx>, String> {
        self.codegen_and_verify_with_repl_cleanup(program, None, &[])
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn codegen_and_verify_with_repl_cleanup(
        mut self,
        program: &Program,
        cleanup_name: Option<&str>,
        cleanup_globals: &[String],
    ) -> Result<Module<'ctx>, String> {
        if self.target == CompileTarget::Wasm {
            let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
            let size_param = self.usize_type().into();
            self.declare_libc_fn("malloc", &[size_param], Some(pointer.into()));
            self.declare_libc_fn(
                "realloc",
                &[pointer.into(), size_param],
                Some(pointer.into()),
            );
            self.declare_libc_fn("free", &[pointer.into()], None);
        }
        let mut program = program.clone();
        self.auto_route_exports = program.auto_route_exports.clone();
        tyrus_core::type_resolve::normalize_program_for_backend(&mut program);
        let native_behavior_items = program
            .imports
            .iter()
            .flat_map(|import| {
                import.specifiers.iter().filter_map(move |specifier| {
                    let behavior = tyrus_core::module_loader::native_export_behavior(
                        &import.source,
                        &specifier.imported,
                    )?;
                    Some(Item::BehaviorInterfaceDecl {
                        name: specifier.local.clone(),
                        type_params: Vec::new(),
                        targets: behavior.targets,
                        extends: behavior.extends,
                        methods: behavior.methods,
                        bindings: behavior.bindings,
                        span: tyrus_core::span::Span::DUMMY,
                    })
                })
            })
            .collect::<Vec<_>>();
        program.items.extend(native_behavior_items);
        tyrus_core::validators::resolve_membership_decorator_constants(
            &mut program,
            &std::collections::HashMap::default(),
        );
        tyrus_core::array_literal_fusion::fuse(&mut program);
        self.user_validator_names
            .extend(program.items.iter().filter_map(|item| match item {
                Item::FnDecl {
                    name,
                    is_validator: true,
                    ..
                } => Some(name.clone()),
                _ => None,
            }));
        let program = &program;
        self.contained_arrays = tyrus_core::escape_analysis::analyze(program);
        self.constant_global_arrays =
            tyrus_core::escape_analysis::analyze_constant_global_arrays(program);
        self.checked_indexes = tyrus_core::bounds_check::analyze(program);
        self.confined_closures = tyrus_core::escape_analysis::analyze_confined_closures(program);
        for item in &program.items {
            if let Item::BehaviorInterfaceDecl { name, targets, .. } = item {
                self.behavior_targets.insert(name.clone(), targets.clone());
            }
        }
        for _ in 0..program.items.len() {
            let mut changed = false;
            for item in &program.items {
                let Item::BehaviorInterfaceDecl { name, extends, .. } = item else {
                    continue;
                };
                let inherited = extends
                    .iter()
                    .filter_map(|supertype| {
                        let Type::Named(super_name) = supertype else {
                            return None;
                        };
                        self.behavior_targets.get(super_name).cloned()
                    })
                    .flatten()
                    .collect::<Vec<_>>();
                let entry = self.behavior_targets.entry(name.clone()).or_default();
                for target in inherited {
                    if !entry.contains(&target) {
                        entry.push(target);
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        for item in &program.items {
            let Item::BehaviorInterfaceDecl {
                name,
                targets,
                methods,
                ..
            } = item
            else {
                continue;
            };
            let effective_targets = self
                .behavior_targets
                .get(name)
                .cloned()
                .unwrap_or_else(|| targets.clone());
            for (target_index, target) in effective_targets.iter().enumerate() {
                for method in methods {
                    if !method.type_params.is_empty() {
                        return Err(format!(
                            "error: generic extension method `{}` is not supported yet",
                            method.name
                        ));
                    }
                    if matches!(method.ret_type, Type::Infer) {
                        return Err(format!(
                            "error: extension method `{}` must declare a return type",
                            method.name
                        ));
                    }
                    let symbol = format!("__tyrus_at_{}_{}_{}", name, method.name, target_index);
                    if let Some(existing) = self
                        .extension_methods
                        .iter()
                        .find(|existing| existing.target == *target && existing.name == method.name)
                    {
                        return Err(format!(
                            "error[E0034]: multiple extension methods named `{}` apply to `{target}` through `{}` and `{name}`",
                            method.name, existing.owner
                        ));
                    }
                    self.extension_methods.push(ExtensionMethod {
                        owner: name.clone(),
                        target: target.clone(),
                        name: method.name.clone(),
                        symbol: symbol.clone(),
                    });
                }
                if let Item::BehaviorInterfaceDecl { bindings, .. } = item {
                    for binding in bindings {
                        let method_name = tyrus_core::module_loader::original_global_name(binding);
                        if let Some(existing) = self.extension_methods.iter().find(|existing| {
                            existing.target == *target && existing.name == method_name
                        }) {
                            return Err(format!(
                                "error[E0034]: multiple extension methods named `{method_name}` apply to `{target}` through `{}` and `{name}`",
                                existing.owner
                            ));
                        }
                        self.extension_methods.push(ExtensionMethod {
                            owner: name.clone(),
                            target: target.clone(),
                            name: method_name.to_string(),
                            symbol: binding.clone(),
                        });
                    }
                }
            }
        }
        for item in &program.items {
            if let Item::FnDecl { name, throws, .. } = item
                && !throws.is_empty()
            {
                self.fn_throws.insert(name.clone(), throws.clone());
            }
        }
        for item in &program.items {
            if let Item::FnDecl {
                name, params, body, ..
            } = item
            {
                match returns_owned_container_analysis(
                    params,
                    body,
                    &self.fn_owned_returns,
                    &self.fn_conditionally_owned_returns,
                ) {
                    Some(bits)
                        if !bits.is_empty()
                            && bits.iter().all(|b| *b == ReturnOwnership::Owned) =>
                    {
                        self.fn_owned_returns.insert(name.clone());
                    }
                    Some(bits)
                        if bits.iter().any(|b| {
                            matches!(b, ReturnOwnership::Owned | ReturnOwnership::Forwarded)
                        }) =>
                    {
                        self.fn_conditionally_owned_returns.insert(name.clone());
                    }
                    _ => {}
                }
            }
        }
        self.fn_db_async_returns = db_async_returning_functions(&program.items);
        validate_async_codegen_support(program)?;
        let mut generic_defs = tyrus_core::type_resolve::GenericDefs::collect(program);
        if self.interfaces.contains_key("Page") {
            generic_defs.register_db_page();
        }
        for item in &program.items {
            if let Item::InterfaceDecl {
                name,
                fields,
                type_params,
                db_model,
                ..
            } = item
            {
                if let Some(model) = db_model {
                    self.db_models.insert(name.clone(), model.clone());
                }
                let merged = if type_params.is_empty() {
                    let mut ignored = Vec::new();
                    generic_defs
                        .interface_fields(name, &mut ignored)
                        .unwrap_or_else(|| fields.clone())
                } else {
                    fields.clone()
                };
                if type_params.is_empty() && generic_defs.interface_extends(name, "Error") {
                    self.error_subtypes.insert(name.clone());
                }
                self.register_interface(name.clone(), merged);
            }
            if let Item::EnumDecl {
                name,
                repr,
                members,
                ..
            } = item
            {
                let (resolved, _) = resolve_enum(repr, members);
                self.enums.insert(name.clone(), resolved);
            }
            if let Item::TypeAlias {
                name,
                ty,
                merged_fields,
                type_params,
                ..
            } = item
            {
                let ty = if type_params.is_empty() {
                    let mut ignored = Vec::new();
                    generic_defs.resolve(ty, &mut ignored)
                } else {
                    ty.clone()
                };
                self.type_aliases.insert(name.clone(), ty.clone());
                let ty = &ty;
                if !merged_fields.is_empty() {
                    self.register_interface(name.clone(), merged_fields.clone());
                } else if let Type::Union(variants) = ty {
                    let fields = merged_union_fields(variants);
                    if !fields.is_empty() {
                        self.register_interface(name.clone(), fields);
                    }
                }
            }
        }
        let mut thrown_spellings = self
            .interfaces
            .keys()
            .chain(self.enums.keys())
            .cloned()
            .collect::<Vec<_>>();
        thrown_spellings.sort();
        if let Some((first, second)) = crate::statements::first_thrown_tag_collision(
            thrown_spellings.iter().map(String::as_str),
            crate::statements::thrown_type_tag_of,
        ) {
            return Err(format!(
                "error: `{first}` and `{second}` have the same thrown type tag, so a `catch` could not tell them apart; rename one of them"
            ));
        }
        for item in &program.items {
            let Item::BehaviorInterfaceDecl {
                name,
                targets: _,
                methods,
                ..
            } = item
            else {
                continue;
            };
            let effective_targets = self.behavior_targets.get(name).cloned().unwrap_or_default();
            for (target_index, target) in effective_targets.iter().enumerate() {
                for method in methods {
                    let symbol = format!("__tyrus_at_{}_{}_{}", name, method.name, target_index);
                    let params = specialize_extension_receiver(&method.params, target);
                    self.forward_declare_fn(&symbol, &params, &method.ret_type);
                }
            }
        }
        for import in &program.imports {
            if import.source == "tyrus:crypto" {
                for spec in &import.specifiers {
                    if let Some((ty, _, _)) = tyrus_core::module_loader::native_export_info(
                        &import.source,
                        &spec.imported,
                    ) {
                        let llvm_ty = self.llvm_type(&ty);
                        let global = self.module.add_global(
                            llvm_ty,
                            None,
                            &format!("__tyrus_ns_{}", spec.local),
                        );
                        global.set_initializer(&llvm_ty.const_zero());
                        self.store_var(&spec.local, global.as_pointer_value(), ty);
                    }
                }
            }
        }
        for name in tyrus_core::module_loader::math_constant_names() {
            if let Some(value) = tyrus_core::module_loader::math_constant_value(name) {
                let target = format!("__tyrus_math_{name}");
                let ty = Type::F64;
                let llvm_ty = self.llvm_type(&ty);
                let float_ty = llvm_ty.into_float_type();
                let global = self.module.add_global(float_ty, None, &target);
                global.set_initializer(&float_ty.const_float(value));
                self.store_var(&target, global.as_pointer_value(), ty);
            }
        }
        for item in &program.items {
            if let Item::FnDecl {
                name,
                is_async: true,
                ..
            } = item
            {
                self.async_functions.insert(name.clone());
            }
            match item {
                Item::FnDecl {
                    name,
                    is_async: _,
                    type_params,
                    params,
                    ret_type,
                    body,
                    ..
                } if !type_params.is_empty() => {
                    self.generic_fns.insert(
                        name.clone(),
                        GenericFunctionDefinition {
                            type_params: type_params.clone(),
                            params: params.clone(),
                            ret_type: ret_type.clone(),
                            body: body.clone(),
                        },
                    );
                }
                Item::FnDecl {
                    name,
                    is_async,
                    params,
                    ret_type,
                    ..
                } if !matches!(ret_type, Type::Infer) => {
                    if *is_async && name == "main" {
                        self.forward_declare_async_main(name, params, ret_type);
                    } else {
                        self.forward_declare_fn(name, params, ret_type);
                    }
                }
                Item::Stmt(Stmt {
                    kind:
                        StmtKind::VarDecl {
                            name, ty, value, ..
                        },
                    ..
                }) => {
                    let ty = self.resolve_binding_type(ty, value.as_ref())?;
                    let llvm_type = self.llvm_type(&ty);
                    let global = self.module.add_global(llvm_type, None, name);
                    global.set_initializer(&llvm_type.const_zero());
                    self.store_var(name, global.as_pointer_value(), ty);
                }
                Item::TypeAlias { .. } => {}
                _ => {}
            }
        }

        let mut pending: Vec<&Item> = program
            .items
            .iter()
            .filter(|item| {
                matches!(
                    item,
                    Item::FnDecl { type_params, ret_type, .. }
                        if type_params.is_empty() && matches!(ret_type, Type::Infer)
                )
            })
            .collect();
        while !pending.is_empty() {
            let round = std::mem::take(&mut pending);
            let before = round.len();
            let mut deferred = Vec::new();
            for item in round {
                let Item::FnDecl {
                    name,
                    is_async,
                    params,
                    ret_type,
                    body,
                    ..
                } = item
                else {
                    continue;
                };
                let resolved = if *is_async {
                    self.resolved_async_fn_ret_type(name, params, ret_type, body)
                } else {
                    self.resolved_fn_ret_type(name, params, ret_type, body)
                };
                match resolved {
                    Ok(ret_type) if *is_async && name == "main" => {
                        self.forward_declare_async_main(name, params, &ret_type);
                    }
                    Ok(ret_type) => self.forward_declare_fn(name, params, &ret_type),
                    Err(_) => deferred.push(item),
                }
            }
            if deferred.len() == before {
                if let Some(Item::FnDecl {
                    name,
                    params,
                    ret_type,
                    body,
                    ..
                }) = deferred.first()
                {
                    self.resolved_fn_ret_type(name, params, ret_type, body)?;
                }
                break;
            }
            pending = deferred;
        }

        for item in &program.items {
            self.emit_item(item)?;
        }

        if self.module.get_function("main").is_none() {
            self.ensure_main_block()?;
        }
        self.finish_main();

        if let Some(cleanup_name) = cleanup_name {
            self.emit_repl_cleanup(cleanup_name, cleanup_globals)?;
        }

        self.module.verify().map_err(|error| {
            if std::env::var("TYRUS_DUMP_IR").is_ok() {
                eprintln!("{}", self.module.print_to_string().to_string());
            }
            format!("invalid LLVM IR: {error}")
        })?;

        Ok(self.module)
    }

    pub(crate) fn emit_repl_cleanup(
        &mut self,
        cleanup_name: &str,
        globals: &[String],
    ) -> Result<(), String> {
        let function_type = self.context.void_type().fn_type(&[], false);
        let function = self.module.add_function(cleanup_name, function_type, None);
        let entry = self.context.append_basic_block(function, "entry");
        let previous_block = self.builder.get_insert_block();
        self.builder.position_at_end(entry);
        self.emit_owned_native_cleanups_for_names(0, globals);
        self.builder.build_return(None).unwrap();
        if let Some(previous_block) = previous_block {
            self.builder.position_at_end(previous_block);
        }
        Ok(())
    }

    pub fn compile(self, program: &Program, output: &str) -> Result<(), String> {
        let module = self.codegen_and_verify(program)?;

        let ll_path = format!("{output}.ll");
        module
            .print_to_file(Path::new(&ll_path))
            .map_err(|e| format!("error writing IR: {e}"))?;

        Ok(())
    }

    pub fn jit_and_run(self, program: &Program) -> Result<String, String> {
        let trace = |phase: &str| {
            if cfg!(windows) && std::env::var_os("TYRUS_JIT_TRACE").is_some() {
                eprintln!("JIT trace: {phase}");
            }
        };
        trace("codegen");
        let module = self.codegen_and_verify(program)?;
        trace("optimization");

        // Run mem2reg (promote alloca to SSA registers) and instcombine before
        // passing to MCJIT. Without these passes, MCJIT crashed with SIGSEGV on
        // functions that use local variables (alloca) or have cross-function
        // calls within the same module. That was observed on LLVM 17 and has
        // not been re-tested since the move to 22, the passes stay because
        // they are cheap and the failure they avoided was a hard crash, not
        // because the crash is known to still be there.
        Self::optimize_module(&module);
        trace("execution engine");

        use inkwell::OptimizationLevel;

        let engine = module
            .create_jit_execution_engine(OptimizationLevel::None)
            .map_err(|e| {
                let msg = e.to_str().unwrap_or("unknown JIT error");
                format!("error[jit]: {msg}")
            })?;

        trace("static constructors");
        engine.run_static_constructors();

        for item in &program.items {
            if let Item::FnDecl { name, .. } = item {
                let _ = engine.get_function_address(name);
            }
        }

        trace("main lookup");
        let main_addr = engine
            .get_function_address("main")
            .map_err(|_| "error[jit]: main not found".to_string())?;

        trace("main execution");
        let result = capture_stdout(|| unsafe {
            let main_fn: unsafe extern "C" fn() -> i32 =
                std::mem::transmute::<usize, unsafe extern "C" fn() -> i32>(main_addr);
            main_fn();
        })?;

        trace("complete");
        Ok(result)
    }

    /// Run optimization passes on the module to enable MCJIT compatibility.
    ///
    /// MCJIT crashed with SIGSEGV on unoptimized IR that uses alloca (local
    /// variables) or cross-function calls; `mem2reg` eliminates the allocas and
    /// `instcombine` simplifies what is left before the JIT sees it. Measured
    /// on LLVM 17 and not re-verified on 22.
    pub(crate) fn optimize_module(module: &Module<'_>) {
        let target_triple = TargetMachine::get_default_triple();
        if let Ok(target) = Target::from_triple(&target_triple)
            && let Some(target_machine) = target.create_target_machine(
                &target_triple,
                "",
                "",
                inkwell::OptimizationLevel::None,
                RelocMode::Default,
                CodeModel::Default,
            )
        {
            let opts = PassBuilderOptions::create();
            let _ = module.run_passes(
                "mem2reg,instcombine<no-verify-fixpoint>",
                &target_machine,
                opts,
            );
        }
    }

    pub(super) fn resolved_fn_ret_type(
        &self,
        name: &str,
        params: &[Param],
        ret_type: &Type,
        body: &[Stmt],
    ) -> Result<Type, String> {
        if !matches!(ret_type, Type::Infer) {
            return Ok(ret_type.clone());
        }
        let Some(returned) = first_value_return(body) else {
            return Ok(Type::Void);
        };
        let mut bindings: tyrus_core::collections::HashMap<String, Type> = params
            .iter()
            .filter_map(|param| {
                let name = param.pattern.binding_names().first().copied()?;
                Some((name.to_string(), param.ty.clone()))
            })
            .collect();
        for stmt in body {
            if let Stmt {
                kind: StmtKind::VarDecl {
                    name, ty, value, ..
                },
                ..
            } = stmt
            {
                let ty = if matches!(ty, Type::Infer) {
                    value
                        .as_ref()
                        .and_then(|value| self.contextual_expr_type_hint(value, &bindings))
                } else {
                    Some(ty.clone())
                };
                if let Some(ty) = ty {
                    bindings.insert(name.clone(), ty);
                }
            }
        }
        self.contextual_expr_type_hint(returned, &bindings)
            .ok_or_else(|| {
                format!(
                    "error: cannot infer the return type of `{name}`; annotate it explicitly, as in `function {name}(...): <type>`"
                )
            })
    }

    pub(super) fn forward_declare_fn(&mut self, name: &str, params: &[Param], ret_type: &Type) {
        let codegen_types: Vec<_> = params
            .iter()
            .map(|param| {
                let ty = if param.optional && param.default_value.is_none() && !param.rest {
                    Type::Optional(Box::new(param.ty.clone()))
                } else {
                    param.ty.clone()
                };
                self.resolve_type(&ty)
            })
            .collect();
        let codegen_ret_type = self.resolve_type(ret_type);

        let fn_type = if name == "main" && matches!(codegen_ret_type, Type::Void) {
            let types: Vec<_> = codegen_types
                .iter()
                .map(|ty| self.llvm_type(ty).into())
                .collect();
            self.context.i32_type().fn_type(&types, false)
        } else {
            self.llvm_fn_type(&codegen_types, &codegen_ret_type)
        };

        let fn_val = self.module.add_function(name, fn_type, None);
        self.fns.insert(name.to_string(), fn_val);
        self.fn_signatures
            .insert(name.to_string(), (codegen_types, codegen_ret_type));
        self.fn_param_metadata
            .insert(name.to_string(), params.to_vec());
    }

    pub(super) fn specialize_generic_function(
        &mut self,
        name: &str,
        type_args: &[Type],
        args: &[Expr],
    ) -> Result<(FunctionValue<'ctx>, Vec<Type>, Type), String> {
        let definition = self
            .generic_fns
            .get(name)
            .cloned()
            .ok_or_else(|| format!("internal: generic function `{name}` was not registered"))?;
        if !type_args.is_empty() && type_args.len() != definition.type_params.len() {
            return Err(format!(
                "error: generic function `{name}` expects {} type argument(s), found {}",
                definition.type_params.len(),
                type_args.len()
            ));
        }

        let mut substitutions = HashMap::default();
        for (parameter, concrete) in definition.type_params.iter().zip(type_args) {
            substitutions.insert(parameter.name.clone(), self.resolve_type(concrete));
        }
        for (parameter, argument) in definition.params.iter().zip(args) {
            if let Some(actual) = self.codegen_expr_type(argument) {
                infer_codegen_generics(&parameter.ty, &actual, &mut substitutions)?;
            }
        }
        for parameter in &definition.type_params {
            if !substitutions.contains_key(&parameter.name) {
                return Err(format!(
                    "error: cannot infer type parameter `{}` for `{name}`",
                    parameter.name
                ));
            }
        }

        let params = definition
            .params
            .iter()
            .map(|parameter| substitute_type(&parameter.ty, &substitutions))
            .collect::<Vec<_>>();
        let ret_type = substitute_type(&definition.ret_type, &substitutions);
        let symbol = format!(
            "{name}.__tyrus_generic.{}",
            definition
                .type_params
                .iter()
                .map(|parameter| mangle_type(&substitutions[&parameter.name]))
                .collect::<Vec<_>>()
                .join(".")
        );
        if let Some(function) = self.generic_specializations.get(&symbol).copied() {
            return Ok((function, params, ret_type));
        }

        let function =
            self.module
                .add_function(&symbol, self.llvm_fn_type(&params, &ret_type), None);
        self.generic_specializations
            .insert(symbol.clone(), function);

        let previous_block = self.builder.get_insert_block();
        let global_scope = self.vars.first().cloned().unwrap_or_default();
        let previous_vars = std::mem::replace(&mut self.vars, vec![global_scope]);
        let previous_owned_closures =
            std::mem::replace(&mut self.owned_closure_vars, vec![HashSet::default()]);
        let previous_owned_closure_drops =
            std::mem::replace(&mut self.owned_closure_drop_fns, vec![HashMap::default()]);
        let previous_owned_native =
            std::mem::replace(&mut self.owned_native_vars, vec![HashMap::default()]);
        let previous_native_results = std::mem::take(&mut self.pending_owned_native_results);
        let previous_for_of_iterables = std::mem::take(&mut self.for_of_iterable_cleanups);
        let previous_non_zero_vars =
            std::mem::replace(&mut self.non_zero_vars, vec![HashSet::default()]);
        self.type_substitutions.push(substitutions);
        self.return_types.push(ret_type.clone());
        self.push_throw_frame(self.fn_throws.get(name).cloned().unwrap_or_default());
        self.push_scope();

        let result = (|| -> Result<(), String> {
            let entry = self.context.append_basic_block(function, "entry");
            self.builder.position_at_end(entry);
            let collects_parameter_issues = definition
                .params
                .iter()
                .any(|parameter| !parameter.decorators.is_empty());
            if collects_parameter_issues {
                self.emit_validation_issues_begin();
            }
            for (index, (parameter, concrete_type)) in
                definition.params.iter().zip(&params).enumerate()
            {
                match &parameter.pattern {
                    ParamPattern::Name(name) => {
                        let alloca = self
                            .builder
                            .build_alloca(self.llvm_type(concrete_type), name)
                            .unwrap();
                        let value = function.get_nth_param(index as u32).unwrap();
                        self.builder.build_store(alloca, value).unwrap();
                        self.store_var(name, alloca, concrete_type.clone());
                        if super::validators::validators_imply_non_zero(&parameter.decorators) {
                            self.store_non_zero_var(name);
                        }
                        let validator_type =
                            super::validators::validator_parameter_type(parameter, concrete_type);
                        self.emit_parameter_validator_call(
                            &symbol,
                            name,
                            value,
                            &validator_type,
                            &parameter.decorators,
                        )?;
                    }
                    ParamPattern::Array(names) => {
                        let llvm_ty = self.llvm_type(concrete_type);
                        let alloca = self
                            .builder
                            .build_alloca(llvm_ty, "generic_param_array")
                            .unwrap();
                        let value = function.get_nth_param(index as u32).unwrap();
                        self.builder.build_store(alloca, value).unwrap();
                        let array_val = self
                            .builder
                            .build_load(llvm_ty, alloca, "generic_param_array_loaded")
                            .map_err(|e| e.to_string())?
                            .into_struct_value();
                        let data = self
                            .builder
                            .build_extract_value(array_val, 0, "generic_param_data")
                            .unwrap()
                            .into_pointer_value();
                        let length = self
                            .builder
                            .build_extract_value(array_val, 1, "generic_param_len")
                            .unwrap()
                            .into_int_value();
                        let Type::Array(element) = &concrete_type else {
                            return Err(
                                "error: generic array destructure param must have array type"
                                    .to_string(),
                            );
                        };
                        let resolved_element = self.resolve_type(element);
                        let element_llvm = self.llvm_type(&resolved_element);
                        let i64_type = self.context.i64_type();
                        for (j, binding_name) in names.iter().enumerate() {
                            let idx = i64_type.const_int(j as u64, false);
                            self.emit_array_bounds_check(idx, length)?;
                            let slot = unsafe {
                                self.builder
                                    .build_gep(element_llvm, data, &[idx], binding_name)
                                    .unwrap()
                            };
                            let val = self
                                .builder
                                .build_load(element_llvm, slot, binding_name)
                                .map_err(|e| e.to_string())?;
                            let binding_alloca = self
                                .builder
                                .build_alloca(element_llvm, binding_name)
                                .unwrap();
                            self.builder.build_store(binding_alloca, val).unwrap();
                            self.store_var(binding_name, binding_alloca, resolved_element.clone());
                        }
                    }
                    ParamPattern::Object(names) => {
                        let llvm_ty = self.llvm_type(concrete_type);
                        let alloca = self
                            .builder
                            .build_alloca(llvm_ty, "generic_param_obj")
                            .unwrap();
                        let value = function.get_nth_param(index as u32).unwrap();
                        self.builder.build_store(alloca, value).unwrap();
                        let struct_val = self
                            .builder
                            .build_load(llvm_ty, alloca, "generic_param_obj_loaded")
                            .map_err(|e| e.to_string())?
                            .into_struct_value();
                        let Type::Named(name) = &concrete_type else {
                            return Err(
                                "error: generic object destructure param must have a named type"
                                    .to_string(),
                            );
                        };
                        let interface = self
                            .interfaces
                            .get(name.as_str())
                            .cloned()
                            .unwrap_or_default();
                        for binding_name in names {
                            let index = interface
                                .iter()
                                .position(|c| c.name == *binding_name)
                                .ok_or_else(|| {
                                    format!("error: type `{name}` has no field `{binding_name}`")
                                })?;
                            let field_val = self
                                .builder
                                .build_extract_value(struct_val, index as u32, binding_name)
                                .map_err(|e| e.to_string())?;
                            let field_ty = field_storage_type(&interface[index]);
                            let resolved_ty = self.resolve_type(&field_ty);
                            let llvm_field = self.llvm_type(&resolved_ty);
                            let binding_alloca =
                                self.builder.build_alloca(llvm_field, binding_name).unwrap();
                            self.builder.build_store(binding_alloca, field_val).unwrap();
                            self.store_var(binding_name, binding_alloca, resolved_ty);
                        }
                    }
                }
            }
            if collects_parameter_issues {
                self.emit_validation_issues_finish();
            }
            for statement in &definition.body {
                self.emit_stmt(statement, function)?;
            }
            if !self.current_block_terminated() {
                self.emit_deferred_finalizers_in_current_scope(ScopeExitReason::Normal)?;
                if matches!(ret_type, Type::Void) {
                    self.builder.build_return(None).unwrap();
                } else {
                    self.builder.build_unreachable().unwrap();
                }
            }
            Ok(())
        })();

        self.pop_scope();
        self.return_types.pop();
        self.pop_throw_frame();
        self.type_substitutions.pop();
        self.vars = previous_vars;
        self.owned_closure_vars = previous_owned_closures;
        self.owned_closure_drop_fns = previous_owned_closure_drops;
        self.owned_native_vars = previous_owned_native;
        self.pending_owned_native_results = previous_native_results;
        self.for_of_iterable_cleanups = previous_for_of_iterables;
        self.non_zero_vars = previous_non_zero_vars;
        if let Some(block) = previous_block {
            self.builder.position_at_end(block);
        }
        result?;
        Ok((function, params, ret_type))
    }

    pub(super) fn codegen_expr_type(&self, expression: &Expr) -> Option<Type> {
        match expression.kind() {
            ExprKind::IntLit(_) => Some(Type::I32),
            ExprKind::UIntLit(_) => Some(Type::U128),
            ExprKind::FloatLit(_) => Some(Type::F64),
            _ => self
                .expr_type_hint(expression)
                .map(|ty| self.resolve_type(&ty)),
        }
    }

    pub(super) fn emit_item(&mut self, item: &Item) -> Result<(), String> {
        match item {
            Item::InterfaceDecl { .. } => Ok(()),
            Item::BehaviorInterfaceDecl { name, methods, .. } => {
                let effective_targets =
                    self.behavior_targets.get(name).cloned().unwrap_or_default();
                for (target_index, target) in effective_targets.iter().enumerate() {
                    for method in methods {
                        let synthetic = Item::FnDecl {
                            name: format!("__tyrus_at_{}_{}_{}", name, method.name, target_index),
                            is_async: false,
                            is_comptime: false,
                            is_validator: false,
                            type_params: method.type_params.clone(),
                            params: specialize_extension_receiver(&method.params, target),
                            ret_type: method.ret_type.clone(),
                            throws: method.throws.clone(),
                            body: method.body.clone(),
                            span: method.span,
                        };
                        self.emit_item(&synthetic)?;
                    }
                }
                Ok(())
            }
            Item::TypeAlias { .. } => Ok(()),
            Item::EnumDecl { .. } => Ok(()),
            Item::FnDecl {
                name,
                is_async,
                type_params,
                params,
                ret_type,
                body,
                ..
            } => {
                if !type_params.is_empty() {
                    return Ok(());
                }
                if *is_async {
                    let ret_type = self.resolved_async_fn_ret_type(name, params, ret_type, body)?;
                    return self.emit_async_function(name, params, &ret_type, body);
                }
                let ret_type = &self.resolved_fn_ret_type(name, params, ret_type, body)?;
                let fn_val = *self
                    .fns
                    .get(name)
                    .ok_or_else(|| format!("internal: function `{name}` not forward-declared"))?;

                if name == "main" {
                    if let Some(block) = self.main_continuation(fn_val) {
                        if block.get_terminator().is_some() {
                            return Err("error: `main` was terminated before its body".to_string());
                        }
                        self.builder.position_at_end(block);
                    } else {
                        let entry = self.context.append_basic_block(fn_val, "entry");
                        self.builder.position_at_end(entry);
                    }
                    self.emit_env_startup()?;
                } else {
                    let entry = self.context.append_basic_block(fn_val, "entry");
                    self.builder.position_at_end(entry);
                }

                self.push_scope();
                self.return_types.push(ret_type.clone());
                self.push_throw_frame(
                    self.fn_throws
                        .get(name.as_str())
                        .cloned()
                        .unwrap_or_default(),
                );

                let collects_parameter_issues =
                    params.iter().any(|param| !param.decorators.is_empty());
                if collects_parameter_issues {
                    self.emit_validation_issues_begin();
                }
                let mut fixed_param_count = 0;
                for param in params {
                    let storage_type = if param.optional && param.default_value.is_none() {
                        Type::Optional(Box::new(param.ty.clone()))
                    } else {
                        param.ty.clone()
                    };
                    let storage_type = self.resolve_type(&storage_type);

                    match &param.pattern {
                        ParamPattern::Name(param_name) => {
                            let llvm_ty = self.llvm_type(&storage_type);
                            let alloca = self.builder.build_alloca(llvm_ty, param_name).unwrap();
                            let arg_val = fn_val.get_nth_param(fixed_param_count as u32).unwrap();
                            self.builder.build_store(alloca, arg_val).unwrap();
                            self.store_var(param_name, alloca, storage_type.clone());
                            if super::validators::validators_imply_non_zero(&param.decorators) {
                                self.store_non_zero_var(param_name);
                            }
                            self.emit_parameter_validator_call(
                                name,
                                param_name,
                                arg_val,
                                &storage_type,
                                &param.decorators,
                            )?;
                            fixed_param_count += 1;
                        }
                        ParamPattern::Array(names) => {
                            let llvm_ty = self.llvm_type(&storage_type);
                            let alloca = self.builder.build_alloca(llvm_ty, "param_array").unwrap();
                            let arg_val = fn_val.get_nth_param(fixed_param_count as u32).unwrap();
                            self.builder.build_store(alloca, arg_val).unwrap();

                            let array_val = self
                                .builder
                                .build_load(llvm_ty, alloca, "param_array_loaded")
                                .map_err(|e| e.to_string())?
                                .into_struct_value();

                            let data = self
                                .builder
                                .build_extract_value(array_val, 0, "param_data")
                                .unwrap()
                                .into_pointer_value();
                            let length = self
                                .builder
                                .build_extract_value(array_val, 1, "param_len")
                                .unwrap()
                                .into_int_value();

                            let Type::Array(element) = &storage_type else {
                                return Err("error: array destructure param must have array type"
                                    .to_string());
                            };
                            let resolved_element = self.resolve_type(element);
                            let element_llvm = self.llvm_type(&resolved_element);
                            let i64_type = self.context.i64_type();
                            for (j, binding_name) in names.iter().enumerate() {
                                let idx = i64_type.const_int(j as u64, false);
                                self.emit_array_bounds_check(idx, length)?;
                                let slot = unsafe {
                                    self.builder
                                        .build_gep(element_llvm, data, &[idx], binding_name)
                                        .unwrap()
                                };
                                let val = self
                                    .builder
                                    .build_load(element_llvm, slot, binding_name)
                                    .map_err(|e| e.to_string())?;
                                let binding_alloca = self
                                    .builder
                                    .build_alloca(element_llvm, binding_name)
                                    .unwrap();
                                self.builder.build_store(binding_alloca, val).unwrap();
                                self.store_var(
                                    binding_name,
                                    binding_alloca,
                                    resolved_element.clone(),
                                );
                            }
                            fixed_param_count += 1;
                        }
                        ParamPattern::Object(names) => {
                            let llvm_ty = self.llvm_type(&storage_type);
                            let alloca = self.builder.build_alloca(llvm_ty, "param_obj").unwrap();
                            let arg_val = fn_val.get_nth_param(fixed_param_count as u32).unwrap();
                            self.builder.build_store(alloca, arg_val).unwrap();

                            let struct_val = self
                                .builder
                                .build_load(llvm_ty, alloca, "param_obj_loaded")
                                .map_err(|e| e.to_string())?
                                .into_struct_value();

                            let (interface, type_name) = match &storage_type {
                                Type::Named(name) => (
                                    self.interfaces
                                        .get(name.as_str())
                                        .cloned()
                                        .unwrap_or_default(),
                                    name.clone(),
                                ),
                                Type::Object(fields) => (fields.clone(), "{ ... }".to_string()),
                                _ => {
                                    return Err(
                                        "error: object destructure param must have a named or inline object type"
                                            .to_string(),
                                    );
                                }
                            };
                            for binding_name in names {
                                let index = interface
                                    .iter()
                                    .position(|c| c.name == *binding_name)
                                    .ok_or_else(|| {
                                        format!("error: type `{type_name}` has no field `{binding_name}`")
                                    })?;
                                let field_val = self
                                    .builder
                                    .build_extract_value(struct_val, index as u32, binding_name)
                                    .map_err(|e| e.to_string())?;
                                let field_ty = field_storage_type(&interface[index]);
                                let resolved_ty = self.resolve_type(&field_ty);
                                let llvm_field = self.llvm_type(&resolved_ty);
                                let binding_alloca =
                                    self.builder.build_alloca(llvm_field, binding_name).unwrap();
                                self.builder.build_store(binding_alloca, field_val).unwrap();
                                self.store_var(binding_name, binding_alloca, resolved_ty);
                            }
                            fixed_param_count += 1;
                        }
                    }
                }
                if collects_parameter_issues {
                    self.emit_validation_issues_finish();
                }

                for stmt in body {
                    self.emit_stmt(stmt, fn_val)?;
                }

                if !self.current_block_terminated() {
                    self.emit_deferred_finalizers_in_current_scope(ScopeExitReason::Normal)?;
                    self.emit_owned_closure_cleanups(None)?;
                    if name == "main" && matches!(ret_type, Type::Void) {
                        self.builder
                            .build_return(Some(&self.context.i32_type().const_zero()))
                            .unwrap();
                    } else if matches!(ret_type, Type::Void) {
                        self.builder.build_return(None).unwrap();
                    } else {
                        self.builder.build_unreachable().unwrap();
                    }
                }

                self.pending_owned_native_results.clear();
                self.pop_scope();
                self.return_types.pop();
                self.pop_throw_frame();
                Ok(())
            }

            Item::Stmt(
                stmt @ Stmt {
                    kind: StmtKind::VarDecl { .. },
                    ..
                },
            ) => self.ensure_main_and_emit_global_var(stmt),
            Item::Stmt(stmt) => self.ensure_main_and_emit(stmt),
        }
    }

    fn recursive_control_loop_targets(
        &self,
        outer_index: usize,
        contexts: &[AsyncControlContext],
        blocks: &AsyncBlockRegistry<'ctx>,
    ) -> Option<(BasicBlock<'ctx>, BasicBlock<'ctx>, usize)> {
        contexts
            .iter()
            .enumerate()
            .rev()
            .find_map(|(depth, context)| {
                matches!(
                    context,
                    AsyncControlContext::While { .. }
                        | AsyncControlContext::DoWhile { .. }
                        | AsyncControlContext::For { .. }
                        | AsyncControlContext::ForOf { .. }
                )
                .then(|| blocks.recursive_control_blocks.get(&(outer_index, depth)))
                .flatten()
                .and_then(|blocks| match blocks {
                    AsyncRecursiveControlBlocks::Loop {
                        continue_target,
                        after,
                        finally_depth,
                        ..
                    } => Some((*continue_target, *after, *finally_depth)),
                    _ => None,
                })
            })
    }

    fn emit_recursive_control_statements(
        &mut self,
        statements: &[Stmt],
        outer_index: usize,
        active_contexts: &[AsyncControlContext],
        blocks: &AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let loop_targets =
            self.recursive_control_loop_targets(outer_index, active_contexts, blocks);
        if let Some((continue_target, after, finally_depth)) = loop_targets {
            self.push_loop_control_with_finally_depth(continue_target, after, finally_depth);
        }
        for statement in statements {
            if self.current_block_terminated() {
                break;
            }
            self.emit_stmt(statement, poll)?;
        }
        if loop_targets.is_some() {
            self.pop_loop_control();
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::too_many_lines)]
    fn begin_recursive_control_path_from(
        &mut self,
        outer_index: usize,
        contexts: &[AsyncControlContext],
        enter_depth: usize,
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &[AsyncFinallyAwaitGroup],
        blocks: &mut AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_for_of_slots: &HashMap<
            AsyncForOfSlotKey,
            (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
        >,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        for (depth, context) in contexts.iter().enumerate().skip(enter_depth) {
            match context {
                AsyncControlContext::Block { before, .. } => {
                    blocks
                        .recursive_control_blocks
                        .insert((outer_index, depth), AsyncRecursiveControlBlocks::Block);
                    self.emit_recursive_control_statements(
                        before,
                        outer_index,
                        &contexts[..depth],
                        blocks,
                        poll,
                    )?;
                }
                AsyncControlContext::If {
                    condition,
                    before,
                    other,
                    selected_in_then,
                    ..
                } => {
                    let selected = self
                        .context
                        .append_basic_block(poll, "async.recursive.if.selected");
                    let alternative = self
                        .context
                        .append_basic_block(poll, "async.recursive.if.alternative");
                    let join = self
                        .context
                        .append_basic_block(poll, "async.recursive.if.after");
                    blocks.recursive_control_blocks.insert(
                        (outer_index, depth),
                        AsyncRecursiveControlBlocks::If { join },
                    );
                    let value = self.emit_condition_value(condition)?;
                    let (then_block, else_block) = if *selected_in_then {
                        (selected, alternative)
                    } else {
                        (alternative, selected)
                    };
                    self.builder
                        .build_conditional_branch(value, then_block, else_block)
                        .unwrap();
                    self.builder.position_at_end(alternative);
                    self.push_scope();
                    self.apply_condition_narrowing(condition, !*selected_in_then);
                    self.emit_recursive_control_statements(
                        other,
                        outer_index,
                        &contexts[..depth],
                        blocks,
                        poll,
                    )?;
                    self.pop_scope();
                    if !self.current_block_terminated() {
                        self.builder.build_unconditional_branch(join).unwrap();
                    }
                    self.builder.position_at_end(selected);
                    self.push_scope();
                    self.apply_condition_narrowing(condition, *selected_in_then);
                    self.emit_recursive_control_statements(
                        before,
                        outer_index,
                        &contexts[..=depth],
                        blocks,
                        poll,
                    )?;
                }
                AsyncControlContext::Try {
                    try_id,
                    finally_body,
                    before,
                    ..
                } => {
                    let suspending_finally =
                        direct_awaits_in_block(finally_body).is_some_and(|sites| !sites.is_empty());
                    let join = self
                        .context
                        .append_basic_block(poll, "async.recursive.try.after");
                    blocks.recursive_control_blocks.insert(
                        (outer_index, depth),
                        AsyncRecursiveControlBlocks::Try { join },
                    );
                    blocks.recursive_try_blocks.insert(*try_id, join);
                    let async_finally = finally_await_groups
                        .iter()
                        .find(|group| group.recursive_try_id == Some(*try_id));
                    if let Some(group) = async_finally {
                        let loop_targets = self.recursive_control_loop_targets(
                            outer_index,
                            &contexts[..depth],
                            blocks,
                        );
                        let completion = group.completion.as_ref().ok_or_else(|| {
                            "internal: recursive async finally has no completion storage"
                                .to_string()
                        })?;
                        let entry = self
                            .context
                            .append_basic_block(poll, "async.recursive.finally.entry");
                        let (mode_slot, _) = self
                            .async_frame_slot(&completion.mode_storage)
                            .ok_or_else(|| {
                                "internal: recursive async finally mode has no frame slot"
                                    .to_string()
                            })?;
                        let (throw_slot, _) = self
                            .async_frame_slot(&completion.throw_storage)
                            .ok_or_else(|| {
                                "internal: recursive async finally throw has no frame slot"
                                    .to_string()
                            })?;
                        let return_slot = self
                            .async_frame_slot(&completion.return_storage)
                            .map(|(slot, ty)| (slot, self.resolve_type(&ty)));
                        let return_storage = return_slot
                            .is_some()
                            .then(|| completion.return_storage.clone());
                        blocks.finally_completion_blocks.insert(
                            group.mode_storage.clone(),
                            AsyncFinallyCompletionBlocks {
                                entry,
                                normal: join,
                                break_target: loop_targets.map(|(_, target, _)| target),
                                continue_target: loop_targets.map(|(target, _, _)| target),
                            },
                        );
                        self.async_finally_completion_scopes
                            .push(AsyncFinallyCompletionScope {
                                mode_slot,
                                return_slot,
                                return_storage,
                                entry,
                                throw_entry: None,
                                throw_storage: completion.throw_storage.clone(),
                                break_target: loop_targets.map(|(_, target, finally_depth)| {
                                    BreakTarget {
                                        block: target,
                                        finally_depth,
                                        validation_depth: self.validation_cleanup_scopes.len(),
                                        scope_depth: self.owned_native_vars.len(),
                                        value_slot: None,
                                    }
                                }),
                                continue_target: loop_targets
                                    .map(|(target, _, finally_depth)| (target, finally_depth)),
                                throw_slot,
                                intercept_throw: false,
                            });
                        let preceding = finally_await_groups
                            .iter()
                            .take_while(|candidate| candidate.mode_storage != group.mode_storage)
                            .map(|candidate| candidate.sites.len())
                            .sum::<usize>();
                        let state = await_sites.len()
                            + catch_await_groups
                                .iter()
                                .map(|candidate| candidate.sites.len())
                                .sum::<usize>()
                            + preceding
                            + 1;
                        let current = self.builder.get_insert_block().ok_or_else(|| {
                            "internal: recursive async finally entry has no source block"
                                .to_string()
                        })?;
                        self.builder.position_at_end(entry);
                        self.emit_async_begin_finally(
                            poll,
                            frame_type,
                            frame,
                            state_pointer,
                            state as u64,
                            group,
                            None,
                        )?;
                        self.builder.position_at_end(current);
                    }
                    if !finally_body.is_empty() && !suspending_finally {
                        let recursive_loop_depth = contexts[..depth]
                            .iter()
                            .filter(|context| {
                                matches!(
                                    context,
                                    AsyncControlContext::While { .. }
                                        | AsyncControlContext::DoWhile { .. }
                                        | AsyncControlContext::For { .. }
                                        | AsyncControlContext::ForOf { .. }
                                )
                            })
                            .count();
                        self.finally_scopes.push(FinallyScope {
                            body: finally_body.clone(),
                            catches_throw: true,
                            loop_depth: self.loop_stack.len() + recursive_loop_depth,
                        });
                    }
                    self.push_scope();
                    self.emit_recursive_control_statements(
                        before,
                        outer_index,
                        &contexts[..depth],
                        blocks,
                        poll,
                    )?;
                }
                AsyncControlContext::While {
                    condition, before, ..
                } => {
                    let loop_condition = self
                        .context
                        .append_basic_block(poll, "async.recursive.while.condition");
                    let loop_body = self
                        .context
                        .append_basic_block(poll, "async.recursive.while.body");
                    let loop_after = self
                        .context
                        .append_basic_block(poll, "async.recursive.while.after");
                    blocks.recursive_control_blocks.insert(
                        (outer_index, depth),
                        AsyncRecursiveControlBlocks::Loop {
                            body: loop_body,
                            continue_target: loop_condition,
                            condition: loop_condition,
                            after: loop_after,
                            finally_depth: self.finally_scopes.len(),
                        },
                    );
                    self.builder
                        .build_unconditional_branch(loop_condition)
                        .unwrap();
                    self.builder.position_at_end(loop_condition);
                    let value = self.emit_condition_value(condition)?;
                    self.builder
                        .build_conditional_branch(value, loop_body, loop_after)
                        .unwrap();
                    self.builder.position_at_end(loop_body);
                    self.push_scope();
                    self.emit_recursive_control_statements(
                        before,
                        outer_index,
                        &contexts[..=depth],
                        blocks,
                        poll,
                    )?;
                }
                AsyncControlContext::DoWhile { before, .. } => {
                    let loop_body = self
                        .context
                        .append_basic_block(poll, "async.recursive.do_while.body");
                    let loop_condition = self
                        .context
                        .append_basic_block(poll, "async.recursive.do_while.condition");
                    let loop_after = self
                        .context
                        .append_basic_block(poll, "async.recursive.do_while.after");
                    blocks.recursive_control_blocks.insert(
                        (outer_index, depth),
                        AsyncRecursiveControlBlocks::Loop {
                            body: loop_body,
                            continue_target: loop_condition,
                            condition: loop_condition,
                            after: loop_after,
                            finally_depth: self.finally_scopes.len(),
                        },
                    );
                    self.builder.build_unconditional_branch(loop_body).unwrap();
                    self.builder.position_at_end(loop_body);
                    self.push_scope();
                    self.emit_recursive_control_statements(
                        before,
                        outer_index,
                        &contexts[..=depth],
                        blocks,
                        poll,
                    )?;
                }
                AsyncControlContext::For {
                    init,
                    condition,
                    before,
                    ..
                } => {
                    let loop_body = self
                        .context
                        .append_basic_block(poll, "async.recursive.for.body");
                    let update = self
                        .context
                        .append_basic_block(poll, "async.recursive.for.update");
                    let loop_condition = self
                        .context
                        .append_basic_block(poll, "async.recursive.for.condition");
                    let loop_after = self
                        .context
                        .append_basic_block(poll, "async.recursive.for.after");
                    blocks.recursive_control_blocks.insert(
                        (outer_index, depth),
                        AsyncRecursiveControlBlocks::Loop {
                            body: loop_body,
                            continue_target: update,
                            condition: loop_condition,
                            after: loop_after,
                            finally_depth: self.finally_scopes.len(),
                        },
                    );
                    self.push_scope();
                    self.emit_stmt(init, poll)?;
                    self.bind_async_frame_locals();
                    if !self.current_block_terminated() {
                        self.builder
                            .build_unconditional_branch(loop_condition)
                            .unwrap();
                    }
                    self.builder.position_at_end(loop_condition);
                    let value = self.emit_condition_value(condition)?;
                    self.builder
                        .build_conditional_branch(value, loop_body, loop_after)
                        .unwrap();
                    self.builder.position_at_end(loop_body);
                    self.push_scope();
                    self.emit_recursive_control_statements(
                        before,
                        outer_index,
                        &contexts[..=depth],
                        blocks,
                        poll,
                    )?;
                }
                AsyncControlContext::ForOf {
                    name,
                    iterable,
                    element_type,
                    before,
                    ..
                } => {
                    let (data_slot, length_slot, index_slot) = async_for_of_slots
                        .get(&AsyncForOfSlotKey::RecursiveControl(outer_index, depth))
                        .copied()
                        .ok_or_else(|| {
                            "internal: recursive async for...of frame slots are missing".to_string()
                        })?;
                    let array = self.emit_expr(iterable)?.into_struct_value();
                    let data = self
                        .builder
                        .build_extract_value(array, 0, "async.recursive.for_of.data")
                        .unwrap();
                    let length = self
                        .builder
                        .build_extract_value(array, 1, "async.recursive.for_of.length")
                        .unwrap();
                    self.builder.build_store(data_slot, data).unwrap();
                    self.builder.build_store(length_slot, length).unwrap();
                    self.builder
                        .build_store(index_slot, self.context.i64_type().const_zero())
                        .unwrap();
                    let loop_body = self
                        .context
                        .append_basic_block(poll, "async.recursive.for_of.body");
                    let update = self
                        .context
                        .append_basic_block(poll, "async.recursive.for_of.update");
                    let loop_condition = self
                        .context
                        .append_basic_block(poll, "async.recursive.for_of.condition");
                    let loop_after = self
                        .context
                        .append_basic_block(poll, "async.recursive.for_of.after");
                    blocks.recursive_control_blocks.insert(
                        (outer_index, depth),
                        AsyncRecursiveControlBlocks::Loop {
                            body: loop_body,
                            continue_target: update,
                            condition: loop_condition,
                            after: loop_after,
                            finally_depth: self.finally_scopes.len(),
                        },
                    );
                    self.builder
                        .build_unconditional_branch(loop_condition)
                        .unwrap();
                    self.builder.position_at_end(loop_condition);
                    let index = self
                        .builder
                        .build_load(
                            self.context.i64_type(),
                            index_slot,
                            "async.recursive.for_of.index",
                        )
                        .unwrap()
                        .into_int_value();
                    let length = self
                        .builder
                        .build_load(
                            self.context.i64_type(),
                            length_slot,
                            "async.recursive.for_of.length",
                        )
                        .unwrap()
                        .into_int_value();
                    let has_next = self
                        .builder
                        .build_int_compare(
                            inkwell::IntPredicate::ULT,
                            index,
                            length,
                            "async.recursive.for_of.has_next",
                        )
                        .unwrap();
                    self.builder
                        .build_conditional_branch(has_next, loop_body, loop_after)
                        .unwrap();
                    self.builder.position_at_end(loop_body);
                    let data = self
                        .builder
                        .build_load(pointer, data_slot, "async.recursive.for_of.data.load")
                        .unwrap()
                        .into_pointer_value();
                    let element_pointer = unsafe {
                        self.builder
                            .build_gep(
                                self.llvm_type(element_type),
                                data,
                                &[index],
                                "async.recursive.for_of.element.pointer",
                            )
                            .unwrap()
                    };
                    let element = self
                        .builder
                        .build_load(
                            self.llvm_type(element_type),
                            element_pointer,
                            "async.recursive.for_of.element",
                        )
                        .unwrap();
                    let (binding_slot, _) = self.async_frame_slot(name).ok_or_else(|| {
                        format!(
                            "internal: recursive async for...of binding `{name}` has no frame slot"
                        )
                    })?;
                    self.builder.build_store(binding_slot, element).unwrap();
                    self.push_scope();
                    self.store_var(name, binding_slot, element_type.clone());
                    self.emit_recursive_control_statements(
                        before,
                        outer_index,
                        &contexts[..=depth],
                        blocks,
                        poll,
                    )?;
                }
            }
            if self.current_block_terminated() {
                return Ok(());
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn finish_recursive_control_path_to(
        &mut self,
        outer_index: usize,
        contexts: &[AsyncControlContext],
        exit_depth: usize,
        await_sites: &[AsyncAwaitSite],
        catch_await_groups: &[AsyncCatchAwaitGroup],
        finally_await_groups: &[AsyncFinallyAwaitGroup],
        blocks: &AsyncBlockRegistry<'ctx>,
        poll: FunctionValue<'ctx>,
        frame_type: inkwell::types::StructType<'ctx>,
        frame: PointerValue<'ctx>,
        state_pointer: PointerValue<'ctx>,
        async_for_of_slots: &HashMap<
            AsyncForOfSlotKey,
            (PointerValue<'ctx>, PointerValue<'ctx>, PointerValue<'ctx>),
        >,
    ) -> Result<(), String> {
        for (depth, context) in contexts.iter().enumerate().rev() {
            if depth < exit_depth {
                continue;
            }
            match context {
                AsyncControlContext::Block { after, .. } => {
                    self.emit_recursive_control_statements(
                        after,
                        outer_index,
                        &contexts[..depth],
                        blocks,
                        poll,
                    )?;
                }
                AsyncControlContext::If { after, .. } => {
                    self.emit_recursive_control_statements(
                        after,
                        outer_index,
                        &contexts[..depth],
                        blocks,
                        poll,
                    )?;
                    self.pop_scope();
                    let AsyncRecursiveControlBlocks::If { join } = blocks
                        .recursive_control_blocks
                        .get(&(outer_index, depth))
                        .copied()
                        .ok_or_else(|| {
                            "internal: recursive async if continuation is missing".to_string()
                        })?
                    else {
                        return Err(
                            "internal: recursive async if block kind is invalid".to_string()
                        );
                    };
                    if !self.current_block_terminated() {
                        self.builder.build_unconditional_branch(join).unwrap();
                    }
                    self.builder.position_at_end(join);
                }
                AsyncControlContext::Try {
                    try_id,
                    finally_body,
                    after,
                    ..
                } => {
                    let async_finally = finally_await_groups
                        .iter()
                        .find(|group| group.recursive_try_id == Some(*try_id));
                    self.emit_recursive_control_statements(
                        after,
                        outer_index,
                        &contexts[..depth],
                        blocks,
                        poll,
                    )?;
                    self.pop_scope();
                    if !finally_body.is_empty() && async_finally.is_none() {
                        self.finally_scopes.pop();
                    }
                    if let Some(group) = async_finally {
                        self.async_finally_completion_scopes.pop();
                        let completion = group.completion.as_ref().ok_or_else(|| {
                            "internal: recursive async finally has no completion storage"
                                .to_string()
                        })?;
                        let (mode_slot, _) = self
                            .async_frame_slot(&completion.mode_storage)
                            .ok_or_else(|| {
                                "internal: recursive async finally mode has no frame slot"
                                    .to_string()
                            })?;
                        if !self.current_block_terminated() {
                            self.builder
                                .build_store(mode_slot, self.context.i32_type().const_zero())
                                .unwrap();
                        }
                    }
                    if !self.current_block_terminated() && async_finally.is_none() {
                        for statement in finally_body {
                            self.emit_stmt(statement, poll)?;
                            if self.current_block_terminated() {
                                break;
                            }
                        }
                    }
                    let AsyncRecursiveControlBlocks::Try { join } = blocks
                        .recursive_control_blocks
                        .get(&(outer_index, depth))
                        .copied()
                        .ok_or_else(|| {
                            "internal: recursive async try continuation is missing".to_string()
                        })?
                    else {
                        return Err(
                            "internal: recursive async try block kind is invalid".to_string()
                        );
                    };
                    if !self.current_block_terminated() {
                        if let Some(group) = async_finally {
                            let preceding = finally_await_groups
                                .iter()
                                .take_while(|candidate| {
                                    candidate.mode_storage != group.mode_storage
                                })
                                .map(|candidate| candidate.sites.len())
                                .sum::<usize>();
                            let state = await_sites.len()
                                + catch_await_groups
                                    .iter()
                                    .map(|candidate| candidate.sites.len())
                                    .sum::<usize>()
                                + preceding
                                + 1;
                            self.emit_async_begin_finally(
                                poll,
                                frame_type,
                                frame,
                                state_pointer,
                                state as u64,
                                group,
                                None,
                            )?;
                        } else {
                            self.builder.build_unconditional_branch(join).unwrap();
                        }
                    }
                    self.builder.position_at_end(join);
                }
                AsyncControlContext::While { after, .. }
                | AsyncControlContext::DoWhile { after, .. }
                | AsyncControlContext::For { after, .. }
                | AsyncControlContext::ForOf { after, .. } => {
                    let AsyncRecursiveControlBlocks::Loop {
                        body,
                        continue_target,
                        condition,
                        after: loop_after,
                        ..
                    } = blocks
                        .recursive_control_blocks
                        .get(&(outer_index, depth))
                        .copied()
                        .ok_or_else(|| {
                            "internal: recursive async loop continuation is missing".to_string()
                        })?
                    else {
                        return Err(
                            "internal: recursive async loop block kind is invalid".to_string()
                        );
                    };
                    self.emit_recursive_control_statements(
                        after,
                        outer_index,
                        &contexts[..=depth],
                        blocks,
                        poll,
                    )?;
                    self.pop_scope();
                    if !self.current_block_terminated() {
                        self.builder
                            .build_unconditional_branch(continue_target)
                            .unwrap();
                    }
                    match context {
                        AsyncControlContext::While { .. } => {}
                        AsyncControlContext::DoWhile {
                            condition: expression,
                            ..
                        } => {
                            self.builder.position_at_end(continue_target);
                            let value = self.emit_expr(expression)?.into_int_value();
                            self.builder
                                .build_conditional_branch(value, body, loop_after)
                                .unwrap();
                        }
                        AsyncControlContext::For { update, .. } => {
                            self.builder.position_at_end(continue_target);
                            self.emit_stmt(update, poll)?;
                            if !self.current_block_terminated() {
                                self.builder.build_unconditional_branch(condition).unwrap();
                            }
                        }
                        AsyncControlContext::ForOf { .. } => {
                            self.builder.position_at_end(continue_target);
                            let (_, _, index_slot) = async_for_of_slots
                                .get(&AsyncForOfSlotKey::RecursiveControl(outer_index, depth))
                                .copied()
                                .ok_or_else(|| {
                                    "internal: recursive async for...of frame slots are missing"
                                        .to_string()
                                })?;
                            let index = self
                                .builder
                                .build_load(
                                    self.context.i64_type(),
                                    index_slot,
                                    "async.recursive.for_of.update.index",
                                )
                                .unwrap()
                                .into_int_value();
                            let next = self
                                .builder
                                .build_int_add(
                                    index,
                                    self.context.i64_type().const_int(1, false),
                                    "async.recursive.for_of.next.index",
                                )
                                .unwrap();
                            self.builder.build_store(index_slot, next).unwrap();
                            self.builder.build_unconditional_branch(condition).unwrap();
                        }
                        AsyncControlContext::Block { .. }
                        | AsyncControlContext::If { .. }
                        | AsyncControlContext::Try { .. } => {
                            unreachable!()
                        }
                    }
                    if matches!(context, AsyncControlContext::For { .. }) {
                        self.pop_scope();
                    }
                    self.builder.position_at_end(loop_after);
                    self.bind_async_frame_locals();
                }
            }
        }
        Ok(())
    }

    fn same_statements(left: &[Stmt], right: &[Stmt]) -> bool {
        left.len() == right.len()
            && left
                .iter()
                .zip(right)
                .all(|(left, right)| left.span == right.span)
    }

    fn emit_keep_thrown_error_subtype_in_frame(
        &mut self,
        poll: FunctionValue<'ctx>,
        context_slot: PointerValue<'ctx>,
        kept: &ErrorSubtypeStorage<'ctx>,
        frame: &ErrorSubtypeStorage<'ctx>,
    ) -> Result<(), String> {
        let pointer = self.context.ptr_type(inkwell::AddressSpace::default());
        let i64_type = self.context.i64_type();
        let tag = self
            .builder
            .build_load(i64_type, kept.tag_slot, "async.catch.frame.tag")
            .unwrap()
            .into_int_value();
        self.builder.build_store(frame.tag_slot, tag).unwrap();
        for ((name, stack_slot), (_, frame_slot)) in kept.slots.iter().zip(frame.slots.iter()) {
            let subtype = Type::Named(name.clone());
            if let Some(local) = self.async_frame_slot_name(*frame_slot) {
                self.clear_async_frame_local_before_store(poll, &local, *frame_slot, &subtype)?;
                self.mark_async_frame_value_initialized(&local, false);
            }
            let matched = self
                .context
                .append_basic_block(poll, "async.catch.frame.subtype");
            let next = self
                .context
                .append_basic_block(poll, "async.catch.frame.subtype.next");
            let is_subtype = self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    tag,
                    i64_type.const_int(self.thrown_type_tag(&subtype), false),
                    "async.catch.frame.is",
                )
                .unwrap();
            self.builder
                .build_conditional_branch(is_subtype, matched, next)
                .unwrap();
            self.builder.position_at_end(matched);
            let value = self
                .builder
                .build_load(
                    self.llvm_type(&subtype),
                    *stack_slot,
                    "async.catch.frame.value",
                )
                .unwrap();
            self.emit_async_clone_value_into(*frame_slot, value, &subtype)?;
            self.mark_async_frame_slot_initialized(*frame_slot, true);
            self.builder.build_unconditional_branch(next).unwrap();
            self.builder.position_at_end(next);
        }
        let owned_field = self
            .builder
            .build_struct_gep(
                self.catch_ctx_type(),
                context_slot,
                3,
                "async.catch.frame.owned_field",
            )
            .unwrap();
        let owned = self
            .builder
            .build_load(pointer, owned_field, "async.catch.frame.owned")
            .unwrap()
            .into_pointer_value();
        self.emit_error_subtype_storage_release(poll, kept, Some(owned))
    }

    fn catch_binding_reads_thrown_cell(&self, ty: &Type) -> bool {
        match ty {
            Type::Str => false,
            Type::Named(name) if name == "Error" || name == "AggregateError" => false,
            Type::Union(variants) => !Self::union_is_payload_free(variants),
            other => !self.is_structured_error_binding(other),
        }
    }

    pub(super) fn is_structured_error_binding(&self, ty: &Type) -> bool {
        let Type::Named(name) = ty else {
            return false;
        };
        let Some(fields) = self.interfaces.get(name) else {
            return false;
        };
        fields.len() == 2
            && fields[0].name == "code"
            && matches!(fields[0].ty, Type::Str)
            && fields[1].name == "message"
            && matches!(fields[1].ty, Type::Str)
    }

    pub(super) fn emit_array_element_clone(
        &mut self,
        element: &Type,
    ) -> Result<FunctionValue<'ctx>, String> {
        self.emit_async_array_element_clone(element)
    }

    pub(super) fn ensure_main_and_emit(&mut self, stmt: &Stmt) -> Result<(), String> {
        let main_fn = self.ensure_main_block()?;
        self.emit_stmt(stmt, main_fn)?;
        self.remember_main_continuation();
        Ok(())
    }

    pub(super) fn emit_runtime_shutdown(&mut self) {
        let shutdown = self.declare_libc_fn("tyrus_runtime_shutdown", &[], None);
        self.builder.build_call(shutdown, &[], "").unwrap();
    }

    fn remember_main_continuation(&mut self) {
        let main = self.module.get_function("main");
        if let Some(block) = self.builder.get_insert_block()
            && block.get_parent() == main
        {
            self.main_block = Some(block);
        }
    }

    fn main_continuation(&self, main: FunctionValue<'ctx>) -> Option<BasicBlock<'ctx>> {
        self.main_block
            .filter(|block| block.get_parent() == Some(main))
            .or_else(|| main.get_last_basic_block())
    }

    pub(super) fn ensure_main_block(&mut self) -> Result<FunctionValue<'ctx>, String> {
        let main_fn = if let Some(f) = self.module.get_function("main") {
            let current_function = self
                .builder
                .get_insert_block()
                .and_then(inkwell::basic_block::BasicBlock::get_parent);
            if current_function == Some(f) {
                if self.current_block_terminated() {
                    return Err(
                        "error: an explicit `main` function runs automatically as the entry point; remove the trailing top-level statement (e.g. the `main()` call)"
                            .to_string(),
                    );
                }
            } else {
                let block = self
                    .main_continuation(f)
                    .unwrap_or_else(|| self.context.append_basic_block(f, "entry"));
                if block.get_terminator().is_some() {
                    return Err(
                        "error: an explicit `main` function runs automatically as the entry point; remove the trailing top-level statement (e.g. the `main()` call)"
                            .to_string(),
                    );
                }
                self.builder.position_at_end(block);
            }
            f
        } else {
            let main_type = self.context.i32_type().fn_type(&[], false);
            let f = self.module.add_function("main", main_type, None);
            let entry = self.context.append_basic_block(f, "entry");
            self.builder.position_at_end(entry);
            f
        };
        self.emit_env_startup()?;
        Ok(main_fn)
    }

    fn emit_constant_global_array(
        &mut self,
        name: &str,
        value: &Expr,
        ty: &Type,
        global: inkwell::values::GlobalValue<'ctx>,
    ) -> Result<bool, String> {
        let Type::Array(element) = transparent_codegen_type(ty).clone() else {
            return Ok(false);
        };
        let ExprKind::ArrayLit(elements) = value.kind() else {
            return Ok(false);
        };
        if elements.is_empty() {
            return Ok(false);
        }
        let element_ty = self.resolve_type(&element);
        let mut emitted = Vec::with_capacity(elements.len());
        for expression in elements {
            let element_value = self.emit_expr_as(expression, &element_ty)?;
            if !constant_basic_value(element_value) {
                return Ok(false);
            }
            emitted.push(element_value);
        }
        let data = match self.llvm_type(&element_ty) {
            BasicTypeEnum::IntType(int_type) => {
                let values: Vec<_> = emitted.iter().map(|v| v.into_int_value()).collect();
                int_type.const_array(&values)
            }
            BasicTypeEnum::FloatType(float_type) => {
                let values: Vec<_> = emitted.iter().map(|v| v.into_float_value()).collect();
                float_type.const_array(&values)
            }
            BasicTypeEnum::StructType(struct_type) => {
                let values: Vec<_> = emitted.iter().map(|v| v.into_struct_value()).collect();
                struct_type.const_array(&values)
            }
            _ => return Ok(false),
        };
        let data_global = self
            .module
            .add_global(data.get_type(), None, &format!("{name}.data"));
        data_global.set_initializer(&data);
        data_global.set_constant(true);
        data_global.set_linkage(inkwell::module::Linkage::Private);
        data_global.set_unnamed_addr(true);
        let header = self.llvm_type(ty).into_struct_type();
        let length = self
            .context
            .i64_type()
            .const_int(elements.len() as u64, false);
        let initializer = header.const_named_struct(&[
            data_global.as_pointer_value().into(),
            length.into(),
            length.into(),
        ]);
        global.set_initializer(&initializer);
        global.set_constant(true);
        Ok(true)
    }

    pub(super) fn ensure_main_and_emit_global_var(&mut self, stmt: &Stmt) -> Result<(), String> {
        let Stmt {
            kind: StmtKind::VarDecl {
                name, ty, value, ..
            },
            ..
        } = stmt
        else {
            unreachable!()
        };
        self.ensure_main_block()?;
        let global = self
            .module
            .get_global(name)
            .ok_or_else(|| format!("internal: global `{name}` was not forward-declared"))?;
        if self.constant_global_arrays.contains(stmt.span)
            && let Some(value) = value
        {
            let resolved_ty = self.resolve_binding_type(ty, Some(value))?;
            if self.emit_constant_global_array(name, value, &resolved_ty, global)? {
                self.discard_owned_native_result();
                self.remember_main_continuation();
                return Ok(());
            }
        }
        if let Some(value) = value {
            let resolved_ty = self.resolve_binding_type(ty, Some(value))?;
            let emitted = self.emit_expr_as(value, &resolved_ty)?;
            self.builder
                .build_store(global.as_pointer_value(), emitted)
                .unwrap();
            self.transfer_owned_native_result(name, emitted);
            if matches!(value.kind(), ExprKind::ArrayLit(_))
                && matches!(transparent_codegen_type(&resolved_ty), Type::Array(element) if element.is_copy())
            {
                self.mark_owned_synchronous_value(name);
            }
            self.discard_owned_native_result();
        }
        self.remember_main_continuation();
        Ok(())
    }

    pub(super) fn finish_main(&mut self) {
        let Some(main) = self.module.get_function("main") else {
            return;
        };
        let block = self
            .builder
            .get_insert_block()
            .filter(|block| block.get_parent() == Some(main))
            .or_else(|| self.main_continuation(main));
        let Some(block) = block else {
            return;
        };
        let terminator = block.get_terminator();
        if terminator.is_some_and(|instruction| {
            instruction.get_opcode() != inkwell::values::InstructionOpcode::Return
        }) {
            return;
        }
        if let Some(terminator) = terminator {
            self.builder.position_before(&terminator);
        } else {
            self.builder.position_at_end(block);
        }
        self.emit_owned_native_cleanups_in_scope(0, None);
        self.emit_runtime_shutdown();
        if terminator.is_none() {
            self.builder
                .build_return(Some(&self.context.i32_type().const_zero()))
                .unwrap();
        }
    }
}

fn constant_basic_value(value: BasicValueEnum<'_>) -> bool {
    match value {
        BasicValueEnum::IntValue(v) => v.is_const(),
        BasicValueEnum::FloatValue(v) => v.is_const(),
        BasicValueEnum::StructValue(v) => v.is_const(),
        BasicValueEnum::PointerValue(v) => v.is_const(),
        BasicValueEnum::ArrayValue(v) => v.is_const(),
        BasicValueEnum::VectorValue(v) => v.is_const(),
        BasicValueEnum::ScalableVectorValue(v) => v.is_const(),
    }
}

fn first_value_return(body: &[Stmt]) -> Option<&Expr> {
    fn in_expr(expr: &Expr) -> Option<&Expr> {
        match expr.kind() {
            ExprKind::If {
                then_block,
                else_block,
                ..
            } => first_value_return(then_block)
                .or_else(|| else_block.as_deref().and_then(first_value_return)),
            _ => None,
        }
    }

    body.iter().find_map(|stmt| match stmt {
        Stmt {
            kind: StmtKind::Return(value),
            ..
        } => value.as_ref(),
        Stmt {
            kind: StmtKind::Expr(expr),
            ..
        } => in_expr(expr),
        Stmt {
            kind: StmtKind::While { body, .. },
            ..
        }
        | Stmt {
            kind: StmtKind::DoWhile { body, .. },
            ..
        } => first_value_return(body),
        Stmt {
            kind: StmtKind::For { body, .. },
            ..
        }
        | Stmt {
            kind: StmtKind::ForOf { body, .. },
            ..
        }
        | Stmt {
            kind: StmtKind::ForIn { body, .. },
            ..
        } => first_value_return(body),
        Stmt {
            kind:
                StmtKind::TryCatch {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                },
            ..
        } => first_value_return(try_body)
            .or_else(|| first_value_return(catch_body))
            .or_else(|| finally_body.as_deref().and_then(first_value_return)),
        Stmt {
            kind: StmtKind::Switch { cases, default, .. },
            ..
        } => cases
            .iter()
            .find_map(|case| first_value_return(&case.body))
            .or_else(|| default.as_deref().and_then(first_value_return)),
        Stmt {
            kind: StmtKind::Match { arms, default, .. },
            ..
        } => arms
            .iter()
            .find_map(|arm| first_value_return(&arm.body))
            .or_else(|| default.as_deref().and_then(first_value_return)),
        _ => None,
    })
}

#[allow(deprecated)]
fn basic_type_to_ptr_type(ty: BasicTypeEnum<'_>) -> PointerType<'_> {
    use inkwell::AddressSpace;
    match ty {
        BasicTypeEnum::PointerType(t) => t,
        BasicTypeEnum::IntType(t) => t.ptr_type(AddressSpace::default()),
        BasicTypeEnum::FloatType(t) => t.ptr_type(AddressSpace::default()),
        BasicTypeEnum::StructType(t) => t.ptr_type(AddressSpace::default()),
        BasicTypeEnum::ArrayType(t) => t.ptr_type(AddressSpace::default()),
        BasicTypeEnum::VectorType(t) => t.ptr_type(AddressSpace::default()),
        BasicTypeEnum::ScalableVectorType(t) => t.ptr_type(AddressSpace::default()),
    }
}

fn align_to(value: usize, alignment: usize) -> usize {
    if alignment <= 1 {
        value
    } else {
        value.div_ceil(alignment) * alignment
    }
}

fn merged_union_fields(variants: &[Type]) -> Vec<Field> {
    let mut fields = Vec::new();
    for variant in variants {
        let Type::Object(variant_fields) = variant else {
            continue;
        };
        for field in variant_fields {
            if !fields
                .iter()
                .any(|candidate: &Field| candidate.name == field.name)
            {
                fields.push(field.clone());
            }
        }
    }
    fields
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ReturnOwnership {
    Owned,
    Borrowed,
    Forwarded,
}

fn returns_owned_container_analysis(
    params: &[Param],
    body: &[Stmt],
    owned_returning_fns: &HashSet<String>,
    conditionally_owned_returning_fns: &HashSet<String>,
) -> Option<Vec<ReturnOwnership>> {
    fn allocates(expr: &Expr, owned_returning_fns: &HashSet<String>) -> bool {
        matches!(
            expr.kind(),
            ExprKind::NewMap { .. } | ExprKind::NewSet { .. }
        ) || matches!(expr.kind(), ExprKind::FnCall { callee, .. } if owned_returning_fns.contains(callee))
            || matches!(expr.kind(), ExprKind::FnCall { callee, .. } if tyrus_core::algorithm::is_collection_constructor_target(callee))
    }

    fn forwards_conditional(
        expr: &Expr,
        conditionally_owned_returning_fns: &HashSet<String>,
    ) -> bool {
        matches!(expr.kind(), ExprKind::FnCall { callee, .. } if conditionally_owned_returning_fns.contains(callee))
    }

    fn shadows_param(params: &[Param], name: &str) -> bool {
        params
            .iter()
            .any(|param| matches!(&param.pattern, ParamPattern::Name(p) if p == name))
    }

    fn walk<'a>(
        body: &'a [Stmt],
        params: &[Param],
        owned_returning_fns: &HashSet<String>,
        conditionally_owned_returning_fns: &HashSet<String>,
        mut owned: HashSet<&'a str>,
        mut forwarded: HashSet<&'a str>,
        bits: &mut Vec<ReturnOwnership>,
    ) -> bool {
        for stmt in body {
            match stmt.kind() {
                StmtKind::Return(expr) => bits.push(match expr {
                    Some(expr) => match expr.kind() {
                        ExprKind::Ident(name) if owned.contains(name.as_str()) => {
                            ReturnOwnership::Owned
                        }
                        ExprKind::Ident(name) if forwarded.contains(name.as_str()) => {
                            ReturnOwnership::Forwarded
                        }
                        ExprKind::Ident(_) => ReturnOwnership::Borrowed,
                        _ if forwards_conditional(expr, conditionally_owned_returning_fns) => {
                            ReturnOwnership::Forwarded
                        }
                        _ if allocates(expr, owned_returning_fns) => ReturnOwnership::Owned,
                        _ => ReturnOwnership::Borrowed,
                    },
                    None => ReturnOwnership::Borrowed,
                }),
                StmtKind::VarDecl {
                    mutable: false,
                    name,
                    value: Some(value),
                    ..
                } => {
                    if allocates(value, owned_returning_fns) && !shadows_param(params, name) {
                        owned.insert(name.as_str());
                        forwarded.remove(name.as_str());
                    } else if forwards_conditional(value, conditionally_owned_returning_fns)
                        && !shadows_param(params, name)
                    {
                        forwarded.insert(name.as_str());
                        owned.remove(name.as_str());
                    } else {
                        owned.remove(name.as_str());
                        forwarded.remove(name.as_str());
                    }
                }
                StmtKind::VarDecl { .. }
                | StmtKind::Assign { .. }
                | StmtKind::IndexAssign { .. }
                | StmtKind::FieldAssign { .. }
                | StmtKind::ArrayDestructure { .. }
                | StmtKind::ObjectDestructure { .. } => {}
                StmtKind::Expr(expr) => match expr.kind() {
                    ExprKind::MethodCall { .. } | ExprKind::FnCall { .. } => {}
                    ExprKind::If {
                        then_block,
                        else_block,
                        ..
                    } => {
                        if !walk(
                            then_block,
                            params,
                            owned_returning_fns,
                            conditionally_owned_returning_fns,
                            owned.clone(),
                            forwarded.clone(),
                            bits,
                        ) {
                            return false;
                        }
                        if let Some(else_block) = else_block
                            && !walk(
                                else_block,
                                params,
                                owned_returning_fns,
                                conditionally_owned_returning_fns,
                                owned.clone(),
                                forwarded.clone(),
                                bits,
                            )
                        {
                            return false;
                        }
                    }
                    _ => return false,
                },
                _ => return false,
            }
        }
        true
    }

    let mut bits = Vec::new();
    walk(
        body,
        params,
        owned_returning_fns,
        conditionally_owned_returning_fns,
        HashSet::default(),
        HashSet::default(),
        &mut bits,
    )
    .then_some(bits)
}

fn db_async_returning_functions(items: &[Item]) -> HashSet<String> {
    fn collect_returns<'a>(statements: &'a [Stmt], returns: &mut Vec<&'a Expr>) {
        for statement in statements {
            match statement.kind() {
                StmtKind::Return(Some(expression)) => returns.push(expression),
                StmtKind::TryCatch {
                    try_body,
                    catch_body,
                    finally_body,
                    ..
                } => {
                    collect_returns(try_body, returns);
                    collect_returns(catch_body, returns);
                    if let Some(finally_body) = finally_body {
                        collect_returns(finally_body, returns);
                    }
                }
                StmtKind::While { body, .. }
                | StmtKind::DoWhile { body, .. }
                | StmtKind::ForOf { body, .. }
                | StmtKind::ForIn { body, .. }
                | StmtKind::Defer { body, .. } => collect_returns(body, returns),
                StmtKind::For { body, .. } => collect_returns(body, returns),
                StmtKind::Switch { cases, default, .. } => {
                    for case in cases {
                        collect_returns(&case.body, returns);
                    }
                    if let Some(default) = default {
                        collect_returns(default, returns);
                    }
                }
                StmtKind::Match { arms, default, .. } => {
                    for arm in arms {
                        collect_returns(&arm.body, returns);
                    }
                    if let Some(default) = default {
                        collect_returns(default, returns);
                    }
                }
                _ => {}
            }
        }
    }

    fn is_db_async_call(expression: &Expr, forwarded: &HashSet<String>) -> bool {
        match expression.kind() {
            ExprKind::MethodCall { method, .. } => matches!(
                method.as_str(),
                "query"
                    | "findMany"
                    | "findOne"
                    | "count"
                    | "insertOne"
                    | "insertMany"
                    | "updateOne"
                    | "updateMany"
                    | "deleteOne"
                    | "deleteMany"
            ),
            ExprKind::FnCall { callee, .. } => {
                forwarded.contains(callee)
                    || forwarded.contains(tyrus_core::module_loader::original_global_name(callee))
                    || matches!(
                        callee.as_str(),
                        tyrus_core::db::QUERY_TARGET
                            | tyrus_core::db::FIND_MANY_TARGET
                            | tyrus_core::db::FIND_ONE_TARGET
                            | tyrus_core::db::COUNT_TARGET
                            | tyrus_core::db::INSERT_ONE_TARGET
                            | tyrus_core::db::INSERT_MANY_TARGET
                            | tyrus_core::db::UPDATE_ONE_TARGET
                            | tyrus_core::db::UPDATE_MANY_TARGET
                            | tyrus_core::db::DELETE_ONE_TARGET
                            | tyrus_core::db::DELETE_MANY_TARGET
                            | tyrus_core::db::BEGIN_TRANSACTION_TARGET
                            | tyrus_core::db::COMMIT_TRANSACTION_TARGET
                            | tyrus_core::db::ROLLBACK_TRANSACTION_TARGET
                    )
            }
            _ => false,
        }
    }

    let functions: Vec<_> = items
        .iter()
        .filter_map(|item| match item {
            Item::FnDecl { name, body, .. } => Some((name, body)),
            Item::Stmt(statement) => match statement.kind() {
                StmtKind::VarDecl {
                    name,
                    value: Some(expression),
                    ..
                } => match expression.kind() {
                    ExprKind::ArrowFn { body, .. } => Some((name, body)),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
        .collect();
    let mut forwarded = HashSet::default();
    loop {
        let previous_len = forwarded.len();
        for (name, body) in &functions {
            let mut returns = Vec::new();
            collect_returns(body, &mut returns);
            if !returns.is_empty()
                && returns
                    .iter()
                    .all(|expression| is_db_async_call(expression, &forwarded))
            {
                forwarded.insert((*name).clone());
            }
        }
        if forwarded.len() == previous_len {
            return forwarded;
        }
    }
}

fn validate_async_codegen_support(program: &Program) -> Result<(), String> {
    for item in &program.items {
        validate_codegen_item(item)?;
    }
    Ok(())
}

fn validate_async_codegen_stmts(stmts: &[Stmt]) -> Result<(), String> {
    let _ = stmts;
    Ok(())
}

pub(crate) fn promise_payload_type(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Apply { name, args }
            if matches!(name.as_str(), "Promise" | "DbPromise") && args.len() == 1 =>
        {
            args.first()
        }
        _ => None,
    }
}

pub(crate) fn join_handle_payload(ty: &Type) -> Option<&Type> {
    match ty {
        Type::Apply { name, args } if name == "JoinHandle" && args.len() == 1 => args.first(),
        _ => None,
    }
}

fn simple_async_statement_expression(statement: &Stmt) -> Option<&Expr> {
    match statement.kind() {
        StmtKind::VarDecl { value, .. } => value.as_ref(),
        StmtKind::Assign { value, .. }
        | StmtKind::Throw(value)
        | StmtKind::Expr(value)
        | StmtKind::ArrayDestructure { value, .. }
        | StmtKind::ObjectDestructure { value, .. } => Some(value),
        StmtKind::IndexAssign { .. } => None,
        StmtKind::Return(value) => value.as_ref(),
        _ => None,
    }
}

fn simple_async_statement_expression_mut(statement: &mut Stmt) -> Option<&mut Expr> {
    match &mut statement.kind {
        StmtKind::VarDecl { value, .. } => value.as_mut(),
        StmtKind::Assign { value, .. }
        | StmtKind::Throw(value)
        | StmtKind::Expr(value)
        | StmtKind::ArrayDestructure { value, .. }
        | StmtKind::ObjectDestructure { value, .. } => Some(value),
        StmtKind::IndexAssign { .. } => None,
        StmtKind::Return(value) => value.as_mut(),
        _ => None,
    }
}

fn assign_async_expression_ids(expression: &mut Expr, outer_index: usize, next: &mut usize) {
    *next += 1;
    expression.span = Span::new(
        usize::MAX,
        outer_index.wrapping_mul(65_537).wrapping_add(*next),
    );
    match &mut expression.kind {
        ExprKind::Await(dependency) => assign_async_expression_ids(dependency, outer_index, next),
        ExprKind::NewSet { initial, .. } | ExprKind::NewMap { initial, .. } => {
            if let Some(initial) = initial {
                assign_async_expression_ids(initial, outer_index, next);
            }
        }
        ExprKind::TemplateLit(parts) => {
            for part in parts {
                if let TemplatePart::Expr(expression, _) = part {
                    assign_async_expression_ids(expression, outer_index, next);
                }
            }
        }
        ExprKind::ObjectLit(fields) => {
            for (_, value) in fields {
                assign_async_expression_ids(value, outer_index, next);
            }
        }
        ExprKind::ObjectUpdate { base, fields } => {
            assign_async_expression_ids(base, outer_index, next);
            for (_, value) in fields {
                assign_async_expression_ids(value, outer_index, next);
            }
        }
        ExprKind::ArrayLit(values) => {
            for value in values {
                assign_async_expression_ids(value, outer_index, next);
            }
        }
        ExprKind::ArrayRepeat { value, count } => {
            assign_async_expression_ids(value, outer_index, next);
            assign_async_expression_ids(count, outer_index, next);
        }
        ExprKind::FieldAccess { object, .. } | ExprKind::OptionalFieldAccess { object, .. } => {
            assign_async_expression_ids(object, outer_index, next);
        }
        ExprKind::IndexAccess { object, index } => {
            assign_async_expression_ids(object, outer_index, next);
            assign_async_expression_ids(index, outer_index, next);
        }
        ExprKind::BinOp { left, right, .. }
        | ExprKind::Pipeline { left, right }
        | ExprKind::In {
            key: left,
            object: right,
        } => {
            assign_async_expression_ids(left, outer_index, next);
            assign_async_expression_ids(right, outer_index, next);
        }
        ExprKind::Range { start, end, .. } => {
            assign_async_expression_ids(start, outer_index, next);
            assign_async_expression_ids(end, outer_index, next);
        }
        ExprKind::ChainedComparison { terms, .. } => {
            for term in terms {
                assign_async_expression_ids(term, outer_index, next);
            }
        }
        ExprKind::Not(inner)
        | ExprKind::Neg(inner)
        | ExprKind::BitNot(inner)
        | ExprKind::PrefixIncr(inner)
        | ExprKind::PrefixDecr(inner)
        | ExprKind::PostfixIncr(inner)
        | ExprKind::PostfixDecr(inner)
        | ExprKind::Unwrap(inner)
        | ExprKind::Typeof(inner)
        | ExprKind::Try(inner)
        | ExprKind::Spread(inner)
        | ExprKind::TypeAssert { value: inner, .. }
        | ExprKind::ConstAssert(inner)
        | ExprKind::Comptime(inner) => assign_async_expression_ids(inner, outer_index, next),
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } => {
            assign_async_expression_ids(cond, outer_index, next);
            assign_async_expression_ids(then_expr, outer_index, next);
            assign_async_expression_ids(else_expr, outer_index, next);
        }
        ExprKind::OptionalMethodCall { object, args, .. }
        | ExprKind::MethodCall { object, args, .. } => {
            assign_async_expression_ids(object, outer_index, next);
            for argument in args {
                assign_async_expression_ids(argument, outer_index, next);
            }
        }
        ExprKind::FnCall { args, .. } => {
            for argument in args {
                assign_async_expression_ids(argument, outer_index, next);
            }
        }
        ExprKind::CallValue { callee, args, .. } => {
            assign_async_expression_ids(callee, outer_index, next);
            for argument in args {
                assign_async_expression_ids(argument, outer_index, next);
            }
        }
        ExprKind::If { cond, .. } => assign_async_expression_ids(cond, outer_index, next),
        ExprKind::Loop { .. } => {}
        ExprKind::ArrowFn { .. }
        | ExprKind::StrLit(_)
        | ExprKind::RegexLit { .. }
        | ExprKind::IntLit(_)
        | ExprKind::UIntLit(_)
        | ExprKind::FloatLit(_)
        | ExprKind::DecimalLit(_)
        | ExprKind::BoolLit(_)
        | ExprKind::Null
        | ExprKind::None
        | ExprKind::Ident(_)
        | ExprKind::Topic
        | ExprKind::Borrow { .. }
        | ExprKind::Is { .. } => {}
    }
}

fn first_await_in_simple_async_statement(statement: &Stmt) -> Option<&Expr> {
    simple_async_statement_expression(statement).and_then(first_await_expression)
}

pub(crate) fn first_await_in_block(body: &[Stmt]) -> Option<&Expr> {
    let mut statements = Vec::new();
    collect_nested_async_block(body, &mut statements);
    statements
        .into_iter()
        .find_map(first_await_in_simple_async_statement)
}

fn collect_nested_async_statements<'a>(statement: &'a Stmt, statements: &mut Vec<&'a Stmt>) {
    match statement.kind() {
        StmtKind::TryCatch {
            try_body,
            catch_body,
            finally_body,
            ..
        } => {
            collect_nested_async_block(try_body, statements);
            collect_nested_async_block(catch_body, statements);
            if let Some(finally_body) = finally_body {
                collect_nested_async_block(finally_body, statements);
            }
        }
        StmtKind::While { body, .. }
        | StmtKind::DoWhile { body, .. }
        | StmtKind::ForOf { body, .. }
        | StmtKind::ForIn { body, .. } => collect_nested_async_block(body, statements),
        StmtKind::For {
            init, update, body, ..
        } => {
            statements.push(init);
            collect_nested_async_statements(init, statements);
            collect_nested_async_block(body, statements);
            statements.push(update);
            collect_nested_async_statements(update, statements);
        }
        StmtKind::Switch { cases, default, .. } => {
            for case in cases {
                collect_nested_async_block(&case.body, statements);
            }
            if let Some(default) = default {
                collect_nested_async_block(default, statements);
            }
        }
        StmtKind::Match { arms, default, .. } => {
            for arm in arms {
                collect_nested_async_block(&arm.body, statements);
            }
            if let Some(default) = default {
                collect_nested_async_block(default, statements);
            }
        }
        StmtKind::Expr(expression) => {
            if let ExprKind::If {
                then_block,
                else_block,
                ..
            } = expression.kind()
            {
                collect_nested_async_block(then_block, statements);
                if let Some(else_block) = else_block {
                    collect_nested_async_block(else_block, statements);
                }
            }
        }
        StmtKind::VarDecl { .. }
        | StmtKind::ArrayDestructure { .. }
        | StmtKind::ObjectDestructure { .. }
        | StmtKind::Assign { .. }
        | StmtKind::IndexAssign { .. }
        | StmtKind::FieldAssign { .. }
        | StmtKind::Return(_)
        | StmtKind::Break(_)
        | StmtKind::Continue
        | StmtKind::Throw(_)
        | StmtKind::Defer { .. } => {}
    }
}

fn collect_nested_async_block<'a>(body: &'a [Stmt], statements: &mut Vec<&'a Stmt>) {
    for nested in body {
        statements.push(nested);
        collect_nested_async_statements(nested, statements);
    }
}

fn direct_awaits_in_block(body: &[Stmt]) -> Option<Vec<(usize, Expr, AsyncNestedAwaitAction)>> {
    let direct = body
        .iter()
        .enumerate()
        .filter_map(|(index, statement)| match statement.kind() {
            StmtKind::Expr(expression) => match expression.kind() {
                ExprKind::Await(dependency) if first_await_expression(dependency).is_none() => {
                    Some((
                        index,
                        dependency.as_ref().clone(),
                        AsyncNestedAwaitAction::Discard,
                    ))
                }
                _ => None,
            },
            StmtKind::VarDecl {
                name,
                ty,
                value: Some(value),
                ..
            } => match value.kind() {
                ExprKind::Await(dependency) if first_await_expression(dependency).is_none() => {
                    Some((
                        index,
                        dependency.as_ref().clone(),
                        AsyncNestedAwaitAction::Binding {
                            name: name.clone(),
                            ty: ty.clone(),
                        },
                    ))
                }
                _ => None,
            },
            StmtKind::Assign {
                name,
                op: AssignOp::Assign,
                value,
            } => match value.kind() {
                ExprKind::Await(dependency) if first_await_expression(dependency).is_none() => {
                    Some((
                        index,
                        dependency.as_ref().clone(),
                        AsyncNestedAwaitAction::Assignment { name: name.clone() },
                    ))
                }
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    if body.iter().any(|statement| {
            !matches!(statement.kind(), StmtKind::Expr(expression) if matches!(expression.kind(), ExprKind::Await(_)))
                && !matches!(statement.kind(), StmtKind::VarDecl { value: Some(value), .. } if matches!(value.kind(), ExprKind::Await(_)))
                && !matches!(statement.kind(), StmtKind::Assign { op: AssignOp::Assign, value, .. } if matches!(value.kind(), ExprKind::Await(_)))
                && first_await_in_block(std::slice::from_ref(statement)).is_some()
        })
    {
        None
    } else {
        Some(direct)
    }
}

struct AsyncSwitchAwaitSlice {
    await_clause: usize,
    dependency: Expr,
    previous: Vec<Stmt>,
    before: Vec<Stmt>,
    after: Vec<Stmt>,
    action: AsyncNestedAwaitAction,
    first_in_clause: bool,
    final_in_clause: bool,
    entry_guard: Option<AsyncSwitchClauseGuard>,
}

#[derive(Clone)]
struct AsyncSwitchClauseGuard {
    condition: Expr,
    schedule_when_true: bool,
    outer_before: Vec<Stmt>,
    selected_before: Vec<Stmt>,
    bypass: Vec<Stmt>,
    repeat: bool,
    enter_before_condition: bool,
    repeat_update: Option<Box<Stmt>>,
    for_of: Option<AsyncSwitchClauseForOf>,
}

#[derive(Clone)]
struct AsyncSwitchClauseForOf {
    name: String,
    iterable: Expr,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum AsyncForOfSlotKey {
    Statement(usize),
    RecursiveControl(usize, usize),
    SwitchContext(usize),
    SwitchClause(usize, usize),
}

fn direct_awaits_in_switch(
    cases: &[SwitchCase],
    default: &Option<Vec<Stmt>>,
    default_index: Option<usize>,
) -> Option<Vec<AsyncSwitchAwaitSlice>> {
    let mut clauses = cases
        .iter()
        .map(|case| case.body.as_slice())
        .collect::<Vec<_>>();
    if let Some(default_body) = default {
        clauses.insert(default_index?, default_body);
    }
    let mut result = Vec::new();
    for (clause_index, body) in clauses.into_iter().enumerate() {
        let direct = direct_awaits_in_block(body);
        let (awaits, entry_guard, await_body, outer_after) = match direct {
            Some(awaits) if !awaits.is_empty() || first_await_in_block(body).is_none() => {
                (awaits, None, body.to_vec(), Vec::new())
            }
            _ => nested_if_awaits_in_switch_clause(body)
                .or_else(|| nested_while_awaits_in_switch_clause(body))
                .or_else(|| nested_do_while_awaits_in_switch_clause(body))
                .or_else(|| nested_for_awaits_in_switch_clause(body))
                .or_else(|| nested_for_of_awaits_in_switch_clause(body))?,
        };
        for (site_index, (await_index, dependency, action)) in awaits.iter().enumerate() {
            let start = site_index
                .checked_sub(1)
                .map_or(0, |previous| awaits[previous].0 + 1);
            let final_in_clause = site_index + 1 == awaits.len();
            result.push(AsyncSwitchAwaitSlice {
                await_clause: clause_index,
                dependency: dependency.clone(),
                previous: {
                    let mut previous = entry_guard
                        .as_ref()
                        .map(|guard| guard.outer_before.clone())
                        .unwrap_or_default();
                    previous.extend_from_slice(&await_body[..start]);
                    previous
                },
                before: if site_index == 0 {
                    entry_guard.as_ref().map_or_else(
                        || await_body[start..*await_index].to_vec(),
                        |guard| guard.outer_before.clone(),
                    )
                } else {
                    await_body[start..*await_index].to_vec()
                },
                after: final_in_clause
                    .then(|| {
                        let mut after = await_body[await_index + 1..].to_vec();
                        after.extend(outer_after.clone());
                        after
                    })
                    .unwrap_or_default(),
                action: action.clone(),
                first_in_clause: site_index == 0,
                final_in_clause,
                entry_guard: entry_guard.clone(),
            });
        }
    }
    (!result.is_empty()).then_some(result)
}

type DirectAsyncAwait = (usize, Expr, AsyncNestedAwaitAction);
type AsyncSwitchNestedAwaitShape = (
    Vec<DirectAsyncAwait>,
    Option<AsyncSwitchClauseGuard>,
    Vec<Stmt>,
    Vec<Stmt>,
);

fn nested_if_awaits_in_switch_clause(body: &[Stmt]) -> Option<AsyncSwitchNestedAwaitShape> {
    let (if_index, condition, then_block, else_block) =
        body.iter()
            .enumerate()
            .find_map(|(index, statement)| match statement.kind() {
                StmtKind::Expr(Expr {
                    kind:
                        ExprKind::If {
                            cond,
                            then_block,
                            else_block,
                        },
                    ..
                }) if first_await_in_block(std::slice::from_ref(statement)).is_some() => Some((
                    index,
                    cond.as_ref().clone(),
                    then_block.as_slice(),
                    else_block.as_deref().unwrap_or_default(),
                )),
                _ => None,
            })?;
    if body[..if_index]
        .iter()
        .chain(&body[if_index + 1..])
        .any(|statement| first_await_in_block(std::slice::from_ref(statement)).is_some())
    {
        return None;
    }
    let then_awaits = direct_awaits_in_block(then_block)?;
    let else_awaits = direct_awaits_in_block(else_block)?;
    let (selected, bypass, schedule_when_true, awaits) =
        match (then_awaits.is_empty(), else_awaits.is_empty()) {
            (false, true) => (then_block, else_block, true, then_awaits),
            (true, false) => (else_block, then_block, false, else_awaits),
            (false, false)
                if then_awaits.len() == 1
                    && else_awaits.len() == 1
                    && then_awaits[0].0 == 0
                    && else_awaits[0].0 == 0
                    && then_block.len() == 1
                    && else_block.len() == 1
                    && matches!(then_awaits[0].2, AsyncNestedAwaitAction::Discard)
                    && matches!(else_awaits[0].2, AsyncNestedAwaitAction::Discard) =>
            {
                let dependency = Expr::synthetic(ExprKind::Ternary {
                    cond: Box::new(condition.clone()),
                    then_expr: Box::new(then_awaits[0].1.clone()),
                    else_expr: Box::new(else_awaits[0].1.clone()),
                });
                let guard = AsyncSwitchClauseGuard {
                    condition: Expr::synthetic(ExprKind::BoolLit(true)),
                    schedule_when_true: true,
                    outer_before: body[..if_index].to_vec(),
                    selected_before: Vec::new(),
                    bypass: body[if_index + 1..].to_vec(),
                    repeat: false,
                    enter_before_condition: false,
                    repeat_update: None,
                    for_of: None,
                };
                return Some((
                    vec![(0, dependency, AsyncNestedAwaitAction::Discard)],
                    Some(guard),
                    then_block.to_vec(),
                    body[if_index + 1..].to_vec(),
                ));
            }
            _ => return None,
        };
    let first_await = awaits.first()?.0;
    let mut bypass_statements = bypass.to_vec();
    bypass_statements.extend_from_slice(&body[if_index + 1..]);
    let guard = AsyncSwitchClauseGuard {
        condition,
        schedule_when_true,
        outer_before: body[..if_index].to_vec(),
        selected_before: selected[..first_await].to_vec(),
        bypass: bypass_statements,
        repeat: false,
        enter_before_condition: false,
        repeat_update: None,
        for_of: None,
    };
    Some((
        awaits,
        Some(guard),
        selected.to_vec(),
        body[if_index + 1..].to_vec(),
    ))
}

fn nested_while_awaits_in_switch_clause(body: &[Stmt]) -> Option<AsyncSwitchNestedAwaitShape> {
    let (loop_index, condition, loop_body) =
        body.iter()
            .enumerate()
            .find_map(|(index, statement)| match statement.kind() {
                StmtKind::While {
                    cond,
                    body: loop_body,
                } if first_await_in_block(loop_body).is_some() => {
                    Some((index, cond.clone(), loop_body.as_slice()))
                }
                _ => None,
            })?;
    if body[..loop_index]
        .iter()
        .chain(&body[loop_index + 1..])
        .any(|statement| first_await_in_block(std::slice::from_ref(statement)).is_some())
    {
        return None;
    }
    let awaits = direct_awaits_in_block(loop_body)?
        .into_iter()
        .collect::<Vec<_>>();
    let first_await = awaits.first()?.0;
    let guard = AsyncSwitchClauseGuard {
        condition,
        schedule_when_true: true,
        outer_before: body[..loop_index].to_vec(),
        selected_before: loop_body[..first_await].to_vec(),
        bypass: body[loop_index + 1..].to_vec(),
        repeat: true,
        enter_before_condition: false,
        repeat_update: None,
        for_of: None,
    };
    Some((awaits, Some(guard), loop_body.to_vec(), Vec::new()))
}

fn nested_do_while_awaits_in_switch_clause(body: &[Stmt]) -> Option<AsyncSwitchNestedAwaitShape> {
    let (loop_index, loop_body, condition) =
        body.iter()
            .enumerate()
            .find_map(|(index, statement)| match statement.kind() {
                StmtKind::DoWhile {
                    body: loop_body,
                    cond,
                } if first_await_in_block(loop_body).is_some() => {
                    Some((index, loop_body.as_slice(), cond.clone()))
                }
                _ => None,
            })?;
    if body[..loop_index]
        .iter()
        .chain(&body[loop_index + 1..])
        .any(|statement| first_await_in_block(std::slice::from_ref(statement)).is_some())
    {
        return None;
    }
    let awaits = direct_awaits_in_block(loop_body)?
        .into_iter()
        .collect::<Vec<_>>();
    let first_await = awaits.first()?.0;
    let guard = AsyncSwitchClauseGuard {
        condition,
        schedule_when_true: true,
        outer_before: body[..loop_index].to_vec(),
        selected_before: loop_body[..first_await].to_vec(),
        bypass: body[loop_index + 1..].to_vec(),
        repeat: true,
        enter_before_condition: true,
        repeat_update: None,
        for_of: None,
    };
    Some((awaits, Some(guard), loop_body.to_vec(), Vec::new()))
}

fn nested_for_awaits_in_switch_clause(body: &[Stmt]) -> Option<AsyncSwitchNestedAwaitShape> {
    let (loop_index, init, condition, update, loop_body) =
        body.iter()
            .enumerate()
            .find_map(|(index, statement)| match statement.kind() {
                StmtKind::For {
                    init,
                    cond,
                    update,
                    body: loop_body,
                } if first_await_in_block(loop_body).is_some() => Some((
                    index,
                    init.as_ref().clone(),
                    cond.clone(),
                    update.as_ref().clone(),
                    loop_body.as_slice(),
                )),
                _ => None,
            })?;
    if first_await_in_block(std::slice::from_ref(&init)).is_some()
        || first_await_expression(&condition).is_some()
        || first_await_in_block(std::slice::from_ref(&update)).is_some()
        || body[..loop_index]
            .iter()
            .chain(&body[loop_index + 1..])
            .any(|statement| first_await_in_block(std::slice::from_ref(statement)).is_some())
    {
        return None;
    }
    let awaits = direct_awaits_in_block(loop_body)?
        .into_iter()
        .collect::<Vec<_>>();
    let first_await = awaits.first()?.0;
    let mut outer_before = body[..loop_index].to_vec();
    outer_before.push(init);
    let guard = AsyncSwitchClauseGuard {
        condition,
        schedule_when_true: true,
        outer_before,
        selected_before: loop_body[..first_await].to_vec(),
        bypass: body[loop_index + 1..].to_vec(),
        repeat: true,
        enter_before_condition: false,
        repeat_update: Some(Box::new(update)),
        for_of: None,
    };
    Some((awaits, Some(guard), loop_body.to_vec(), Vec::new()))
}

fn nested_for_of_awaits_in_switch_clause(body: &[Stmt]) -> Option<AsyncSwitchNestedAwaitShape> {
    let (loop_index, name, iterable, loop_body) =
        body.iter()
            .enumerate()
            .find_map(|(index, statement)| match statement.kind() {
                StmtKind::ForOf {
                    name,
                    iterable,
                    body: loop_body,
                    ..
                } if first_await_expression(iterable).is_none()
                    && matches!(iterable.kind(), ExprKind::Ident(_))
                    && first_await_in_block(loop_body).is_some() =>
                {
                    Some((index, name.clone(), iterable.clone(), loop_body.as_slice()))
                }
                _ => None,
            })?;
    if body[..loop_index]
        .iter()
        .chain(&body[loop_index + 1..])
        .any(|statement| first_await_in_block(std::slice::from_ref(statement)).is_some())
    {
        return None;
    }
    let awaits = direct_awaits_in_block(loop_body)?
        .into_iter()
        .collect::<Vec<_>>();
    let first_await = awaits.first()?.0;
    let guard = AsyncSwitchClauseGuard {
        condition: Expr::synthetic(ExprKind::BoolLit(true)),
        schedule_when_true: true,
        outer_before: body[..loop_index].to_vec(),
        selected_before: loop_body[..first_await].to_vec(),
        bypass: body[loop_index + 1..].to_vec(),
        repeat: true,
        enter_before_condition: false,
        repeat_update: None,
        for_of: Some(AsyncSwitchClauseForOf { name, iterable }),
    };
    Some((awaits, Some(guard), loop_body.to_vec(), Vec::new()))
}

fn block_contains_simple_async_await(body: &[Stmt]) -> bool {
    body.iter()
        .any(|statement| first_await_in_simple_async_statement(statement).is_some())
}

#[derive(Clone)]
struct AsyncConditionExpr {
    expression: Expr,
    span: tyrus_core::span::Span,
    payload: Type,
    guard: Option<Expr>,
}

#[derive(Clone)]
enum AsyncNestedAwaitAction {
    Discard,
    Binding { name: String, ty: Type },
    Assignment { name: String },
}

#[derive(Clone)]
enum AsyncSwitchContext {
    Recursive {
        contexts: Vec<AsyncControlContext>,
        enter_depth: usize,
        exit_depth: usize,
    },
    If {
        cond: Expr,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        other: Vec<Stmt>,
        switch_in_then: bool,
    },
    While {
        cond: Expr,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
    },
    DoWhile {
        cond: Expr,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
    },
    For {
        init: Box<Stmt>,
        cond: Expr,
        update: Box<Stmt>,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
    },
    ForOf {
        name: String,
        iterable: Expr,
        element_type: Type,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
    },
    Try {
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        catch_binding: String,
        catch_type: Type,
        catch_body: Vec<Stmt>,
        finally_body: Vec<Stmt>,
    },
}

fn async_await_kind_name(kind: &AsyncAwaitKind) -> &'static str {
    match kind {
        AsyncAwaitKind::Direct { .. } => "await",
        AsyncAwaitKind::Binding { .. } => "await binding",
        AsyncAwaitKind::Assignment { .. } => "await assignment",
        AsyncAwaitKind::IfCondition { .. } => "if condition",
        AsyncAwaitKind::WhileCondition { .. } => "while condition",
        AsyncAwaitKind::WhileBody { .. } => "while body",
        AsyncAwaitKind::DoWhileCondition { .. } => "do-while condition",
        AsyncAwaitKind::DoWhileBody { .. } => "do-while body",
        AsyncAwaitKind::ForCondition { .. } => "for condition",
        AsyncAwaitKind::ForBody { .. } => "for body",
        AsyncAwaitKind::ForOfBody { .. } => "for-of body",
        AsyncAwaitKind::ControlFlow { .. } => "nested control flow",
        AsyncAwaitKind::IfBranch { .. } => "if branch",
        AsyncAwaitKind::IfBothBranches { .. } => "if branches",
        AsyncAwaitKind::SwitchSubject { .. } => "switch subject",
        AsyncAwaitKind::SwitchCaseExpression { .. } => "switch case expression",
        AsyncAwaitKind::Switch { .. } => "switch body",
        AsyncAwaitKind::Expression { .. } => "await expression",
        AsyncAwaitKind::Try { .. } => "try body",
        AsyncAwaitKind::FinallyCompletion { .. } => "finally completion",
    }
}

fn conditional_guard_for_await(expression: &Expr, target: Span) -> Option<AsyncExpressionGuard> {
    match expression.kind() {
        ExprKind::BinOp { op, left, right }
            if expression_contains_await_span(right, target)
                && !expression_contains_await_span(left, target) =>
        {
            match op {
                BinOp::And => Some(AsyncExpressionGuard {
                    condition: left.as_ref().clone(),
                    kind: AsyncExpressionGuardKind::Boolean {
                        schedule_when_true: true,
                    },
                }),
                BinOp::Or => Some(AsyncExpressionGuard {
                    condition: left.as_ref().clone(),
                    kind: AsyncExpressionGuardKind::Boolean {
                        schedule_when_true: false,
                    },
                }),
                BinOp::NullCoalesce => Some(AsyncExpressionGuard {
                    condition: left.as_ref().clone(),
                    kind: AsyncExpressionGuardKind::Optional {
                        schedule_when_null: true,
                    },
                }),
                _ => conditional_guard_for_await(right, target),
            }
        }
        ExprKind::Ternary {
            cond,
            then_expr,
            else_expr,
        } if expression_contains_await_span(then_expr, target) => Some(AsyncExpressionGuard {
            condition: cond.as_ref().clone(),
            kind: AsyncExpressionGuardKind::Boolean {
                schedule_when_true: true,
            },
        }),
        ExprKind::Ternary {
            cond, else_expr, ..
        } if expression_contains_await_span(else_expr, target) => Some(AsyncExpressionGuard {
            condition: cond.as_ref().clone(),
            kind: AsyncExpressionGuardKind::Boolean {
                schedule_when_true: false,
            },
        }),
        ExprKind::OptionalMethodCall { object, args, .. }
            if !expression_contains_await_span(object, target)
                && args
                    .iter()
                    .any(|argument| expression_contains_await_span(argument, target)) =>
        {
            Some(AsyncExpressionGuard {
                condition: object.as_ref().clone(),
                kind: AsyncExpressionGuardKind::Optional {
                    schedule_when_null: false,
                },
            })
        }
        _ => async_expression_children(expression)
            .into_iter()
            .find(|child| expression_contains_await_span(child, target))
            .and_then(|child| conditional_guard_for_await(child, target)),
    }
}

#[derive(Clone)]
#[allow(
    clippy::large_enum_variant,
    reason = "boxing async suspension metadata would add allocations throughout code generation"
)]
enum AsyncAwaitKind {
    Direct {
        returns: bool,
    },
    Binding {
        name: String,
        ty: Type,
        onfail: Option<OnFail>,
        declared_ty: Option<Type>,
        value: Option<Expr>,
    },
    Assignment {
        name: String,
    },
    IfCondition {
        then_block: Vec<Stmt>,
        else_block: Vec<Stmt>,
        condition: Option<AsyncConditionExpr>,
    },
    WhileCondition {
        body: Vec<Stmt>,
        condition: Option<AsyncConditionExpr>,
    },
    WhileBody {
        cond: Expr,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        action: AsyncNestedAwaitAction,
        first_in_body: bool,
        final_in_body: bool,
    },
    DoWhileCondition {
        body: Vec<Stmt>,
        condition: Option<AsyncConditionExpr>,
    },
    DoWhileBody {
        cond: Expr,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        action: AsyncNestedAwaitAction,
        first_in_body: bool,
        final_in_body: bool,
    },
    ForCondition {
        init: Box<Stmt>,
        update: Box<Stmt>,
        body: Vec<Stmt>,
        condition: Option<AsyncConditionExpr>,
    },
    ForBody {
        init: Box<Stmt>,
        cond: Expr,
        update: Box<Stmt>,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        action: AsyncNestedAwaitAction,
        first_in_body: bool,
        final_in_body: bool,
    },
    ForOfBody {
        name: String,
        iterable: Expr,
        element_type: Type,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        action: AsyncNestedAwaitAction,
        first_in_body: bool,
        final_in_body: bool,
    },
    ControlFlow {
        contexts: Vec<AsyncControlContext>,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        action: AsyncNestedAwaitAction,
        enter_depth: usize,
        exit_depth: usize,
    },
    IfBranch {
        cond: Expr,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        other: Vec<Stmt>,
        await_in_then: bool,
        action: AsyncNestedAwaitAction,
        first_in_branch: bool,
        final_in_branch: bool,
        try_context: Option<AsyncTryContext>,
        control_context: Option<AsyncControlContext>,
    },
    IfBothBranches {
        contexts: Vec<AsyncControlContext>,
        enter_depth: usize,
        exit_depth: usize,
        if_id: Span,
        cond: Expr,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        action: AsyncNestedAwaitAction,
        in_then: bool,
        first_in_if: bool,
        first_in_branch: bool,
        final_in_branch: bool,
        try_context: Option<AsyncTryContext>,
    },
    SwitchSubject {
        switch_id: Span,
        cases: Vec<SwitchCase>,
        default: Option<Vec<Stmt>>,
        default_index: Option<usize>,
        subject_type: Type,
        subject_storage: Option<String>,
        context: Option<AsyncSwitchContext>,
    },
    SwitchCaseExpression {
        switch_id: Span,
        expr: Expr,
        cases: Vec<SwitchCase>,
        default: Option<Vec<Stmt>>,
        default_index: Option<usize>,
        subject_type: Type,
        subject_storage: String,
        case_index: usize,
        span: Span,
        payload_type: Type,
        prefixes: Vec<(Expr, Type)>,
        guard: Option<AsyncExpressionGuard>,
        final_in_case_expression: bool,
        first_in_switch: bool,
        context: Option<AsyncSwitchContext>,
    },
    Switch {
        switch_id: Span,
        expr: Expr,
        cases: Vec<SwitchCase>,
        default: Option<Vec<Stmt>>,
        default_index: Option<usize>,
        await_clause: usize,
        previous: Vec<Stmt>,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        action: AsyncNestedAwaitAction,
        first_in_clause: bool,
        final_in_clause: bool,
        first_in_switch: bool,
        final_in_switch: bool,
        entry_guard: Option<AsyncSwitchClauseGuard>,
        context: Option<AsyncSwitchContext>,
    },
    Expression {
        statement: Stmt,
        span: Span,
        ty: Type,
        prefixes: Vec<(Expr, Type)>,
        guard: Option<AsyncExpressionGuard>,
        final_in_statement: bool,
    },
    Try {
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        catch_binding: String,
        catch_type: Type,
        catch_body: Vec<Stmt>,
        finally_body: Vec<Stmt>,
        action: AsyncNestedAwaitAction,
        final_in_try: bool,
    },
    FinallyCompletion {
        contexts: Vec<AsyncControlContext>,
        enter_depth: usize,
        exit_depth: usize,
        try_body: Vec<Stmt>,
        catch_binding: String,
        catch_type: Type,
        catch_body: Vec<Stmt>,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        action: AsyncNestedAwaitAction,
        mode_storage: String,
        throw_storage: String,
        return_storage: Option<String>,
        first_in_finally: bool,
        final_in_finally: bool,
    },
}

#[derive(Clone)]
struct AsyncTryContext {
    catch_binding: String,
    catch_type: Type,
    catch_body: Vec<Stmt>,
    finally_body: Vec<Stmt>,
}

#[derive(Clone, PartialEq)]
enum AsyncControlContext {
    Block {
        before: Vec<Stmt>,
        after: Vec<Stmt>,
    },
    If {
        condition: Expr,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
        other: Vec<Stmt>,
        selected_in_then: bool,
    },
    While {
        condition: Expr,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
    },
    DoWhile {
        condition: Expr,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
    },
    For {
        init: Box<Stmt>,
        condition: Expr,
        update: Box<Stmt>,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
    },
    ForOf {
        name: String,
        iterable: Expr,
        element_type: Type,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
    },
    Try {
        try_id: Span,
        catch_binding: String,
        catch_type: Type,
        catch_body: Vec<Stmt>,
        finally_body: Vec<Stmt>,
        before: Vec<Stmt>,
        after: Vec<Stmt>,
    },
}

fn control_flow_contexts(site: &AsyncAwaitSite) -> Option<&[AsyncControlContext]> {
    match &site.kind {
        AsyncAwaitKind::ControlFlow { contexts, .. }
        | AsyncAwaitKind::IfBothBranches { contexts, .. }
        | AsyncAwaitKind::FinallyCompletion { contexts, .. } => Some(contexts),
        AsyncAwaitKind::SwitchSubject {
            context: Some(AsyncSwitchContext::Recursive { contexts, .. }),
            ..
        }
        | AsyncAwaitKind::SwitchCaseExpression {
            context: Some(AsyncSwitchContext::Recursive { contexts, .. }),
            ..
        }
        | AsyncAwaitKind::Switch {
            context: Some(AsyncSwitchContext::Recursive { contexts, .. }),
            ..
        } => Some(contexts),
        _ => None,
    }
}

fn control_flow_contexts_mut(
    site: &mut AsyncAwaitSite,
) -> Option<(&[AsyncControlContext], &mut usize, &mut usize)> {
    match &mut site.kind {
        AsyncAwaitKind::ControlFlow {
            contexts,
            enter_depth,
            exit_depth,
            ..
        }
        | AsyncAwaitKind::IfBothBranches {
            contexts,
            enter_depth,
            exit_depth,
            ..
        }
        | AsyncAwaitKind::FinallyCompletion {
            contexts,
            enter_depth,
            exit_depth,
            ..
        } => Some((contexts, enter_depth, exit_depth)),
        AsyncAwaitKind::SwitchSubject {
            context:
                Some(AsyncSwitchContext::Recursive {
                    contexts,
                    enter_depth,
                    exit_depth,
                }),
            ..
        }
        | AsyncAwaitKind::SwitchCaseExpression {
            context:
                Some(AsyncSwitchContext::Recursive {
                    contexts,
                    enter_depth,
                    exit_depth,
                }),
            ..
        }
        | AsyncAwaitKind::Switch {
            context:
                Some(AsyncSwitchContext::Recursive {
                    contexts,
                    enter_depth,
                    exit_depth,
                }),
            ..
        } => Some((contexts, enter_depth, exit_depth)),
        _ => None,
    }
}

fn common_control_context_prefix(
    left: &[AsyncControlContext],
    right: &[AsyncControlContext],
) -> usize {
    left.iter()
        .zip(right)
        .take_while(|(left, right)| left == right)
        .count()
}

impl AsyncControlContext {
    fn with_surrounding(self, before: Vec<Stmt>, after: Vec<Stmt>) -> Self {
        match self {
            Self::Block { .. } => Self::Block { before, after },
            Self::If {
                condition,
                other,
                selected_in_then,
                ..
            } => Self::If {
                condition,
                before,
                after,
                other,
                selected_in_then,
            },
            Self::While { condition, .. } => Self::While {
                condition,
                before,
                after,
            },
            Self::DoWhile { condition, .. } => Self::DoWhile {
                condition,
                before,
                after,
            },
            Self::For {
                init,
                condition,
                update,
                ..
            } => Self::For {
                init,
                condition,
                update,
                before,
                after,
            },
            Self::ForOf {
                name,
                iterable,
                element_type,
                ..
            } => Self::ForOf {
                name,
                iterable,
                element_type,
                before,
                after,
            },
            Self::Try {
                try_id,
                catch_binding,
                catch_type,
                catch_body,
                finally_body,
                ..
            } => Self::Try {
                try_id,
                catch_binding,
                catch_type,
                catch_body,
                finally_body,
                before,
                after,
            },
        }
    }
}

#[derive(Clone)]
struct AsyncExpressionGuard {
    condition: Expr,
    kind: AsyncExpressionGuardKind,
}

#[derive(Clone)]
enum AsyncExpressionGuardKind {
    Boolean { schedule_when_true: bool },
    Optional { schedule_when_null: bool },
}

#[derive(Clone)]
struct AsyncFrameParam {
    storage_name: String,
    ty: Type,
    pattern: ParamPattern,
}

#[derive(Clone, Copy)]
struct AsyncPendingFinally<'a, 'ctx> {
    group: &'a AsyncFinallyAwaitGroup,
    state: u64,
    poll: FunctionValue<'ctx>,
    frame_type: inkwell::types::StructType<'ctx>,
    frame: PointerValue<'ctx>,
    state_pointer: PointerValue<'ctx>,
}

impl<'ctx> AsyncPendingFinally<'_, 'ctx> {
    fn begin(
        &self,
        codegen: &mut Codegen<'ctx>,
        failure: PointerValue<'ctx>,
    ) -> Result<(), String> {
        codegen.emit_async_begin_finally(
            self.poll,
            self.frame_type,
            self.frame,
            self.state_pointer,
            self.state,
            self.group,
            Some(failure),
        )
    }
}

#[derive(Clone, Copy)]
struct AsyncTryRecovery<'a, 'ctx> {
    await_site: &'a AsyncAwaitSite,
    await_sites: &'a [AsyncAwaitSite],
    catch_await_groups: &'a [AsyncCatchAwaitGroup],
    finally_await_groups: &'a [AsyncFinallyAwaitGroup],
    recursive_control_blocks: &'a HashMap<(usize, usize), AsyncRecursiveControlBlocks<'ctx>>,
    poll: FunctionValue<'ctx>,
    frame_type: inkwell::types::StructType<'ctx>,
    frame: PointerValue<'ctx>,
    state_pointer: PointerValue<'ctx>,
    async_result: PointerValue<'ctx>,
    dependency: PointerValue<'ctx>,
    dependency_pointer: PointerValue<'ctx>,
    promise_free: FunctionValue<'ctx>,
    cancelled_flag: PointerValue<'ctx>,
}

#[derive(Clone)]
pub(crate) enum AsyncErrorCleanup<'ctx> {
    Message(StructValue<'ctx>),
    NetError {
        code: StructValue<'ctx>,
        message: StructValue<'ctx>,
    },
    Aggregate(PointerValue<'ctx>),
    Value {
        slot: PointerValue<'ctx>,
        ty: Type,
    },
    KeptError {
        slot: PointerValue<'ctx>,
        storage: ErrorSubtypeStorage<'ctx>,
    },
}

enum AsyncSegmentFlow {
    SkipRest,
    Proceed,
}

#[derive(Default)]
struct AsyncBlockRegistry<'ctx> {
    loop_blocks: HashMap<usize, (BasicBlock<'ctx>, BasicBlock<'ctx>, BasicBlock<'ctx>)>,
    for_blocks: HashMap<
        usize,
        (
            BasicBlock<'ctx>,
            BasicBlock<'ctx>,
            BasicBlock<'ctx>,
            BasicBlock<'ctx>,
        ),
    >,
    if_blocks: HashMap<usize, BasicBlock<'ctx>>,
    if_branch_path_blocks: HashMap<Span, BasicBlock<'ctx>>,
    control_body_blocks: HashMap<
        usize,
        (
            BasicBlock<'ctx>,
            BasicBlock<'ctx>,
            BasicBlock<'ctx>,
            BasicBlock<'ctx>,
        ),
    >,
    recursive_control_blocks: HashMap<(usize, usize), AsyncRecursiveControlBlocks<'ctx>>,
    recursive_try_blocks: HashMap<Span, BasicBlock<'ctx>>,
    finally_completion_blocks: HashMap<String, AsyncFinallyCompletionBlocks<'ctx>>,
    while_body_blocks: HashMap<usize, (BasicBlock<'ctx>, BasicBlock<'ctx>)>,
    do_while_body_blocks: HashMap<usize, (BasicBlock<'ctx>, BasicBlock<'ctx>, BasicBlock<'ctx>)>,
    for_body_blocks: HashMap<
        usize,
        (
            BasicBlock<'ctx>,
            BasicBlock<'ctx>,
            BasicBlock<'ctx>,
            BasicBlock<'ctx>,
        ),
    >,
    expression_guard_blocks: HashMap<usize, BasicBlock<'ctx>>,
    for_of_blocks: HashMap<usize, (BasicBlock<'ctx>, BasicBlock<'ctx>, BasicBlock<'ctx>)>,
    try_blocks: HashMap<usize, BasicBlock<'ctx>>,
    switch_blocks: HashMap<Span, AsyncSwitchBlocks<'ctx>>,
    switch_context_blocks: HashMap<Span, AsyncSwitchContextBlocks<'ctx>>,
    switch_clause_loop_blocks: HashMap<(Span, usize), (BasicBlock<'ctx>, BasicBlock<'ctx>)>,
}

#[derive(Clone, Copy)]
enum AsyncRecursiveControlBlocks<'ctx> {
    Block,
    If {
        join: BasicBlock<'ctx>,
    },
    Try {
        join: BasicBlock<'ctx>,
    },
    Loop {
        body: BasicBlock<'ctx>,
        continue_target: BasicBlock<'ctx>,
        condition: BasicBlock<'ctx>,
        after: BasicBlock<'ctx>,
        finally_depth: usize,
    },
}

#[derive(Clone, Copy)]
struct AsyncFinallyCompletionBlocks<'ctx> {
    entry: BasicBlock<'ctx>,
    normal: BasicBlock<'ctx>,
    break_target: Option<BasicBlock<'ctx>>,
    continue_target: Option<BasicBlock<'ctx>>,
}

#[derive(Clone)]
struct AsyncSwitchBlocks<'ctx> {
    end: BasicBlock<'ctx>,
    case_blocks: Vec<BasicBlock<'ctx>>,
    unmatched: BasicBlock<'ctx>,
    clause_fallthrough: Vec<BasicBlock<'ctx>>,
}

#[derive(Clone, Copy)]
enum AsyncSwitchContextBlocks<'ctx> {
    Recursive,
    If {
        join: BasicBlock<'ctx>,
    },
    While {
        condition: BasicBlock<'ctx>,
        after: BasicBlock<'ctx>,
    },
    DoWhile {
        condition: BasicBlock<'ctx>,
        body: BasicBlock<'ctx>,
        after: BasicBlock<'ctx>,
    },
    For {
        condition: BasicBlock<'ctx>,
        update: BasicBlock<'ctx>,
        after: BasicBlock<'ctx>,
    },
    ForOf {
        condition: BasicBlock<'ctx>,
        update: BasicBlock<'ctx>,
        after: BasicBlock<'ctx>,
        index: PointerValue<'ctx>,
    },
    Try {
        join: BasicBlock<'ctx>,
    },
}

#[derive(Clone)]
struct AsyncAwaitSite {
    outer_index: usize,
    dependency: Expr,
    kind: AsyncAwaitKind,
}

#[derive(Clone)]
struct AsyncCatchAwaitSite {
    dependency: Expr,
    before: Vec<Stmt>,
    after: Vec<Stmt>,
    action: AsyncNestedAwaitAction,
    final_in_catch: bool,
}

#[derive(Clone)]
struct AsyncSynchronousCatch<'ctx> {
    context_slot: PointerValue<'ctx>,
    catch_top: PointerValue<'ctx>,
    previous: PointerValue<'ctx>,
    error_slot: PointerValue<'ctx>,
    owned_slot: PointerValue<'ctx>,
    catch_block: BasicBlock<'ctx>,
}

struct AsyncCatchAwaitGroup {
    outer_index: usize,
    recursive_try_depth: Option<usize>,
    recursive_try_id: Option<Span>,
    catch_binding: String,
    catch_type: Type,
    storage_name: String,
    finally_body: Vec<Stmt>,
    finally_at_recovered: bool,
    sites: Vec<AsyncCatchAwaitSite>,
}

#[derive(Clone)]
struct AsyncFinallyAwaitSite {
    dependency: Expr,
    before: Vec<Stmt>,
    after: Vec<Stmt>,
    action: AsyncNestedAwaitAction,
    final_in_finally: bool,
}

#[derive(Clone)]
struct AsyncFinallyAwaitGroup {
    outer_index: usize,
    recursive_try_id: Option<Span>,
    mode_storage: String,
    failure_storage: String,
    completion: Option<AsyncFinallyCompletionStorage>,
    sites: Vec<AsyncFinallyAwaitSite>,
}

#[derive(Clone)]
struct AsyncFinallyCompletionStorage {
    mode_storage: String,
    throw_storage: String,
    return_storage: String,
}

fn async_promise_handle(ty: &Type) -> bool {
    matches!(transparent_codegen_type(ty), Type::Apply { name, args } if matches!(name.as_str(), "Promise" | "DbPromise") && args.len() == 1)
}

fn async_http_reply_handle(ty: &Type) -> bool {
    matches!(
        transparent_codegen_type(ty),
        Type::Apply { name, args } if name == "HttpReply" && args.len() == 1
    )
}

fn async_optional_copy(ty: &Type) -> bool {
    matches!(transparent_codegen_type(ty), Type::Optional(inner) if inner.is_copy())
}

pub(super) fn async_move_only_native_handle(ty: &Type) -> bool {
    match transparent_codegen_type(ty) {
        Type::Named(name) => {
            tyrus_core::native_handles::is_opaque_handle_name(name)
                && !matches!(name.as_str(), "HttpServer" | "Semaphore" | "WebSocket")
        }
        Type::Apply { name, args } => {
            matches!(name.as_str(), "MutexGuard" | "RwReadGuard" | "RwWriteGuard")
                && args.len() == 1
        }
        _ => false,
    }
}

fn async_ref_counted_native_handle(ty: &Type) -> bool {
    matches!(
        transparent_codegen_type(ty),
        Type::Apply { name, args }
            if matches!(name.as_str(), "Sender" | "Receiver" | "Mutex" | "RwLock")
                && args.len() == 1
    ) || matches!(
        transparent_codegen_type(ty),
        Type::Apply { name, args }
            if matches!(name.as_str(), "DuplexStream" | "TransformStream") && args.len() == 2
    ) || matches!(
        transparent_codegen_type(ty),
        Type::Apply { name, args }
            if matches!(name.as_str(), "ReadableStream" | "WritableStream") && args.len() == 1
    ) || matches!(transparent_codegen_type(ty), Type::Named(name) if matches!(name.as_str(), "Semaphore" | "ChildProcess" | "WebSocket"))
}

fn async_frame_ownable_shape(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Named(_) | Type::Object(_) | Type::Result { .. } | Type::Union(_) | Type::Tuple(_)
    )
}

fn async_copy_array(ty: &Type) -> bool {
    matches!(transparent_codegen_type(ty), Type::Array(element) if element.is_copy())
}

fn async_string_array(ty: &Type) -> bool {
    matches!(transparent_codegen_type(ty), Type::Array(element) if matches!(transparent_codegen_type(element), Type::Str))
}

static STDOUT_CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn capture_stdout<F: FnOnce()>(run: F) -> Result<String, String> {
    let _serialized = STDOUT_CAPTURE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let stdout_handle = std::io::stdout();
    let mut rust_stdout = stdout_handle.lock();
    let _ = std::io::Write::flush(&mut rust_stdout);
    if unsafe { libc::fflush(std::ptr::null_mut()) } != 0 {
        return Err("error: failed to flush C stdio before stdout capture".to_string());
    }

    let mut fds: [i32; 2] = [-1, -1];
    if create_capture_pipe(&mut fds) != 0 {
        return Err("error: failed to create pipe for stdout capture".to_string());
    }
    let read_fd = fds[0];
    let write_fd = fds[1];

    let orig_stdout = unsafe { libc::dup(1) };
    if orig_stdout < 0 {
        unsafe {
            libc::close(read_fd);
            libc::close(write_fd);
        }
        return Err("error: failed to dup stdout".to_string());
    }

    let reader = match std::thread::Builder::new()
        .name("tyrus-stdout-capture".to_string())
        .spawn(move || read_capture_pipe(read_fd))
    {
        Ok(reader) => reader,
        Err(_) => {
            unsafe {
                libc::close(read_fd);
                libc::close(write_fd);
                libc::close(orig_stdout);
            }
            return Err("error: failed to start the stdout capture reader".to_string());
        }
    };

    struct RestoreStdout {
        descriptor: i32,
    }
    impl Drop for RestoreStdout {
        fn drop(&mut self) {
            unsafe {
                libc::fflush(std::ptr::null_mut());
                libc::dup2(self.descriptor, 1);
                libc::close(self.descriptor);
                #[cfg(windows)]
                {
                    let handle = libc::get_osfhandle(1);
                    if handle != -1 {
                        windows_sys::Win32::System::Console::SetStdHandle(
                            windows_sys::Win32::System::Console::STD_OUTPUT_HANDLE,
                            handle as windows_sys::Win32::Foundation::HANDLE,
                        );
                    }
                }
            }
        }
    }

    {
        let _restore = RestoreStdout {
            descriptor: orig_stdout,
        };
        let redirected = unsafe { libc::dup2(write_fd, 1) };
        unsafe { libc::close(write_fd) };
        if redirected < 0 {
            drop(_restore);
            let _ = reader.join();
            return Err("error: failed to redirect stdout for capture".to_string());
        }

        #[cfg(windows)]
        {
            let handle = unsafe { libc::get_osfhandle(1) };
            if handle == -1
                || unsafe {
                    windows_sys::Win32::System::Console::SetStdHandle(
                        windows_sys::Win32::System::Console::STD_OUTPUT_HANDLE,
                        handle as windows_sys::Win32::Foundation::HANDLE,
                    )
                } == 0
            {
                drop(_restore);
                let _ = reader.join();
                return Err(
                    "error: failed to redirect the Win32 stdout handle for capture".to_string(),
                );
            }
        }

        run();

        unsafe { libc::fflush(std::ptr::null_mut()) };
    }

    let output = reader
        .join()
        .map_err(|_| "error: the stdout capture reader panicked".to_string())?;

    Ok(String::from_utf8_lossy(&output).into_owned())
}

#[cfg(not(windows))]
fn create_capture_pipe(fds: &mut [i32; 2]) -> i32 {
    unsafe { libc::pipe(fds.as_mut_ptr()) }
}

#[cfg(windows)]
fn create_capture_pipe(fds: &mut [i32; 2]) -> i32 {
    unsafe { libc::pipe(fds.as_mut_ptr(), 64 * 1024, libc::O_BINARY) }
}

#[cfg(not(windows))]
fn read_capture_pipe(read_fd: i32) -> Vec<u8> {
    let mut pipe = unsafe { <std::fs::File as std::os::fd::FromRawFd>::from_raw_fd(read_fd) };
    let mut output = Vec::new();
    let _ = std::io::Read::read_to_end(&mut pipe, &mut output);
    output
}

#[cfg(windows)]
fn read_capture_pipe(read_fd: i32) -> Vec<u8> {
    let mut output = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = unsafe {
            libc::read(
                read_fd,
                buffer.as_mut_ptr().cast(),
                buffer.len() as libc::c_uint,
            )
        };
        if count <= 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }
    unsafe { libc::close(read_fd) };
    output
}

pub(crate) const PURE_TOTAL_RUNTIME_FNS: &[&str] = &[
    "tyrus_char_is_alphabetic",
    "tyrus_char_is_alphanumeric",
    "tyrus_char_is_digit",
    "tyrus_char_is_valid",
    "tyrus_char_is_whitespace",
    "tyrus_char_to_lower",
    "tyrus_char_to_upper",
    "tyrus_math_abs",
    "tyrus_math_acos",
    "tyrus_math_asin",
    "tyrus_math_atan",
    "tyrus_math_atan2",
    "tyrus_math_cbrt",
    "tyrus_math_ceil",
    "tyrus_math_clamp",
    "tyrus_math_clamp_max",
    "tyrus_math_clamp_min",
    "tyrus_math_clz32",
    "tyrus_math_copysign",
    "tyrus_math_cos",
    "tyrus_math_cosh",
    "tyrus_math_deg_to_rad",
    "tyrus_math_exp",
    "tyrus_math_floor",
    "tyrus_math_fmod",
    "tyrus_math_gcd",
    "tyrus_math_hypot",
    "tyrus_math_imul",
    "tyrus_math_inverse_lerp",
    "tyrus_math_lerp",
    "tyrus_math_log",
    "tyrus_math_log10",
    "tyrus_math_log2",
    "tyrus_math_log_base",
    "tyrus_math_max",
    "tyrus_math_min",
    "tyrus_math_pow",
    "tyrus_math_powi",
    "tyrus_math_rad_to_deg",
    "tyrus_math_remap",
    "tyrus_math_round",
    "tyrus_math_round_to",
    "tyrus_math_sign",
    "tyrus_math_sin",
    "tyrus_math_sinh",
    "tyrus_math_smoothstep",
    "tyrus_math_sqrt",
    "tyrus_math_tan",
    "tyrus_math_tanh",
    "tyrus_math_to_cartesian",
    "tyrus_math_to_polar",
    "tyrus_math_trunc",
];

pub(crate) fn is_pure_total_runtime_fn(name: &str) -> bool {
    PURE_TOTAL_RUNTIME_FNS.contains(&name)
}

pub(crate) const READING_TOTAL_RUNTIME_FNS: &[&str] = &[
    "tyrus_math_approx_equal",
    "tyrus_math_mean",
    "tyrus_math_std_dev",
    "tyrus_math_variance",
];

pub(crate) fn is_reading_total_runtime_fn(name: &str) -> bool {
    READING_TOTAL_RUNTIME_FNS.contains(&name)
}

pub(crate) const PURE_ABORTING_RUNTIME_FNS: &[&str] = &[
    "tyrus_math_combination",
    "tyrus_math_factorial",
    "tyrus_math_fibonacci",
    "tyrus_math_is_prime",
    "tyrus_math_lcm",
    "tyrus_math_next_prime",
    "tyrus_math_permutation",
];

pub(crate) fn is_pure_aborting_runtime_fn(name: &str) -> bool {
    PURE_ABORTING_RUNTIME_FNS.contains(&name)
}
