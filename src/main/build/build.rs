use super::{codegen, compile, sealed_env};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use tyrus_core::{env as tyrus_env, module_loader, project};

pub(super) fn parse_pgo_mode(args: &[String], release: bool) -> compile::PgoMode {
    match pgo_mode_from(args, release) {
        Ok(mode) => mode,
        Err(message) => {
            eprintln!("{message}");
            process::exit(1);
        }
    }
}

pub(super) fn pgo_mode_from(args: &[String], release: bool) -> Result<compile::PgoMode, String> {
    let generate = args.iter().any(|arg| arg == "--pgo-generate");
    let use_profile = args.iter().any(|arg| arg == "--pgo-use");
    if generate && use_profile {
        return Err("error: `--pgo-generate` and `--pgo-use` are the two halves of the same workflow; pass one at a time".to_string());
    }
    if (generate || use_profile) && !release {
        return Err("error: profile-guided optimization needs `--release`\nnote: an instrumented `-O0` build profiles code the optimizer never sees".to_string());
    }
    Ok(match (generate, use_profile) {
        (true, _) => compile::PgoMode::Generate,
        (_, true) => compile::PgoMode::Use,
        _ => compile::PgoMode::Off,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn cmd_build(
    dir: &Path,
    release: bool,
    rotate_seal_key: bool,
    test_mode: bool,
    entry_path: Option<&Path>,
    compile_target: codegen::CompileTarget,
    sandbox: bool,
    pgo: compile::PgoMode,
) -> PathBuf {
    let config = project::load_project(dir).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });

    let project_name = &config.project.name;
    let entry = entry_path.map_or_else(|| dir.join("src/main.tyr"), |path| dir.join(path));

    if !entry.exists() {
        eprintln!("error: entry file `{}` not found", entry.display());
        process::exit(1);
    }

    let optimized = release || config.build.optimize;
    let profile = if optimized { "release" } else { "debug" };
    eprintln!(
        "  Compiling {} v{} ({profile})",
        project_name, config.project.version
    );

    let env_schema = if sandbox {
        None
    } else {
        let raw = tyrus_env::load_env_schema(dir, &config).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            process::exit(1);
        });
        match raw {
            Some((schema, diagnostics)) => {
                if let Err(e) = tyrus_env::sync_env_example(&schema) {
                    eprintln!(
                        "warning: could not write `{}`: {e}",
                        tyrus_env::example_path(&schema.file_path).display()
                    );
                }
                if !diagnostics.is_empty() {
                    let rendered = diagnostics
                        .iter()
                        .map(|diagnostic| {
                            tyrus_env::render_env_diagnostic(
                                &schema.file_path,
                                &schema.source,
                                &schema.vars,
                                diagnostic,
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    eprintln!("{rendered}");
                    process::exit(1);
                }
                Some(schema)
            }
            None => None,
        }
    };

    let env_sealed = config
        .security
        .as_ref()
        .is_some_and(|security| security.env_sealed);

    let worker_threads = config.runtime.worker_threads;
    if worker_threads == Some(0) {
        eprintln!("error: `runtime.workerThreads` must be at least 1");
        process::exit(1);
    }
    if rotate_seal_key && !env_sealed {
        eprintln!("error: `--rotate-seal-key` requires `security.envSealed` to be true");
        process::exit(1);
    }

    let restriction = if sandbox {
        module_loader::ModuleRestriction::Sandbox
    } else if matches!(compile_target, codegen::CompileTarget::Wasm) {
        module_loader::ModuleRestriction::WasmTarget
    } else {
        module_loader::ModuleRestriction::None
    };
    let program = module_loader::load_program_sandboxed(&entry, env_schema.as_ref(), restriction)
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            process::exit(1);
        });

    let uses_test_framework = tyrus_core::test::program_uses_test_framework(&program);
    if !test_mode && uses_test_framework {
        eprintln!(
            "error: `{}` uses `tyrus:test`, which only `tyrus test` can run\nhelp: move the tests into a `*.test.tyr` file, for example under `__tests__/`, and run `tyrus test`",
            entry.display()
        );
        process::exit(1);
    }

    let target_dir = dir.join("target");
    fs::create_dir_all(&target_dir).unwrap();
    let sealed_env = if env_sealed {
        let Some(schema) = env_schema.as_ref() else {
            eprintln!("error: `security.envSealed` requires `security.envFile` to be configured");
            process::exit(1);
        };
        let seal_artifact_dir = target_dir.join(profile);
        fs::create_dir_all(&seal_artifact_dir).unwrap();
        let sealed = sealed_env::prepare(
            project_name,
            &schema.source,
            &seal_artifact_dir,
            rotate_seal_key,
        )
        .unwrap_or_else(|e| {
            eprintln!("error: {e}");
            process::exit(1);
        });
        eprintln!("  Sealed env → {}", sealed.sealed_path.display());
        if sealed.generated_key {
            eprintln!("  Generated seal key → {}", sealed.key_path.display());
        } else {
            eprintln!("  Reusing seal key → {}", sealed.key_path.display());
        }
        eprintln!("  Public seal key → {}", sealed.public_key_path.display());
        Some(codegen::SealedEnvMetadata {
            project_name: project_name.clone(),
            fingerprint: sealed.fingerprint,
        })
    } else {
        None
    };

    // A test binary never overwrites the program's own binary: `tyrus build`
    // writes `target/<profile>/<project>`, while a test file compiles below
    // `target/test/files/<file stem>`. Sharing the path meant a `tyrus test` run
    // silently replaced the artifact that profiling and leak checks measure.
    let (artifact_dir, artifact_stem) = if test_mode {
        let relative = entry.strip_prefix(dir).unwrap_or(entry.as_path());
        let stem = relative
            .file_name()
            .and_then(|name| name.to_str())
            .map_or(project_name.as_str(), |name| {
                name.trim_end_matches(".tyr").trim_end_matches(".test")
            })
            .to_string();
        let test_dir = match relative.parent() {
            Some(parent) => target_dir.join("test/files").join(parent),
            None => target_dir.join("test/files"),
        };
        fs::create_dir_all(&test_dir).unwrap();
        (test_dir, stem)
    } else {
        let profile_dir = target_dir.join(profile);
        fs::create_dir_all(&profile_dir).unwrap();
        (profile_dir, project_name.clone())
    };

    let ll_stem = artifact_dir.join(&artifact_stem);
    let ll_path = format!("{}.ll", ll_stem.display());

    let database_target = config
        .database
        .as_ref()
        .map(|database| database.runtime_target(dir))
        .transpose()
        .unwrap_or_else(|error| {
            eprintln!("error: {error}");
            process::exit(1);
        });
    codegen::compile_with_runtime_options(
        &program,
        project_name,
        &ll_stem.to_string_lossy(),
        optimized,
        env_schema.as_ref(),
        sealed_env.as_ref(),
        compile_target,
        worker_threads,
        config.development.https,
        database_target.as_deref(),
    )
    .unwrap_or_else(|e| {
        eprintln!("error[codegen]: {e}");
        process::exit(1);
    });

    let output_path = artifact_dir.join(&artifact_stem);
    let binary_path = if compile_target == codegen::CompileTarget::Native {
        crate::platform::executable_path(&output_path)
    } else {
        output_path
    };

    match compile_target {
        codegen::CompileTarget::Native => {
            let clang = find_clang();
            let mut clang_command = process::Command::new(&clang);
            clang_command.arg(&ll_path);
            if optimized {
                compile::configure_release_debug_stripping(&mut clang_command);
            }
            compile::configure_clang_optimization(&mut clang_command, &clang, optimized);
            let Some(runtime) = find_runtime_library(optimized) else {
                eprintln!(
                    "{}",
                    crate::platform::missing_runtime_library_error(optimized)
                );
                process::exit(1);
            };
            clang_command.arg(runtime);
            if let Err(message) =
                crate::platform::configure_runtime_system_libraries(&mut clang_command, &clang)
            {
                eprintln!("error: {message}");
                process::exit(1);
            }
            compile::configure_sanitizer_linking(&mut clang_command, &clang);
            if let Err(message) =
                compile::configure_pgo(&mut clang_command, pgo, &target_dir, &clang)
            {
                eprintln!("{message}");
                process::exit(1);
            }
            crate::platform::configure_clang_output(
                &mut clang_command,
                &clang,
                binary_path.as_os_str(),
            );
            compile::configure_dead_code_stripping(&mut clang_command, &clang);
            if let Err(message) = crate::platform::run_clang_link(&mut clang_command, &clang) {
                eprintln!("{message}");
                process::exit(1);
            }
        }
        codegen::CompileTarget::Wasm => {
            let obj_path = format!("{}.o", ll_stem.display());
            let wasm_path = target_dir.join(format!("{project_name}.wasm"));

            let llc = find_llc();
            let mut llc_command = process::Command::new(&llc);
            llc_command.args(["--mtriple=wasm32-unknown-unknown", "--filetype=obj"]);
            llc_command.arg(&ll_path);
            llc_command.args(["-o", &obj_path]);
            if optimized {
                llc_command.args(["-O3"]);
            }
            let status = llc_command.status().unwrap_or_else(|error| {
                eprintln!(
                    "{}",
                    llvm_tool_start_error("LLVM code generator", &llc, "TYRUS_LLC", &error)
                );
                process::exit(1);
            });
            if !status.success() {
                eprintln!("error: llc failed to compile IR to WASM object");
                process::exit(1);
            }

            let wasm_ld = find_wasm_ld();
            let mut ld_command = process::Command::new(&wasm_ld);
            ld_command.arg(&obj_path);
            ld_command.arg("--no-entry");
            // Export only what the host actually calls, and drop everything the
            // program does not reach. `--export-dynamic` exports every symbol in
            // the runtime, which both publishes the whole internal C ABI to the
            // page and pins every function against garbage collection.
            ld_command.args([
                "--export=main",
                "--export=tyrus_playground_stdout_ptr",
                "--export=tyrus_playground_stdout_len",
                "--export=tyrus_playground_stderr_ptr",
                "--export=tyrus_playground_stderr_len",
                "--export=tyrus_playground_clear",
                "--gc-sections",
                "--strip-all",
            ]);
            ld_command.args([
                "--import-memory",
                "--initial-memory=1048576",
                "--max-memory=8388608",
            ]);
            ld_command.arg("--fatal-warnings");
            let undefined_path = target_dir.join("wasm-host-imports.txt");
            if fs::write(&undefined_path, "tyrus_host_now_millis\n").is_ok() {
                ld_command.arg(format!(
                    "--allow-undefined-file={}",
                    undefined_path.to_string_lossy()
                ));
            }
            // Link against the WASM runtime staticlib if available.
            if let Some(wasm_runtime) = find_wasm_runtime_library() {
                ld_command.arg(&wasm_runtime);
            }
            let wasm_arg = wasm_path.to_string_lossy().to_string();
            ld_command.args(["-o", &wasm_arg]);
            let status = ld_command.status().unwrap_or_else(|error| {
                eprintln!(
                    "{}",
                    llvm_tool_start_error("WebAssembly linker", &wasm_ld, "TYRUS_WASM_LD", &error)
                );
                process::exit(1);
            });
            if !status.success() {
                eprintln!("error: wasm-ld failed to link WASM object");
                process::exit(1);
            }

            let _ = fs::remove_file(&obj_path);

            return wasm_path;
        }
    }

    eprintln!("  Finished → {}", binary_path.display());
    binary_path
}

pub(super) fn parse_run_args(
    args: &[String],
) -> Result<(Option<PathBuf>, bool, bool, Vec<String>), String> {
    let separator = args.iter().position(|arg| arg == "--");
    let compiler_args = &args[..separator.unwrap_or(args.len())];
    let forwarded = separator
        .map(|index| args[index + 1..].to_vec())
        .unwrap_or_default();
    let mut entry = None;
    let mut release = false;
    let mut rotate_seal_key = false;

    let mut skip_next = false;
    for arg in compiler_args {
        if skip_next {
            skip_next = false;
            continue;
        }
        match arg.as_str() {
            "--release" => release = true,
            "--rotate-seal-key" => rotate_seal_key = true,
            "--compile-target" => {
                skip_next = true;
            }
            "--sandbox" => {}
            _ if arg.starts_with('-') => return Err(format!("unknown option `{arg}`")),
            _ if entry.is_some() => {
                return Err("only one standalone entry file may be specified".to_string());
            }
            _ => entry = Some(PathBuf::from(arg)),
        }
    }

    Ok((entry, release, rotate_seal_key, forwarded))
}

/// The runtime staticlib to link, preferring one built with the same
/// optimization profile the user asked for.
///
/// Cross-language LTO is intentionally disabled until the Rust and Clang
/// toolchains are verified to exchange compatible bitcode on every target.
/// Release builds use the optimized native archive from the sibling
/// `target/release` profile when the compiler runs from a development checkout.
///
/// Measured on a loop of 300 000 awaited async calls: 4,63 s against 0,40 s,
/// **11,6×**, from that one difference. Every benchmark run out of a
/// development checkout was measuring the debug runtime.
pub(super) fn find_runtime_library(optimized: bool) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("TYRUS_RUNTIME_LIB").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    let executable = std::env::current_exe().ok()?;
    let executable_dir = executable.parent()?;
    let candidates = runtime_library_candidates(executable_dir, optimized);
    let chosen = candidates
        .into_iter()
        .find(|path| path.is_file())
        .or_else(|| find_development_runtime_library(executable_dir))?;
    if optimized && runtime_library_is_unoptimized(&chosen) {
        eprintln!(
            "  note: linking an unoptimized runtime ({}); run `cargo build --release -p tyrus-runtime` for a representative release build",
            chosen.display()
        );
    }
    Some(chosen)
}

pub(super) fn runtime_library_candidates(executable_dir: &Path, optimized: bool) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if optimized && let Some(profiles) = executable_dir.parent() {
        candidates.extend(crate::platform::runtime_library_candidates(
            &profiles.join("release"),
            "tyrus_runtime",
        ));
    }
    candidates.extend(crate::platform::runtime_library_candidates(
        executable_dir,
        "tyrus_runtime",
    ));
    candidates.extend(crate::platform::runtime_library_candidates(
        &executable_dir.join("../lib/tyrus"),
        "tyrus_runtime",
    ));
    candidates
}

/// Whether the staticlib sits directly in a Cargo `debug/` profile directory.
///
/// Path-shaped on purpose: the archive itself carries no profile marker. Only
/// the directory holding it is read, never the whole path, an ancestor may be
/// named `debug` for unrelated reasons. An installed compiler never reaches
/// here anyway: its staticlib answers as the first candidate, from a directory
/// named neither `debug` nor `release`.
pub(super) fn runtime_library_is_unoptimized(path: &Path) -> bool {
    path.parent()
        .and_then(Path::file_name)
        .is_some_and(|directory| directory == "debug")
}

pub(super) fn find_development_runtime_library(executable_dir: &Path) -> Option<PathBuf> {
    let deps = executable_dir.join("deps");
    let mut candidates = std::fs::read_dir(deps)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| crate::platform::is_hashed_runtime_library(path, "tyrus_runtime"))
        .filter_map(|path| {
            let modified = path.metadata().ok()?.modified().ok()?;
            Some((modified, path))
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    candidates.pop().map(|(_, path)| path)
}

pub(super) fn find_clang() -> String {
    crate::platform::find_llvm_tool(
        std::env::var_os("TYRUS_CLANG"),
        option_env!("TYRUS_TOOLCHAIN_CLANG"),
        &[
            "clang-23", "clang-22", "clang-21", "clang-20", "clang-19", "clang-18", "clang",
        ],
    )
}

pub(super) fn find_llc() -> String {
    crate::platform::find_llvm_tool(
        std::env::var_os("TYRUS_LLC"),
        option_env!("TYRUS_TOOLCHAIN_LLC"),
        &[
            "llc-23", "llc-22", "llc-21", "llc-20", "llc-19", "llc-18", "llc",
        ],
    )
}

pub(super) fn find_wasm_ld() -> String {
    crate::platform::find_llvm_tool(
        std::env::var_os("TYRUS_WASM_LD"),
        option_env!("TYRUS_TOOLCHAIN_WASM_LD"),
        &[
            "wasm-ld-23",
            "wasm-ld-22",
            "wasm-ld-17",
            "wasm-ld-16",
            "wasm-ld",
        ],
    )
}

/// Find the WASM runtime staticlib for linking.
pub(super) fn find_wasm_runtime_library() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("TYRUS_WASM_RUNTIME_LIB").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    let executable = std::env::current_exe().ok()?;
    let executable_dir = executable.parent()?;
    let target_dir = executable_dir.parent()?;
    let candidates = [
        target_dir.join("wasm32-unknown-unknown/release/libtyrus_runtime_wasm.a"),
        target_dir.join("wasm32-unknown-unknown/debug/libtyrus_runtime_wasm.a"),
    ];
    candidates.into_iter().find(|path| path.is_file())
}

fn llvm_tool_start_error(
    component: &str,
    executable: &str,
    override_name: &str,
    error: &std::io::Error,
) -> String {
    format!(
        "error: could not start {component} `{executable}` ({error}); install LLVM 23.1.1, set `LLVM_SYS_231_PREFIX` to its installation directory or `{override_name}` to the executable path"
    )
}

#[cfg(test)]
mod tests {
    use super::llvm_tool_start_error;

    #[test]
    fn llvm_tool_start_errors_name_the_platform_neutral_override() {
        let error = std::io::Error::from(std::io::ErrorKind::NotFound);
        let llc = llvm_tool_start_error("LLVM code generator", "llc", "TYRUS_LLC", &error);
        assert!(llc.contains("LLVM code generator `llc`"));
        assert!(llc.contains("`TYRUS_LLC`"));
        assert!(!llc.contains("apt"));

        let linker =
            llvm_tool_start_error("WebAssembly linker", "wasm-ld", "TYRUS_WASM_LD", &error);
        assert!(linker.contains("WebAssembly linker `wasm-ld`"));
        assert!(linker.contains("`TYRUS_WASM_LD`"));
    }
}
