use inkwell::IntPredicate;
use inkwell::attributes::{Attribute, AttributeLoc};
use inkwell::basic_block::BasicBlock;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::intrinsics::Intrinsic;
use inkwell::module::Module;
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{CodeModel, RelocMode, Target, TargetMachine};
use inkwell::types::{
    BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FloatType, FunctionType, IntType, PointerType,
    StructType,
};
use inkwell::values::{
    AsValueRef, BasicMetadataValueEnum, BasicValueEnum, FloatValue, FunctionValue, GlobalValue,
    IntValue, PointerValue, StructValue,
};
use std::path::Path;
use tyrus_core::ast::*;
use tyrus_core::collections::{HashMap, HashSet};
use tyrus_core::env::EnvSchema;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompileTarget {
    Native,
    Wasm,
}

impl CompileTarget {
    pub fn triple(self) -> &'static str {
        match self {
            CompileTarget::Native => "",
            CompileTarget::Wasm => "wasm32-unknown-unknown",
        }
    }
}

#[path = "algorithm/algorithm.rs"]
mod algorithm;
#[path = "arrays/arrays.rs"]
mod arrays;
#[path = "cli/cli.rs"]
mod cli;
#[path = "core/core.rs"]
mod core;
#[path = "crypto/crypto.rs"]
mod crypto;
#[path = "encoding/encoding.rs"]
mod encoding;
#[path = "env/env.rs"]
mod env;
#[path = "expressions/expressions.rs"]
mod expressions;
#[path = "filesystem/filesystem.rs"]
mod filesystem;
#[path = "functional_arrays/functional_arrays.rs"]
mod functional_arrays;
#[path = "helpers/helpers.rs"]
mod helpers;
#[path = "http/http.rs"]
mod http;
#[path = "maps/maps.rs"]
mod maps;
#[path = "memory/memory.rs"]
mod memory;
#[path = "methods/methods.rs"]
mod methods;
#[path = "process/process.rs"]
mod process;
#[path = "repl_session/repl_session.rs"]
mod repl_session;
#[path = "statements/statements.rs"]
mod statements;
use statements::{AsyncRejection, ThrownValue};
#[path = "string_repr/string_repr.rs"]
mod string_repr;
#[path = "strings/strings.rs"]
mod strings;
#[path = "test/test.rs"]
mod test;
#[path = "validators/validators.rs"]
mod validators;

use helpers::{
    closure_captures, const_float_literal, field_storage_type, infer_codegen_generics,
    is_float_type, is_functional_array_method, is_integer_type, is_unsigned_int_type, mangle_type,
    native_collection_handle, numeric_constant_type, substitute_type, transparent_codegen_type,
};

/// How to release something the current scope owns.
///
/// Ownership used to be a plain "this scope owns it" flag, and the release
/// worked the rest out from the value's *type*. That is enough while every
/// allocable thing inside a value is an allocation, and wrong as soon as one is
/// not: `["a", "b"]` allocates its buffer but not its elements, so a release
/// driven by the type alone either frees static strings (corruption) or frees
/// nothing and leaks the buffer.
///
/// Recording the plan at the point ownership is *taken*, where what was
/// allocated is still known, is what lets the two be told apart.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReleasePlan {
    Deep,
    Buffer,
    Value,
    /// Release only the listed fields, each under its own plan.
    ///
    /// What lets an aggregate be claimed when its parts disagree. The release
    /// used to free an object's string fields as a group, so an object mixing
    /// an allocated and a static string could not be claimed at all without
    /// freeing the static one, it was left to leak. Naming the fields makes
    /// the mixed case expressible, and reaches fields the group rule never
    /// looked at, such as an array held by an object.
    Fields(Vec<(usize, ReleasePlan)>),
    /// Release an array's buffer, plus the elements at the listed buffer
    /// offsets (each freed `Deep` under the array's own element type).
    ///
    /// `Fields` cannot express this: it reaches into an aggregate by
    /// `extract_value` on a fixed struct index, while an array element lives
    /// at a runtime `gep` offset into a malloc'd buffer. A literal mixing
    /// static and freshly-allocated elements (`["a", b.toUpperCase(), "c"]`)
    /// needed the same "name the parts that disagree" idea `Fields` gave
    /// objects, applied to buffer offsets instead of struct fields, without
    /// it, this shape could only be claimed all-or-nothing, and a mix left
    /// both the buffer and every owned element to leak.
    ArrayElements(Vec<usize>),
    /// Release an `Optional` box: free what the boxed value owns under the
    /// given inner plan, then free the box pointer itself.
    ///
    /// `emit_optional_box` never registers its own release, its result is
    /// used inside conditional branches a caller further merges into a phi
    /// in some call sites, and a pre-merge registration does not dominate a
    /// post-merge free. Boxing a value that *itself* already carries its own
    /// pending registration (an object literal with an owned optional field,
    /// coerced into `T?`) left that inner registration on the pre-box value,
    /// a different SSA value than the box pointer everything downstream
    /// actually holds. Nothing ever matched it by identity to forget or
    /// transfer it, so it survived as an orphaned pending entry, eventually
    /// swept by an unrelated later statement's cleanup, which crashed the
    /// LLVM verifier ("does not dominate all uses") when that statement
    /// happened to sit in a sibling branch, and silently reused already-freed
    /// memory (wrong output, not caught by valgrind) when it did not. `Box`
    /// re-registers the inner plan against the box's own pointer instead, so
    /// the existing identity-based forget/transfer machinery reaches it.
    Box(Box<ReleasePlan>),
}

pub struct Codegen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    vars: Vec<HashMap<String, (PointerValue<'ctx>, Type)>>,
    var_concrete_types: Vec<HashMap<String, Type>>,
    non_zero_vars: Vec<HashSet<String>>,
    narrowed_paths: Vec<HashSet<String>>,
    owned_closure_vars: Vec<HashSet<String>>,
    owned_closure_drop_fns: Vec<HashMap<String, FunctionValue<'ctx>>>,
    owned_native_vars: Vec<HashMap<String, ReleasePlan>>,
    for_index_hint: Option<Type>,
    pending_owned_native_results: Vec<(BasicValueEnum<'ctx>, Type, ReleasePlan)>,
    for_of_iterable_cleanups: Vec<(BasicValueEnum<'ctx>, Type, ReleasePlan)>,
    interned_strings: std::cell::RefCell<HashMap<String, inkwell::values::GlobalValue<'ctx>>>,
    pending_owned_native_guard: Vec<(usize, usize)>,
    pending_conditionally_owned_results: Vec<(BasicValueEnum<'ctx>, Type, IntValue<'ctx>)>,
    conditionally_owned_vars: Vec<HashMap<String, IntValue<'ctx>>>,
    mutable_string_ownership: Vec<HashMap<String, PointerValue<'ctx>>>,
    /// A synchronous `string[]` local's ownership *mode*, a stack slot holding
    /// an `i8`, the array sibling of `mutable_string_ownership` and the
    /// synchronous mirror of the async frame's tracked string-array mode.
    ///
    /// `1` means the elements are static (a literal of `.rodata` pointers) and
    /// only the buffer may be freed; `2` means every element is its own
    /// allocation, so the whole array is released through
    /// `tyrus_array_drop_string_contents`. A static `ReleasePlan` cannot answer
    /// this, for the same reason it cannot answer the mutable-string question
    /// above: `parts.push(x)` inside an `if` compiles to one point in the AST
    /// but only sometimes runs, so "were the elements replaced by clones?" is a
    /// property of the execution, not of the compilation. Freeing on the wrong
    /// answer is not a leak but a crash, `free` on a `.rodata` pointer, which
    /// is why the mode is read back at run time instead of assumed.
    ///
    /// Mode `1` is what keeps a purely static array allocation-free: it is
    /// promoted to `2` only when something actually mutates the array, and the
    /// promotion clones the existing elements exactly once
    /// (`normalize_string_array_for_mutation`). A name tracked here is never
    /// also entered into `owned_native_vars`: the two are mutually exclusive
    /// per binding.
    string_array_ownership: Vec<HashMap<String, PointerValue<'ctx>>>,
    pending_closure_drop_fn: Option<FunctionValue<'ctx>>,
    capture_test_assertions: bool,
    contained_arrays: tyrus_core::escape_analysis::ContainedArrays,
    async_capture_param_count: usize,
    constant_global_arrays: tyrus_core::escape_analysis::ConstantGlobalArrays,
    checked_indexes: tyrus_core::bounds_check::CheckedIndexes,
    confined_closures: tyrus_core::escape_analysis::ConfinedClosures,
    confined_closure_pending: bool,
    closure_env_on_stack: bool,
    stack_array_pending: bool,
    /// What the array literal just emitted actually did, as opposed to what it
    /// was asked to do. The declaration reads this to decide whether to claim
    /// the buffer, and the two questions are not the same: the literal refuses
    /// the stack for shapes the analysis never sees, and a declaration that
    /// skipped the claim for a buffer that ended up on the heap would leak it.
    array_literal_on_stack: bool,
    fns: HashMap<String, FunctionValue<'ctx>>,
    auto_route_exports: Vec<String>,
    fn_signatures: HashMap<String, (Vec<Type>, Type)>,
    user_validator_names: HashSet<String>,
    extension_methods: Vec<ExtensionMethod>,
    behavior_targets: HashMap<String, Vec<Type>>,
    fn_owned_returns: HashSet<String>,
    fn_conditionally_owned_returns: HashSet<String>,
    fn_db_async_returns: HashSet<String>,
    fn_throws: HashMap<String, Vec<Type>>,
    /// Local closures declared with `throws`, whose variable type does not
    /// carry it. Only decides where a call gets a throw landing, so a stale
    /// name after its scope ends costs a `setjmp` and nothing else.
    throwing_closures: HashSet<String>,
    throw_frames: Vec<crate::statements::ThrowFrame>,
    error_subtypes: std::collections::HashSet<String>,
    catch_error_subtypes: Vec<crate::statements::ErrorSubtypeStorage<'ctx>>,
    next_assert_throws_id: usize,
    async_functions: HashSet<String>,
    fn_param_metadata: HashMap<String, Vec<Param>>,
    generic_fns: HashMap<String, GenericFunctionDefinition>,
    generic_specializations: HashMap<String, FunctionValue<'ctx>>,
    type_substitutions: Vec<HashMap<String, Type>>,
    return_types: Vec<Type>,
    async_results: Vec<PointerValue<'ctx>>,
    async_temporaries_tracked: usize,
    async_frame_slots: Vec<HashMap<String, (PointerValue<'ctx>, Type)>>,
    async_frame_string_ownership: Vec<HashMap<String, PointerValue<'ctx>>>,
    async_frame_tracked_copy_arrays: Vec<HashSet<String>>,
    async_frame_string_array_ownership: Vec<HashMap<String, PointerValue<'ctx>>>,
    async_frame_owned_values: Vec<HashSet<String>>,
    async_frame_owned_initialization: Vec<HashMap<String, PointerValue<'ctx>>>,
    async_frame_local_storage: Vec<HashMap<String, core::AsyncLocalStorage>>,
    async_await_values: Vec<HashMap<tyrus_core::span::Span, (PointerValue<'ctx>, Type)>>,
    async_prescan_types: HashMap<String, Type>,
    type_aliases: HashMap<String, Type>,
    interfaces: HashMap<String, Vec<Field>>,
    declared_interfaces: HashMap<String, Vec<Field>>,
    db_models: HashMap<String, tyrus_core::ast::DbModelMetadata>,
    /// Interfaces whose field order is an ABI contract with the runtime, and so
    /// must never be permuted. Captured from whatever `Codegen::new` seeded
    /// before any user interface is registered, which is exactly the native
    /// set, a gate that cannot fall out of date as native modules are added.
    native_layout_interfaces: HashSet<String>,
    enums: HashMap<String, ResolvedEnum>,
    arrow_fn_count: usize,
    /// Depth of optional-chain lowering currently open.
    ///
    /// A `?.` call is emitted inside a conditional branch, while a scope's
    /// cleanup runs after the branches rejoin. Recording ownership there
    /// produced LLVM IR where the value did not dominate its own release, so
    /// ownership is not tracked inside a chain: the value is left to leak
    /// rather than emitting invalid code or freeing on a path that never ran.
    optional_chain_depth: usize,
    main_block: Option<BasicBlock<'ctx>>,
    loop_stack: Vec<(BasicBlock<'ctx>, BasicBlock<'ctx>, usize)>,
    break_targets: Vec<BreakTarget<'ctx>>,
    deferred_scopes: Vec<Vec<DeferredFinalizer>>,
    finally_scopes: Vec<FinallyScope>,
    async_finally_completion_scopes: Vec<AsyncFinallyCompletionScope<'ctx>>,
    async_catch_cleanup_scopes: Vec<AsyncCatchCleanupScope<'ctx>>,
    catch_context_restores: Vec<(PointerValue<'ctx>, PointerValue<'ctx>, usize)>,
    catch_binding_types: Vec<Type>,
    async_throw_regions: Vec<core::throw_regions::AsyncThrowRegions<'ctx>>,
    catch_error_owned_slots: Vec<(String, PointerValue<'ctx>)>,
    match_result_bindings: Vec<HashSet<String>>,
    catch_binding_releases: Vec<statements::CatchBindingRelease<'ctx>>,
    async_error_cleanups: Vec<(String, core::AsyncErrorCleanup<'ctx>)>,
    validation_cleanup_scopes: Vec<ValidationCleanupScope<'ctx>>,
    env_startup: Option<EnvStartup>,
    worker_threads: Option<u32>,
    development_https: bool,
    database_path: Option<String>,
    env_var_types: HashMap<String, Type>,
    env_validation_mode: bool,
    validation_target_path: Vec<String>,
    validation_depth: usize,
    topic_values: Vec<(BasicValueEnum<'ctx>, Type)>,
    target: CompileTarget,
}

#[derive(Clone)]
struct ExtensionMethod {
    owner: String,
    target: Type,
    name: String,
    symbol: String,
}

fn specialize_extension_receiver(params: &[Param], target: &Type) -> Vec<Param> {
    let mut params = params.to_vec();
    if let Some(receiver) = params.first_mut()
        && matches!(receiver.ty, Type::Infer)
    {
        receiver.ty = target.clone();
    }
    params
}

#[derive(Clone)]
struct BreakTarget<'ctx> {
    block: BasicBlock<'ctx>,
    finally_depth: usize,
    validation_depth: usize,
    scope_depth: usize,
    value_slot: Option<(PointerValue<'ctx>, Type)>,
}

#[derive(Clone)]
struct FinallyScope {
    body: Vec<Stmt>,
    catches_throw: bool,
    loop_depth: usize,
}

#[derive(Clone)]
struct DeferredFinalizer {
    kind: DeferKind,
    body: Vec<Stmt>,
}

#[derive(Clone)]
struct AsyncFinallyCompletionScope<'ctx> {
    mode_slot: PointerValue<'ctx>,
    return_slot: Option<(PointerValue<'ctx>, Type)>,
    return_storage: Option<String>,
    entry: BasicBlock<'ctx>,
    throw_entry: Option<BasicBlock<'ctx>>,
    throw_storage: String,
    break_target: Option<BreakTarget<'ctx>>,
    continue_target: Option<(BasicBlock<'ctx>, usize)>,
    throw_slot: PointerValue<'ctx>,
    intercept_throw: bool,
}

#[derive(Clone)]
struct AsyncCatchCleanupScope<'ctx> {
    slot: PointerValue<'ctx>,
    ty: Type,
    binding: String,
    storage_name: String,
    break_depth: usize,
    loop_depth: usize,
    transferred: bool,
}

#[derive(Clone, Copy)]
struct ValidationCleanupScope<'ctx> {
    error_slot: PointerValue<'ctx>,
    loop_depth: usize,
    transferred: bool,
}

struct EnvStartup {
    init: EnvInit,
    vars: Vec<EnvStartupVar>,
}

enum EnvInit {
    File {
        path: String,
    },
    Sealed {
        project_name: String,
        fingerprint: String,
    },
}

struct EnvStartupVar {
    name: String,
    ty: Type,
    message: Option<String>,
    decorators: Vec<ValidatorDecorator>,
}

pub struct SealedEnvMetadata {
    pub project_name: String,
    pub fingerprint: String,
}

#[derive(Clone)]
struct GenericFunctionDefinition {
    type_params: Vec<TypeParam>,
    params: Vec<Param>,
    ret_type: Type,
    body: Vec<Stmt>,
}

pub fn compile(
    program: &Program,
    project_name: &str,
    output_path: &str,
    optimize: bool,
) -> Result<(), String> {
    compile_with_env(program, project_name, output_path, optimize, None)
}

pub fn compile_with_env(
    program: &Program,
    project_name: &str,
    output_path: &str,
    optimize: bool,
    env_schema: Option<&EnvSchema>,
) -> Result<(), String> {
    compile_with_env_options(
        program,
        project_name,
        output_path,
        optimize,
        env_schema,
        None,
    )
}

pub fn compile_with_env_options(
    program: &Program,
    project_name: &str,
    output_path: &str,
    optimize: bool,
    env_schema: Option<&EnvSchema>,
    sealed_env: Option<&SealedEnvMetadata>,
) -> Result<(), String> {
    compile_with_env_options_and_target(
        program,
        project_name,
        output_path,
        optimize,
        env_schema,
        sealed_env,
        CompileTarget::Native,
    )
}

pub fn compile_with_env_options_and_target(
    program: &Program,
    project_name: &str,
    output_path: &str,
    optimize: bool,
    env_schema: Option<&EnvSchema>,
    sealed_env: Option<&SealedEnvMetadata>,
    target: CompileTarget,
) -> Result<(), String> {
    compile_with_runtime_options(
        program,
        project_name,
        output_path,
        optimize,
        env_schema,
        sealed_env,
        target,
        None,
        false,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn compile_with_runtime_options(
    program: &Program,
    project_name: &str,
    output_path: &str,
    optimize: bool,
    env_schema: Option<&EnvSchema>,
    sealed_env: Option<&SealedEnvMetadata>,
    target: CompileTarget,
    worker_threads: Option<u32>,
    development_https: bool,
    database_path: Option<&str>,
) -> Result<(), String> {
    let context = Context::create();
    let mut codegen = Codegen::new(&context, project_name, optimize, target);
    if let Some(threads) = worker_threads {
        codegen.set_worker_threads(threads);
    }
    if development_https {
        codegen.set_development_https();
    }
    if let Some(path) = database_path {
        codegen.set_database_path(path);
    }
    if let Some(schema) = env_schema {
        if let Some(sealed) = sealed_env {
            codegen.set_sealed_env_schema(schema, sealed);
        } else {
            codegen.set_env_schema(schema);
        }
    }
    codegen.compile(program, output_path)
}

pub fn jit_run(program: &Program, project_name: &str) -> Result<String, String> {
    if cfg!(windows) && std::env::var_os("TYRUS_JIT_TRACE").is_some() {
        eprintln!("JIT trace: context creation");
    }
    let context = Context::create();
    if cfg!(windows) && std::env::var_os("TYRUS_JIT_TRACE").is_some() {
        eprintln!("JIT trace: codegen initialization");
    }
    let codegen = Codegen::new(&context, project_name, false, CompileTarget::Native);
    codegen.jit_and_run(program)
}

pub use repl_session::ReplJitSession;

#[cfg(test)]
#[path = "../tests/unit/codegen.rs"]
mod tests;
