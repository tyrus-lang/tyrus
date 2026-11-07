#[path = "src/platform/llvm_tool_path.rs"]
mod llvm_tool_path;
#[path = "build_support/runtime_exports.rs"]
mod runtime_exports;

/// The profile directories the staticlib could be in, in search order.
///
/// The cross-compiled directory comes first. Searching only the host one is how
/// this went wrong once: `scripts/sanitizers` passes `--target`, found no
/// staticlib there, silently skipped the whole extraction, and left the JIT with
/// no runtime symbols to resolve, every call became a call to address zero. It
/// looked green for as long as a stale host-path staticlib from an earlier build
/// happened to be lying around to be found.
fn staticlib_roots() -> Vec<std::path::PathBuf> {
    let Ok(out_dir) = std::env::var("OUT_DIR") else {
        return Vec::new();
    };
    let out_dir = std::path::PathBuf::from(out_dir);
    // `OUT_DIR` is `<profile>/build/<pkg>-<hash>/out`, so the profile directory
    // is three levels up. A build script always runs on the host, so under an
    // explicit `--target` this is the *host* profile directory while the
    // artifacts the link needs live beside it under the target triple.
    let Some(profile_dir) = out_dir
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
    else {
        return Vec::new();
    };

    let mut roots = Vec::new();
    if let Ok(triple) = std::env::var("TARGET")
        && let (Some(root), Some(profile)) = (profile_dir.parent(), profile_dir.file_name())
    {
        roots.push(root.join(triple).join(profile));
    }
    roots.push(profile_dir.to_path_buf());
    roots
}

fn find_staticlib() -> Option<std::path::PathBuf> {
    staticlib_roots()
        .into_iter()
        .find_map(|root| staticlib_in(&root))
}

fn target_is_msvc() -> bool {
    std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|environment| environment == "msvc")
}

fn target_runtime_archive_names() -> [&'static str; 2] {
    if target_is_msvc() {
        ["tyrus_runtime.lib", ""]
    } else {
        ["libtyrus_runtime.a", ""]
    }
}

fn target_hashed_runtime_archive_prefix() -> &'static str {
    if target_is_msvc() {
        "tyrus_runtime-"
    } else {
        "libtyrus_runtime-"
    }
}

fn target_hashed_runtime_archive_extension() -> &'static str {
    if target_is_msvc() { ".lib" } else { ".a" }
}

fn llvm_binary(bin: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    [name, &format!("{name}.exe")]
        .into_iter()
        .map(|candidate| bin.join(candidate))
        .find(|candidate| candidate.is_file())
}

/// The canonical staticlib in `root`, or the newest hashed one Cargo left in
/// its `deps` directory.
fn staticlib_in(root: &std::path::Path) -> Option<std::path::PathBuf> {
    for name in target_runtime_archive_names() {
        if name.is_empty() {
            continue;
        }
        let candidate = root.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let prefix = target_hashed_runtime_archive_prefix();
    let extension = target_hashed_runtime_archive_extension();
    let mut candidates: Vec<_> = std::fs::read_dir(root.join("deps"))
        .ok()?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(prefix) && name.ends_with(extension))
        })
        .collect();
    candidates.sort_by_key(|path| {
        std::fs::metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
    });
    candidates.pop()
}

fn llvm_tool(name: &str) -> Option<std::path::PathBuf> {
    if let Some(candidate) = llvm_tool_path::prefixed_tool(
        std::env::var_os("LLVM_SYS_231_PREFIX").as_deref(),
        name,
        cfg!(windows),
    ) {
        return Some(candidate);
    }
    for candidate in [name, &format!("{name}.exe")] {
        if std::process::Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            return Some(candidate.into());
        }
    }
    None
}

fn runtime_abi_symbols(archive: &std::path::Path) -> Result<Vec<String>, String> {
    let nm = llvm_tool("llvm-nm")
        .or_else(|| llvm_tool("nm"))
        .ok_or_else(|| {
            llvm_tool_path::missing_tool_message("llvm-nm", "export JIT runtime symbols")
        })?;
    let output = std::process::Command::new(&nm)
        .args(["--extern-only", "--defined-only", "--format=posix"])
        .arg(archive)
        .output()
        .map_err(|error| {
            format!(
                "failed to inspect runtime symbols with `{}`: {error}",
                nm.display()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "`{}` failed while inspecting runtime symbols",
            nm.display()
        ));
    }
    let symbols = runtime_exports::parse_symbols(&String::from_utf8_lossy(&output.stdout));
    if symbols.is_empty() {
        return Err("no exported `tyrus_*` runtime symbols were found for the JIT".to_string());
    }
    Ok(symbols)
}

fn write_runtime_exports(
    out_dir: &std::path::Path,
    symbols: &[String],
) -> Result<Option<std::path::PathBuf>, String> {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "linux" && target_os != "macos" && !target_is_msvc() {
        return Ok(None);
    }
    let (file_name, contents) = if target_is_msvc() {
        let entries = symbols.join("\n");
        ("runtime_exports.def", format!("EXPORTS\n{entries}\n"))
    } else if target_os == "linux" {
        let entries = symbols
            .iter()
            .map(|symbol| format!("    {symbol};"))
            .collect::<Vec<_>>()
            .join("\n");
        ("runtime_exports.list", format!("{{\n{entries}\n}};\n"))
    } else {
        ("runtime_exports.list", format!("{}\n", symbols.join("\n")))
    };
    let path = out_dir.join(file_name);
    std::fs::write(&path, contents)
        .map_err(|error| format!("failed to write runtime export list: {error}"))?;
    Ok(Some(path))
}

fn emit_runtime_link_args(symbols: &[String], out_dir: &std::path::Path) {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_is_msvc() {
        if let Some(exports) = write_runtime_exports(out_dir, symbols)
            .unwrap_or_else(|error| panic!("failed to prepare JIT runtime exports: {error}"))
        {
            println!("cargo:rustc-link-arg-bins=/DEF:{}", exports.display());
            println!("cargo:rustc-link-arg-tests=/DEF:{}", exports.display());
        }
    } else if target_os == "macos" {
        // Retain the C ABI from Cargo's rlibs without importing staticlib allocator shims.
        for symbol in symbols {
            for target in ["bins", "tests"] {
                println!("cargo:rustc-link-arg-{target}=-Wl,-u,{symbol}");
            }
        }
        if let Some(exports) = write_runtime_exports(out_dir, symbols)
            .unwrap_or_else(|error| panic!("failed to prepare JIT runtime exports: {error}"))
        {
            for target in ["bins", "tests"] {
                println!("cargo:rustc-link-arg-{target}=-Xlinker");
                println!("cargo:rustc-link-arg-{target}=-exported_symbols_list");
                println!("cargo:rustc-link-arg-{target}=-Xlinker");
                println!("cargo:rustc-link-arg-{target}={}", exports.display());
            }
        }
    } else {
        println!("cargo:rustc-link-arg-bins=-Wl,--allow-multiple-definition");
        println!("cargo:rustc-link-arg-tests=-Wl,--allow-multiple-definition");
        if target_os == "linux" {
            if let Some(exports) = write_runtime_exports(out_dir, symbols)
                .unwrap_or_else(|error| panic!("failed to prepare JIT runtime exports: {error}"))
            {
                for target in ["bins", "tests"] {
                    println!("cargo:rustc-link-arg-{target}=-Xlinker");
                    println!("cargo:rustc-link-arg-{target}=--dynamic-list");
                    println!("cargo:rustc-link-arg-{target}=-Xlinker");
                    println!("cargo:rustc-link-arg-{target}={}", exports.display());
                }
            }
        } else if target_os == "windows" {
            println!("cargo:rustc-link-arg-bins=-Wl,--export-all-symbols");
            println!("cargo:rustc-link-arg-tests=-Wl,--export-all-symbols");
        }
    }
}

fn macos_runtime_native_link_args() -> Vec<String> {
    let Some(manifest) = staticlib_roots()
        .into_iter()
        .map(|root| root.join("tyrus-native-libs.json"))
        .find(|path| path.is_file())
    else {
        panic!(
            "macOS JIT runtime objects require tyrus-native-libs.json; build the runtime native link manifest before the CLI"
        );
    };
    println!("cargo:rerun-if-changed={}", manifest.display());
    let contents = std::fs::read_to_string(&manifest)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", manifest.display()));
    let value: serde_json::Value = serde_json::from_str(&contents)
        .unwrap_or_else(|error| panic!("invalid {}: {error}", manifest.display()));
    let target = std::env::var("TARGET").expect("Cargo must provide the compilation target");
    assert_eq!(
        value["target"].as_str(),
        Some(target.as_str()),
        "native link manifest target does not match the Cargo target"
    );
    value["arguments"]
        .as_array()
        .expect("native link manifest must contain an arguments array")
        .iter()
        .map(|argument| {
            argument
                .as_str()
                .expect("native link arguments must be strings")
                .to_string()
        })
        .collect()
}

fn main() {
    println!("cargo:rerun-if-changed=build_support/runtime_exports.rs");
    let target = std::env::var("TARGET").expect("Cargo must provide the compilation target");
    println!("cargo:rustc-env=TYRUS_TARGET_TRIPLE={target}");

    // Watching the runtime's *sources*, not only the staticlib built from them.
    //
    // The staticlib is a build artifact of another crate, and the fingerprint
    // Cargo compares it against is the one recorded on the previous run, so on
    // the run that first produces a new staticlib, this script does not re-run
    // at all, and the objects extracted below stay a generation behind. That is
    // invisible until the generation gap adds a symbol: JIT process-symbol
    // lookup cannot find it, and generated code can fail when it first reaches
    // that runtime function. Watching the sources closes the gap, because a
    // runtime change always touches one of them.
    println!("cargo:rerun-if-changed=crates/tyrus-runtime/src");

    // The JIT engines resolve runtime calls through host-process symbol lookup,
    // but the tyrus-runtime C ABI functions (tyrus_*) are not referenced by
    // Rust code, so the linker may discard them from the rlib. Inspect the
    // staticlib members to export the C ABI. Mach-O retains those symbols as
    // undefined roots and loads archive members lazily to avoid duplicate dependencies.
    if let Some(ref staticlib) = find_staticlib() {
        // Belt and braces with the source watch above: this catches a
        // staticlib that changed without its sources changing, such as a
        // rebuild under different flags.
        println!("cargo:rerun-if-changed={}", staticlib.display());
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let symbols = runtime_abi_symbols(staticlib)
            .unwrap_or_else(|error| panic!("failed to inspect JIT runtime symbols: {error}"));
        emit_runtime_link_args(&symbols, std::path::Path::new(&out_dir));
        if target_is_msvc() || std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
            if !target_is_msvc() {
                for argument in macos_runtime_native_link_args() {
                    println!("cargo:rustc-link-arg-bins={argument}");
                    println!("cargo:rustc-link-arg-tests={argument}");
                }
                for target in ["bins", "tests"] {
                    println!("cargo:rustc-link-arg-{target}=-lc++");
                }
            }
            println!("cargo:warning=retained JIT runtime symbols from Cargo runtime rlibs");
            record_toolchain_clang();
            return;
        }
        let extract_dir = std::path::Path::new(&out_dir).join("runtime_objects");
        // Emptied first, because `ar x` unpacks over whatever is already here
        // rather than replacing it. Object file names carry a per-compilation
        // hash, so a rebuilt runtime writes *new* names beside the old ones and
        // leaves both linked -- and with `--allow-multiple-definition` the
        // stale definition can be the one that wins. That is a JIT calling
        // yesterday's version of a runtime function, which reads its arguments
        // under the old signature: not a link error, a crash somewhere else
        // entirely.
        let _ = std::fs::remove_dir_all(&extract_dir);
        std::fs::create_dir_all(&extract_dir).unwrap();
        let archive_tool = llvm_tool("llvm-ar").unwrap_or_else(|| {
            if target_is_msvc() {
                panic!(
                    "{}",
                    llvm_tool_path::missing_tool_message(
                        "llvm-ar",
                        "extract the runtime archive for MSVC"
                    )
                )
            }
            "ar".into()
        });
        let status = std::process::Command::new(&archive_tool)
            .args(["x", &staticlib.to_string_lossy()])
            .current_dir(&extract_dir)
            .status()
            .unwrap_or_else(|error| {
                panic!(
                    "failed to start archiver `{}` while extracting `{}`: {error}; install a compatible archiver and set `LLVM_SYS_231_PREFIX` or update `PATH`",
                    archive_tool.display(),
                    staticlib.display()
                )
            });
        if !status.success() {
            panic!(
                "archiver `{}` failed with {} while extracting `{}`; check the archiver diagnostics above and verify that it supports the archive format",
                archive_tool.display(),
                status,
                staticlib.display()
            );
        }
        {
            let mut o_count = 0u32;
            let mut entries: Vec<_> = std::fs::read_dir(&extract_dir)
                .unwrap()
                .filter_map(std::result::Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|e| e == "o" || e == "obj"))
                .collect();
            entries.sort();
            for o in &entries {
                println!("cargo:rustc-link-arg-bins={}", o.display());
                println!("cargo:rustc-link-arg-tests={}", o.display());
                o_count += 1;
            }
            println!("cargo:warning=linked {o_count} runtime objects for JIT symbol resolution");
        }
    } else {
        // Watch the places the staticlib *would* be, so that the run which
        // finally produces it re-runs this script.
        //
        // Without this, not finding the staticlib is a state the build gets
        // stuck in. `cargo check` does not build a staticlib, but it does run
        // this script and record its output; the next `cargo test` sees a
        // fingerprint that is still fresh, the runtime sources have not
        // changed, reuses that output, and links a test binary with no runtime
        // objects at all. Every `tyrus_*` symbol then resolves to null and the
        // first JIT-compiled call jumps to address zero. That is how the whole
        // `--bin tyrus` suite came to die of SIGSEGV in `jit_basic_expression`
        // after a routine `cargo check --workspace --all-targets`, and why it
        // came back to life after an unrelated edit happened to touch this file.
        //
        // Cargo accepts `rerun-if-changed` for a path that does not exist yet
        // and re-runs when it appears, which is exactly the event that matters.
        for root in staticlib_roots() {
            println!(
                "cargo:rerun-if-changed={}",
                root.join(target_runtime_archive_names()[0]).display()
            );
            println!("cargo:rerun-if-changed={}", root.join("deps").display());
        }
        // Loudly, because the failure this produces is silent and distant: with
        // no objects linked, the JIT resolves every `tyrus_*` symbol to null
        // and the first generated call jumps to address zero, in a test whose
        // program may not go anywhere near the feature that is missing. It cost
        // a full investigation once, under `scripts/sanitizers`, which passes
        // `--target` and so looked in a directory this search did not know
        // about. A warning here would have pointed at it immediately.
        println!(
            "cargo:warning=no tyrus-runtime staticlib found: JIT-compiled code will resolve every \
             runtime symbol to null. Build it with `cargo build -p tyrus-runtime`."
        );
    }
    record_toolchain_clang();
}

/// Bake in the `clang` that sits beside the LLVM this compiler was built
/// against, so the binary it produces is linked by a matching version.
///
/// The generated `.ll` is written by the LLVM `inkwell` is linked to, and an
/// older clang cannot be relied on to read a newer LLVM's textual IR. Searching
/// `PATH` for `clang-<n>` does not settle it either: a build against an LLVM
/// installed under its own prefix, the ordinary case for a release tarball,
/// has no matching `clang` on `PATH` at all, and putting one there would
/// capture every other project's `clang` calls too.
///
/// Absent when the prefix is unset, which is the distribution-package case:
/// `find_clang` then falls back to searching, as it always did.
fn record_toolchain_clang() {
    println!("cargo:rerun-if-env-changed=LLVM_SYS_231_PREFIX");
    let Ok(prefix) = std::env::var("LLVM_SYS_231_PREFIX") else {
        return;
    };
    let bin = std::path::PathBuf::from(prefix).join("bin");
    if let Some(clang) = llvm_binary(&bin, "clang") {
        println!("cargo:rustc-env=TYRUS_TOOLCHAIN_CLANG={}", clang.display());
    }
    // The WebAssembly path also consumes textual LLVM IR. Keep `llc` paired
    // with the inkwell LLVM exactly as `clang` is for native builds: an older
    // system `llc` can reject valid newer IR before the playground program
    // reaches its runtime or linker.
    if let Some(llc) = llvm_binary(&bin, "llc") {
        println!("cargo:rustc-env=TYRUS_TOOLCHAIN_LLC={}", llc.display());
    }
    if let Some(wasm_ld) = llvm_binary(&bin, "wasm-ld") {
        println!(
            "cargo:rustc-env=TYRUS_TOOLCHAIN_WASM_LD={}",
            wasm_ld.display()
        );
    }
    // `ld.lld` from the same toolchain, which is what makes link-time
    // optimization across the generated code and the runtime possible: it reads
    // the bitcode the runtime staticlib carries natively, where the system
    // linker would need a plugin. Recorded rather than searched for the same
    // reason as `clang`, a release tarball puts it under its own prefix, and
    // it is deliberately not on `PATH`.
    let lld_name = match (
        std::env::var("CARGO_CFG_TARGET_OS").as_deref(),
        std::env::var("CARGO_CFG_TARGET_ENV").as_deref(),
    ) {
        (Ok("macos"), _) => "ld64.lld",
        (_, Ok("msvc")) => "lld-link",
        _ => "ld.lld",
    };
    if let Some(lld) = llvm_binary(&bin, lld_name) {
        println!("cargo:rustc-env=TYRUS_TOOLCHAIN_LLD={}", lld.display());
    }
}
