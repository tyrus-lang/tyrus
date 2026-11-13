#[path = "build_support/windows_llvm.rs"]
mod windows_llvm;

use std::path::PathBuf;
use std::process::Command;

fn llvm_config(executable: &std::path::Path, arguments: &[&str]) -> String {
    let output = Command::new(executable)
        .args(arguments)
        .output()
        .unwrap_or_else(|error| panic!("could not run {}: {error}", executable.display()));
    assert!(
        output.status.success(),
        "{} {} failed: {}",
        executable.display(),
        arguments.join(" "),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("llvm-config output must be UTF-8")
}

fn main() {
    println!("cargo:rerun-if-changed=build_support/windows_llvm.rs");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_231_PREFIX");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_231_USE_DEBUG_MSVCRT");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_231_FFI_WORKAROUND");
    println!("cargo:rerun-if-env-changed=LIB");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let executable = std::env::var_os("LLVM_SYS_231_PREFIX").map_or_else(
        || PathBuf::from("llvm-config.exe"),
        |prefix| PathBuf::from(prefix).join("bin/llvm-config.exe"),
    );
    let version = llvm_config(&executable, &["--version"]);
    assert!(
        version.trim().starts_with("23.1."),
        "expected LLVM 23.1.x, found {}",
        version.trim()
    );
    let directory = PathBuf::from(llvm_config(&executable, &["--libdir"]).trim());
    let mut search_directories = vec![directory];
    if let Some(paths) = std::env::var_os("LIB") {
        search_directories.extend(std::env::split_paths(&paths));
    }
    let static_crt = std::env::var("CARGO_CFG_TARGET_FEATURE")
        .unwrap_or_default()
        .split(',')
        .any(|feature| feature == "crt-static");
    for (option, kind) in [
        ("--libnames", "static"),
        ("--system-libs", if static_crt { "static" } else { "dylib" }),
    ] {
        let output = llvm_config(&executable, &[option, "--link-static"]);
        let files = windows_llvm::library_files(&output)
            .unwrap_or_else(|error| panic!("invalid llvm-config {option} output: {error}"));
        for file in files {
            let library = windows_llvm::resolve_library(&file, &search_directories)
                .unwrap_or_else(|error| panic!("{error}"));
            println!(
                "cargo:rustc-link-search=native={}",
                library.directory.display()
            );
            println!("cargo:rustc-link-lib={kind}={}", library.name);
        }
    }
    if std::env::var_os("LLVM_SYS_231_USE_DEBUG_MSVCRT").is_some()
        || llvm_config(&executable, &["--build-mode"]).contains("Debug")
    {
        println!("cargo:rustc-link-lib=msvcrtd");
    }
    if std::env::var_os("LLVM_SYS_231_FFI_WORKAROUND").is_some() {
        println!("cargo:rustc-link-lib=dylib=ffi");
    }
}
