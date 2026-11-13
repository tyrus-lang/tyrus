use std::ffi::CString;
use std::mem::forget;
use tyrus_core::collections::HashSet;

use super::CompileTarget;
use inkwell::context::Context;
use inkwell::module::{Linkage, Module};
use llvm_sys::core::{LLVMContextCreate, LLVMContextDispose};
use llvm_sys::error::{LLVMDisposeErrorMessage, LLVMErrorRef, LLVMGetErrorMessage};
use llvm_sys::orc2::lljit::{
    LLVMOrcCreateLLJIT, LLVMOrcCreateLLJITBuilder, LLVMOrcDisposeLLJIT,
    LLVMOrcLLJITAddLLVMIRModule, LLVMOrcLLJITBuilderSetJITTargetMachineBuilder,
    LLVMOrcLLJITGetMainJITDylib, LLVMOrcLLJITLookup, LLVMOrcLLJITRef,
};
use llvm_sys::orc2::{
    LLVMOrcCreateDynamicLibrarySearchGeneratorForProcess,
    LLVMOrcCreateNewThreadSafeContextFromLLVMContext, LLVMOrcCreateNewThreadSafeModule,
    LLVMOrcDisposeThreadSafeContext, LLVMOrcJITDylibAddGenerator, LLVMOrcJITDylibRef,
    LLVMOrcJITTargetMachineBuilderDetectHost, LLVMOrcThreadSafeContextRef,
};
use llvm_sys::prelude::LLVMContextRef;
use llvm_sys::target::{
    LLVM_InitializeNativeAsmParser, LLVM_InitializeNativeAsmPrinter, LLVM_InitializeNativeTarget,
};
use tyrus_core::ast::{Item, Program, Stmt, StmtKind};

use crate::Codegen;
use crate::core::capture_stdout;

pub struct ReplJitSession {
    context: LLVMContextRef,
    ts_context: LLVMOrcThreadSafeContextRef,
    jit: LLVMOrcLLJITRef,
    main_dylib: LLVMOrcJITDylibRef,
    known_symbols: HashSet<String>,
    cleanup_symbols: Vec<String>,
    next_cleanup_id: usize,
}

impl ReplJitSession {
    pub fn new() -> Result<Self, String> {
        unsafe {
            LLVM_InitializeNativeTarget();
            LLVM_InitializeNativeAsmPrinter();
            LLVM_InitializeNativeAsmParser();

            let context = LLVMContextCreate();
            if context.is_null() {
                return Err("failed to create the JIT context".to_string());
            }
            let ts_context = LLVMOrcCreateNewThreadSafeContextFromLLVMContext(context);
            if ts_context.is_null() {
                LLVMContextDispose(context);
                return Err("failed to create the JIT context".to_string());
            }

            let mut target_machine = std::ptr::null_mut();
            let err = LLVMOrcJITTargetMachineBuilderDetectHost(&mut target_machine);
            if !err.is_null() {
                let msg = orc_error_message(err);
                LLVMOrcDisposeThreadSafeContext(ts_context);
                return Err(format!("failed to detect the host target: {msg}"));
            }

            let builder = LLVMOrcCreateLLJITBuilder();
            LLVMOrcLLJITBuilderSetJITTargetMachineBuilder(builder, target_machine);

            let mut jit = std::ptr::null_mut();
            let err = LLVMOrcCreateLLJIT(&mut jit, builder);
            if !err.is_null() {
                let msg = orc_error_message(err);
                LLVMOrcDisposeThreadSafeContext(ts_context);
                return Err(format!("failed to create the JIT engine: {msg}"));
            }

            let main_dylib = LLVMOrcLLJITGetMainJITDylib(jit);

            let mut generator = std::ptr::null_mut();
            let err = LLVMOrcCreateDynamicLibrarySearchGeneratorForProcess(
                &mut generator,
                0,
                None,
                std::ptr::null_mut(),
            );
            if !err.is_null() {
                let msg = orc_error_message(err);
                LLVMOrcDisposeLLJIT(jit);
                LLVMOrcDisposeThreadSafeContext(ts_context);
                return Err(format!(
                    "failed to create the process symbol generator: {msg}"
                ));
            }
            LLVMOrcJITDylibAddGenerator(main_dylib, generator);

            Ok(Self {
                context,
                ts_context,
                jit,
                main_dylib,
                known_symbols: HashSet::default(),
                cleanup_symbols: Vec::new(),
                next_cleanup_id: 0,
            })
        }
    }

    pub fn add_and_run(
        &mut self,
        program: &Program,
        project_name: &str,
        step_name: &str,
    ) -> Result<String, String> {
        self.add_module(program, project_name)?;
        self.run_step(step_name)
    }

    pub fn add_module(&mut self, program: &Program, project_name: &str) -> Result<(), String> {
        unsafe {
            let raw_context = self.context;
            // `context` does not own `raw_context`: it is borrowed from
            // `self.ts_context`, which disposes it in `Drop for
            // ReplJitSession`. `Context::new` builds an *owning* wrapper
            // whose own Drop impl calls `LLVMContextDispose`, so it must
            // never run here, wrap it in `ManuallyDrop` immediately, before
            // any fallible call (`codegen_and_verify` below returns `Err` on
            // ordinary compiler bugs such as an IR-verification failure).
            // Without this, an early `?` return would dispose the context
            // out from under `ts_context`: a use-after-free the moment the
            // corrupted session compiles anything else, and a double free
            // when `ts_context` is later disposed for real.
            let context = std::mem::ManuallyDrop::new(Context::new(raw_context));
            let cleanup_name = format!("__tyrus_repl_cleanup_{}", self.next_cleanup_id);
            self.next_cleanup_id += 1;
            let new_globals = collect_new_globals(program, &self.known_symbols);
            let codegen = Codegen::new(&context, project_name, false, CompileTarget::Native);
            let module = codegen.codegen_and_verify_with_repl_cleanup(
                program,
                Some(&cleanup_name),
                &new_globals,
            )?;
            self.mark_known_symbols(&module);
            Codegen::<'_>::optimize_module(&module);
            let new_symbols = collect_new_symbols(&module);

            let ts_module = LLVMOrcCreateNewThreadSafeModule(module.as_mut_ptr(), self.ts_context);
            forget(module);
            if ts_module.is_null() {
                return Err("failed to create the JIT module".to_string());
            }

            let err = LLVMOrcLLJITAddLLVMIRModule(self.jit, self.main_dylib, ts_module);
            if !err.is_null() {
                let msg = orc_error_message(err);
                return Err(format!("error[jit]: failed to add the module: {msg}"));
            }

            self.known_symbols.extend(new_symbols);
            self.cleanup_symbols.push(cleanup_name);
        }
        Ok(())
    }

    fn mark_known_symbols(&self, module: &Module<'_>) {
        for function in module.get_functions() {
            if let Ok(name) = function.get_name().to_str()
                && self.known_symbols.contains(name)
            {
                function.set_linkage(Linkage::AvailableExternally);
            }
        }
        for global in module.get_globals() {
            if let Ok(name) = global.get_name().to_str()
                && self.known_symbols.contains(name)
            {
                global.set_linkage(Linkage::AvailableExternally);
            }
        }
    }

    fn run_step(&self, step_name: &str) -> Result<String, String> {
        let c_step =
            CString::new(step_name).map_err(|_| "error[jit]: invalid step name".to_string())?;
        let mut address: u64 = 0;
        let err = unsafe { LLVMOrcLLJITLookup(self.jit, &mut address, c_step.as_ptr()) };
        if !err.is_null() {
            let msg = orc_error_message(err);
            return Err(format!("error[jit]: step `{step_name}` not found: {msg}"));
        }
        if address == 0 {
            return Err(format!(
                "error[jit]: step `{step_name}` resolved to a null address"
            ));
        }
        capture_stdout(|| unsafe {
            let step: unsafe extern "C" fn() -> i32 = std::mem::transmute(address);
            step();
        })
    }
}

impl Drop for ReplJitSession {
    fn drop(&mut self) {
        unsafe {
            for cleanup_name in self.cleanup_symbols.iter().rev() {
                let Ok(cleanup_name) = CString::new(cleanup_name.as_str()) else {
                    continue;
                };
                let mut address: u64 = 0;
                let err = LLVMOrcLLJITLookup(self.jit, &mut address, cleanup_name.as_ptr());
                if !err.is_null() {
                    llvm_sys::error::LLVMConsumeError(err);
                    continue;
                }
                if address != 0 {
                    let cleanup: unsafe extern "C" fn() = std::mem::transmute(address);
                    cleanup();
                }
            }
            let err = LLVMOrcDisposeLLJIT(self.jit);
            if !err.is_null() {
                llvm_sys::error::LLVMConsumeError(err);
            }
            LLVMOrcDisposeThreadSafeContext(self.ts_context);
        }
    }
}

fn collect_new_globals(program: &Program, known_symbols: &HashSet<String>) -> Vec<String> {
    program
        .items
        .iter()
        .filter_map(|item| {
            let Item::Stmt(Stmt {
                kind: StmtKind::VarDecl { name, .. },
                ..
            }) = item
            else {
                return None;
            };
            (!known_symbols.contains(name)).then(|| name.clone())
        })
        .collect()
}

fn collect_new_symbols(module: &Module<'_>) -> Vec<String> {
    let mut names = Vec::new();
    for function in module.get_functions() {
        if function.count_basic_blocks() == 0
            || matches!(function.get_linkage(), Linkage::Internal | Linkage::Private)
        {
            continue;
        }
        if let Ok(name) = function.get_name().to_str() {
            names.push(name.to_string());
        }
    }
    for global in module.get_globals() {
        if matches!(global.get_linkage(), Linkage::Internal | Linkage::Private) {
            continue;
        }
        if let Ok(name) = global.get_name().to_str() {
            names.push(name.to_string());
        }
    }
    names
}

fn orc_error_message(err: LLVMErrorRef) -> String {
    unsafe {
        let message = LLVMGetErrorMessage(err);
        let text = std::ffi::CStr::from_ptr(message)
            .to_string_lossy()
            .into_owned();
        LLVMDisposeErrorMessage(message);
        text
    }
}
