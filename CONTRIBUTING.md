# Contributing to Tyrus

## Architecture and project layout

The compiler pipeline processes source through lexing, parsing, semantic analysis, ownership checking, LLVM IR generation, runtime linkage, and native binary generation. The CLI and Language Server share `tyrus-core` semantic analysis; editor clients use `tyrus lsp` and must not duplicate language rules.

- `crates/tyrus-core`: lexer, parser, AST, semantic analysis, module loading, and borrow checking.
- `crates/tyrus-codegen`: LLVM IR generation and native code linkage.
- `crates/tyrus-runtime`: native runtime linked into generated binaries.
- `crates/tyrus-lsp`: Language Server implementation.
- `src`: CLI and component integration.
- `editors`: editor clients and generated syntax assets.

After changing the runtime, rebuild its canonical static library before validating generated native programs. Editor-specific syntax assets are generated from the compiler's keyword definitions; editor clients should consume shared semantics from the Language Server.

## Development toolchain

The reference toolchain is Rust 1.99.1 with LLVM 23.1.1. Keep the LLVM version used by `rustc -vV` aligned with the LLVM used by Inkwell and `llvm-sys`. Cross-language LTO is disabled until the Rust and Clang toolchains are verified to exchange compatible bitcode on every supported target. Install the matching LLVM release from the [official LLVM 23.1.1 release page](https://github.com/llvm/llvm-project/releases#release-llvmorg-23.1.1), then copy `.cargo/config.toml.example` to `.cargo/config.toml` and set `LLVM_SYS_231_PREFIX` to the LLVM installation directory. The release workflow uses `LLVM-23.1.1-Linux-X64.tar.xz` on Linux, `LLVM-23.1.1-macOS-ARM64.tar.xz` on Apple Silicon, and `clang+llvm-23.1.1-x86_64-pc-windows-msvc.tar.xz` on Windows x86_64. On Linux, macOS, or WSL, set it with `export LLVM_SYS_231_PREFIX="/path/to/LLVM-23.1.1-<platform>"`; in PowerShell, set it with `$env:LLVM_SYS_231_PREFIX = 'C:\path\to\LLVM-23.1.1-<platform>'`. Do not commit `.cargo/config.toml`.

Confirm the active toolchain before building: `llvm-config --version` and `clang --version` must report LLVM/Clang 23.1.1, and `rustc -vV` must report Rust 1.99.1 with `LLVM version: 23.1.1`. In PowerShell, use `& "$env:LLVM_SYS_231_PREFIX\bin\llvm-config.exe" --version` and `& "$env:LLVM_SYS_231_PREFIX\bin\clang.exe" --version` when those executables are not on `PATH`.

The supported release toolchains are pinned as follows:

| Release target | Rust target | LLVM 23.1.1 archive | Native linker and SDK |
|---------------|-------------|---------------------|-----------------------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | `LLVM-23.1.1-Linux-X64.tar.xz` | GNU or LLVM linker tools and glibc development files; the system C toolchain links the Rust workspace, and packaged Clang links generated Tyrus programs. |
| macOS Apple Silicon | `aarch64-apple-darwin` | `LLVM-23.1.1-macOS-ARM64.tar.xz` | Xcode Command Line Tools provide the Apple linker and SDK; packaged Clang links generated Tyrus programs. |
| Windows x86_64 | `x86_64-pc-windows-msvc` | `clang+llvm-23.1.1-x86_64-pc-windows-msvc.tar.xz` | Visual Studio Build Tools with the C++ workload and Windows SDK provide `link.exe`, `lib.exe`, and the MSVC environment; packaged Clang links generated Tyrus programs. |

The LLVM archive supplies `llvm-ar` and `llvm-nm` for runtime object extraction and symbol discovery. Linux and macOS use GNU/Mach-O static libraries (`.a`); Windows uses MSVC static libraries (`.lib`). Rust, LLVM, the target linker, and SDK must all target the same host architecture. Cross-compilation is not supported by the release packages.

The Bash validation scripts support Linux and macOS. Windows contributors can use `scripts/tyrus.ps1` from PowerShell to build, test, and install the native Windows CLI and runtime; it requires Rust 1.99.1, LLVM 23.1.1 with `LLVM_SYS_231_PREFIX` configured, Python 3, and Visual Studio Build Tools with the C++ workload and Windows SDK. Run the sanitizer and editor validation scripts inside WSL 2 on Windows, with their documented dependencies. Keep a WSL checkout in the Linux filesystem, such as `/home/<user>/src/tyrus`, rather than under `/mnt/c`; see Microsoft's [WSL development environment guidance](https://learn.microsoft.com/windows/wsl/setup/environment) for setup and filesystem recommendations.

## Native release packages

The tag-triggered release workflow builds `linux-x86_64`, `macos-aarch64`, and `windows-x86_64` packages on native runners. It validates that a `v<workspace-version>` tag matches the workspace package version, runs platform smoke checks, packages the CLI, runtime, matching LLVM tools, and SHA-256 checksum, then publishes the assets only after the full matrix succeeds. Linux and macOS packages are `.tar.gz` archives; Windows packages are `.zip` archives. Extract the archive and add its `bin` directory to `PATH`; the included `README.txt` gives checksum and native-linker prerequisites for that platform. Native Tyrus programs require the platform C linker and SDK: glibc development files on Linux, Xcode Command Line Tools on macOS, or Visual Studio Build Tools with the C++ workload and Windows SDK on Windows.

To start a release, update the workspace version, create the matching tag with `git tag v<version>`, and push it with `git push origin v<version>`. The tag must exactly match the workspace version, including any prerelease suffix; the workflow validates it before building and then creates the release with the verified platform packages.

## Validation before a pull request

Before opening a pull request, run the native build and test script for your operating system from the repository root:

```sh
./scripts/tyrus
```

On Windows, run the PowerShell equivalent instead:

```powershell
.\scripts\tyrus.ps1
```

On Linux and macOS, run the sanitizer, WebAssembly, and editor validation scripts:

```sh
./scripts/sanitizers
./scripts/wasm
./scripts/editors
```

On Windows, run the sanitizer and editor scripts from WSL 2, then run the WebAssembly validation in PowerShell:

```powershell
.\scripts\wasm.ps1
```

The PowerShell WebAssembly script accepts `debug` as its first argument for a debug runtime and `-Check` to link a minimal program against the runtime. The Bash WebAssembly script accepts `release` or `debug` and an optional `--check` argument. `scripts/tyrus` builds and installs the native compiler and runtime, `scripts/sanitizers` runs the sanitizer suite, `scripts/wasm` rebuilds the WebAssembly runtime, and `scripts/editors` regenerates or validates editor integrations. The sanitizer script requires the Rust nightly toolchain and its `rust-src` component; the other scripts require the project toolchain and their documented platform tools. Review the output and resolve failures before submitting the pull request.

## Pull requests

Open a pull request against the `main` branch after the required validation passes. Describe the user-visible change, summarize the implementation, list the validation commands and their results, and link any related issue. Keep each pull request focused so reviewers can understand and verify it independently.

Read the [Code of Conduct](CODE_OF_CONDUCT.md) before participating in project spaces. Report suspected security vulnerabilities privately according to the [Security Policy](SECURITY.md); do not disclose them through public issues or pull requests.

## Public compiler diagnostic codes

Public compiler diagnostics use a unique `E####` code and are registered in `crates/tyrus-core/src/diagnostics/codes/codes.rs`.

- Add the code, concise English title, category, and documentation anchor to the canonical registry before emitting it.
- Before the first language release, remove retired diagnostics from the registry and tests; their numbers may be reused. After the first release, keep retired codes marked `Deprecated` and never reuse or change the meaning of a published code.
- Keep context-dependent messages, labels, ranges, `help:`, and Quick Fixes at the diagnostic emission site.
- Use `tyrus explain <code>` to verify the public terminal reference.
- Regenerate the documentation table with `cargo run -p xtask` and verify it with `cargo run -p xtask -- --check`.

Lint rules use their rule name and remain registered in `crates/tyrus-lint/src/config/config.rs`. Runtime and native-module errors use their own public `ERR_<MODULE>_<REASON>` contracts. Do not assign either category an `E####` code unless it becomes a compiler language diagnostic.
