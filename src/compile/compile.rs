use std::fs;
use std::path::{Path, PathBuf};
use std::process;

use tyrus_codegen as codegen;
use tyrus_core::module_loader;

#[cfg(test)]
pub(crate) static JIT_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
pub(crate) fn acquire_jit_lock() -> std::sync::MutexGuard<'static, ()> {
    JIT_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub struct ReplCompiler {
    project_name: String,
    clang: String,
    runtime_lib: Option<PathBuf>,
    use_jit: bool,
}

impl ReplCompiler {
    pub fn new() -> Self {
        Self {
            project_name: String::from("__repl__"),
            clang: find_clang(),
            runtime_lib: find_runtime_library(),
            use_jit: true,
        }
    }

    pub fn compile_to_binary(&self, source: &str, output_dir: &Path) -> Result<PathBuf, String> {
        let src_dir = output_dir.join("src");
        fs::create_dir_all(&src_dir).map_err(|e| format!("failed to create src/: {e}"))?;

        let entry = src_dir.join("main.tyr");
        fs::write(&entry, source).map_err(|e| format!("failed to write main.tyr: {e}"))?;

        let program =
            module_loader::load_program(&entry, None).map_err(|e| format!("error: {e}"))?;

        let ll_stem = output_dir.join(&self.project_name);
        let ll_path = format!("{}.ll", ll_stem.display());

        codegen::compile(
            &program,
            &self.project_name,
            &ll_stem.to_string_lossy(),
            false,
        )
        .map_err(|e| format!("error[codegen]: {e}"))?;

        let binary_path = crate::platform::executable_path(&output_dir.join(&self.project_name));

        let mut clang_command = process::Command::new(&self.clang);
        clang_command.arg(&ll_path);
        crate::platform::configure_native_linker(&mut clang_command, &self.clang);
        configure_clang_optimization(&mut clang_command, &self.clang, false);
        configure_msvc_runtime(&mut clang_command, &self.clang);
        let runtime = self
            .runtime_lib
            .as_ref()
            .ok_or_else(|| crate::platform::missing_runtime_library_error(false))?;
        clang_command.arg(runtime);
        crate::platform::configure_runtime_system_libraries(
            &mut clang_command,
            &self.clang,
            runtime,
        )?;
        configure_sanitizer_linking(&mut clang_command, &self.clang);
        crate::platform::configure_clang_output(
            &mut clang_command,
            &self.clang,
            binary_path.as_os_str(),
        );
        configure_dead_code_stripping(&mut clang_command, &self.clang);
        crate::platform::run_clang_link(&mut clang_command, &self.clang)?;

        Ok(binary_path)
    }

    fn jit_run(&self, source: &str, output_dir: &Path) -> Result<String, String> {
        let src_dir = output_dir.join("src");
        fs::create_dir_all(&src_dir).map_err(|e| format!("failed to create src/: {e}"))?;

        let entry = src_dir.join("main.tyr");
        fs::write(&entry, source).map_err(|e| format!("failed to write main.tyr: {e}"))?;

        let program =
            module_loader::load_program(&entry, None).map_err(|e| format!("error: {e}"))?;

        codegen::jit_run(&program, &self.project_name)
    }

    pub fn compile_and_run(&self, source: &str, output_dir: &Path) -> Result<String, String> {
        if self.use_jit {
            match self.jit_run(source, output_dir) {
                Ok(output) => return Ok(output),
                Err(e) => {
                    if !e.starts_with("error[jit]") {
                        return Err(e);
                    }
                    eprintln!("warning: {}", jit_fallback_notice(&e));
                }
            }
        }

        let binary = self.compile_to_binary(source, output_dir)?;

        let run = process::Command::new(&binary)
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .output()
            .map_err(|e| format!("failed to run compiled binary: {e}"))?;

        let stdout = String::from_utf8_lossy(&run.stdout).into_owned();

        if !run.status.success() {
            let stderr = String::from_utf8_lossy(&run.stderr);
            if !stderr.is_empty() {
                return Err(stderr.into_owned());
            }
            return Err(match run.status.code() {
                Some(code) => format!(
                    "error: the compiled program exited with status {code} and wrote nothing to stderr"
                ),
                None => {
                    "error: the compiled program was killed by a signal and wrote nothing to stderr"
                        .to_string()
                }
            });
        }

        Ok(stdout)
    }
}

fn jit_fallback_notice(error: &str) -> String {
    format!("JIT execution failed ({error}); falling back to native compilation")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PgoMode {
    #[default]
    Off,
    Generate,
    Use,
}

pub(crate) fn pgo_directory(target_dir: &Path) -> PathBuf {
    target_dir.join("pgo")
}

pub(crate) fn pgo_profile_path(target_dir: &Path) -> PathBuf {
    pgo_directory(target_dir).join("merged.profdata")
}

pub(crate) fn configure_pgo(
    command: &mut process::Command,
    mode: PgoMode,
    target_dir: &Path,
    clang: &str,
) -> Result<(), String> {
    let clang_cl = crate::platform::clang_driver_is_clang_cl(clang);
    match mode {
        PgoMode::Off => Ok(()),
        PgoMode::Generate => {
            let directory = pgo_directory(target_dir);
            fs::create_dir_all(&directory).map_err(|error| {
                format!("error: cannot create `{}`: {error}", directory.display())
            })?;
            let flag = if clang_cl {
                format!("/clang:-fprofile-generate={}", directory.display())
            } else {
                format!("-fprofile-generate={}", directory.display())
            };
            command.arg(flag);
            Ok(())
        }
        PgoMode::Use => {
            let profile = pgo_profile_path(target_dir);
            if !profile.exists() {
                return Err(format!(
                    "error: no profile at `{}`\n\
                     note: build with `--pgo-generate`, run the program on a real workload, then build with `--pgo-use`",
                    profile.display()
                ));
            }
            let flag = if clang_cl {
                format!("/clang:-fprofile-use={}", profile.display())
            } else {
                format!("-fprofile-use={}", profile.display())
            };
            command.arg(flag);
            Ok(())
        }
    }
}

pub(crate) fn merge_pgo_profiles(target_dir: &Path) -> Result<PathBuf, String> {
    let directory = pgo_directory(target_dir);
    let raw: Vec<PathBuf> = fs::read_dir(&directory)
        .map_err(|error| format!("error: cannot read `{}`: {error}", directory.display()))?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "profraw"))
        .collect();
    if raw.is_empty() {
        return Err(format!(
            "error: no `.profraw` files in `{}`\n\
             note: run the binary built with `--pgo-generate` before merging",
            directory.display()
        ));
    }
    let merged = pgo_profile_path(target_dir);
    let tool = find_llvm_profdata();
    let status = process::Command::new(&tool)
        .arg("merge")
        .arg("-output")
        .arg(&merged)
        .args(&raw)
        .status()
        .map_err(|error| {
            format!("error: `{tool}` could not be started ({error}); it ships with LLVM")
        })?;
    if !status.success() {
        return Err(format!("error: `{tool} merge` failed"));
    }
    Ok(merged)
}

fn find_llvm_profdata() -> String {
    if let Some(tool) =
        crate::platform::nonempty_tool_override(std::env::var_os("TYRUS_LLVM_PROFDATA"))
    {
        return tool.to_string_lossy().into_owned();
    }
    let candidates = [
        "llvm-profdata-23",
        "llvm-profdata-22",
        "llvm-profdata-21",
        "llvm-profdata-20",
        "llvm-profdata-19",
        "llvm-profdata-18",
        "llvm-profdata-17",
        "llvm-profdata",
    ];
    if let Some(configured) = option_env!("TYRUS_TOOLCHAIN_LLVM_PROFDATA")
        && std::path::Path::new(configured).is_file()
    {
        return configured.to_string();
    }
    let driver = find_clang();
    if let Some(directory) = std::path::Path::new(&driver).parent() {
        if let Some(tool) = crate::platform::sibling_llvm_tool(directory, &candidates) {
            return tool.to_string_lossy().into_owned();
        }
    }
    crate::platform::find_llvm_tool(
        std::env::var_os("TYRUS_LLVM_PROFDATA"),
        option_env!("TYRUS_TOOLCHAIN_LLVM_PROFDATA"),
        &candidates,
    )
}

/// Link generated executables to the AddressSanitizer runtime when the host
/// compiler itself is running under the sanitizer test harness.
pub(crate) fn configure_sanitizer_linking(command: &mut process::Command, clang: &str) {
    if std::env::var_os("TYRUS_LINK_ADDRESS_SANITIZER").as_deref()
        == Some(std::ffi::OsStr::new("1"))
    {
        if crate::platform::clang_driver_is_clang_cl(clang) {
            command.arg("/fsanitize=address");
        } else {
            command.arg("-fsanitize=address");
        }
    }
}

/// Let the linker drop what the program never reaches.
///
/// The runtime staticlib carries the whole async runtime, the HTTP stack, the
/// crypto primitives and every collection, and without this the linker copies
/// all of it into every binary it produces. Measured on `console.log(1)`, a
/// one-line program: 254 MB and 6.75 s to link, against 171.7 MB and 3.09 s
/// with the sections collected, 2.2x on link time and a third of the size, for
/// a program that uses none of what is being dropped.
///
/// That link is what a `tyrus build` is almost entirely made of, so this is
/// felt on every `tyrus run` and `tyrus test`, not only in what ships.
///
/// `-ffunction-sections`/`-fdata-sections` apply to the generated IR compiled
/// here; the runtime side already arrives with per-symbol sections. Section
/// collection is safe for what the runtime relies on: the linker's default
/// script keeps `.init_array`, so constructors survive, and every native symbol
/// the generated code calls is referenced from it by name.
///
/// Section generation uses Clang driver flags, while section collection uses
/// linker flags; clang-cl gets its MSVC spellings, and unmeasured hosts keep
/// their existing behavior.
pub(crate) fn configure_dead_code_stripping(command: &mut process::Command, clang: &str) {
    command.args(dead_code_stripping_arguments(
        std::env::consts::OS,
        crate::platform::clang_driver_is_clang_cl(clang),
    ));
}

pub(crate) fn configure_clang_optimization(
    command: &mut process::Command,
    clang: &str,
    optimized: bool,
) {
    if crate::platform::clang_driver_is_clang_cl(clang) {
        command.arg(if optimized { "/O2" } else { "/Od" });
        if optimized {
            command.arg("/DNDEBUG");
        }
    } else if optimized {
        command.args(["-O3", "-DNDEBUG"]);
    } else {
        command.arg("-O0");
    }
}

pub(crate) fn configure_msvc_runtime(command: &mut process::Command, clang: &str) {
    if cfg!(all(windows, target_env = "msvc")) && option_env!("TYRUS_STATIC_CRT") == Some("1") {
        if crate::platform::clang_driver_is_clang_cl(clang) {
            command.arg("/MT");
        } else {
            command.arg("-fms-runtime-lib=static");
        }
    }
}

fn dead_code_stripping_arguments(os: &str, clang_cl: bool) -> &'static [&'static str] {
    match os {
        "macos" => &["-ffunction-sections", "-fdata-sections", "-Wl,-dead_strip"],
        "windows" if clang_cl => &["/Gy", "/Gw", "/link", "/OPT:REF"],
        "windows" => &["-ffunction-sections", "-fdata-sections"],
        _ => &[
            "-ffunction-sections",
            "-fdata-sections",
            "-Wl,--gc-sections",
        ],
    }
}

/// Leave the runtime's debug information out of an optimized build.
///
/// A release binary was 87% DWARF: `.debug_str`, `.debug_info`, `.debug_line`
/// and `.debug_ranges` came to 4.75 MB of a 5.4 MB binary whose `.text` was
/// 320 KB. It arrives from the runtime staticlib, which carries 313 separate
/// `.debug_info` sections, and every program the compiler produces was copying
/// all of it. Dropping it takes `console.log(1)` to 647 KB, the same order as
/// the 446 KB of the equivalent Rust program, which is what the 320 KB of
/// `.text` already said the code itself cost.
///
/// Release only. A development build keeps its debug information, which is the
/// arrangement every other toolchain uses and the reason a debugger still works
/// on `tyrus build` without flags.
///
/// What is given up is the symbol names in a backtrace from a panic *inside the
/// runtime*. What a user sees is unaffected: a tyrus runtime error goes through
/// `tyrus_panic`, which prints its own message, and a stripped binary reports
/// an out-of-bounds index with the same text and the same exit status as an
/// unstripped one, verified, not assumed.
pub(crate) fn configure_release_debug_stripping(command: &mut process::Command) {
    if let Some(argument) = release_debug_stripping_argument(std::env::consts::OS) {
        command.arg(argument);
    }
}

fn release_debug_stripping_argument(os: &str) -> Option<&'static str> {
    match os {
        "macos" => Some("-Wl,-S"),
        "windows" => None,
        _ => Some("-Wl,--strip-debug"),
    }
}

fn find_clang() -> String {
    crate::platform::find_llvm_tool(
        std::env::var_os("TYRUS_CLANG"),
        option_env!("TYRUS_TOOLCHAIN_CLANG"),
        &[
            "clang-23", "clang-22", "clang-21", "clang-20", "clang-19", "clang-18", "clang",
        ],
    )
}

fn find_runtime_library() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("TYRUS_RUNTIME_LIB").map(PathBuf::from)
        && path.is_file()
    {
        return Some(path);
    }
    let executable = std::env::current_exe().ok()?;
    let executable_dir = executable.parent()?;
    let candidates = crate::platform::runtime_library_candidates(executable_dir, "tyrus_runtime")
        .into_iter()
        .chain(crate::platform::runtime_library_candidates(
            &executable_dir.join("../lib/tyrus"),
            "tyrus_runtime",
        ));
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .or_else(|| find_development_runtime_library(executable_dir))
}

fn find_development_runtime_library(executable_dir: &Path) -> Option<PathBuf> {
    let deps = executable_dir.join("deps");
    let mut candidates = fs::read_dir(deps)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_linker_is_told_to_collect_unreached_sections() {
        let mut command = process::Command::new("clang");
        configure_dead_code_stripping(&mut command, "clang");
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(
            args.iter().any(|arg| arg == "-ffunction-sections"),
            "expected per-function sections in the generated object: {args:?}"
        );
        assert!(
            args.iter().any(|arg| arg == "-fdata-sections"),
            "expected per-datum sections in the generated object: {args:?}"
        );
        let collects = if cfg!(target_os = "macos") {
            args.iter().any(|arg| arg == "-Wl,-dead_strip")
        } else if cfg!(target_os = "windows") {
            true
        } else {
            args.iter().any(|arg| arg == "-Wl,--gc-sections")
        };
        assert!(
            collects,
            "expected the linker to collect sections: {args:?}"
        );
    }

    #[test]
    fn dead_code_stripping_flags_cover_all_release_hosts() {
        assert_eq!(
            dead_code_stripping_arguments("linux", false),
            [
                "-ffunction-sections",
                "-fdata-sections",
                "-Wl,--gc-sections"
            ]
        );
        assert_eq!(
            dead_code_stripping_arguments("macos", false),
            ["-ffunction-sections", "-fdata-sections", "-Wl,-dead_strip"]
        );
        assert_eq!(
            dead_code_stripping_arguments("windows", false),
            ["-ffunction-sections", "-fdata-sections"]
        );
        assert_eq!(
            release_debug_stripping_argument("linux"),
            Some("-Wl,--strip-debug")
        );
        assert_eq!(release_debug_stripping_argument("macos"), Some("-Wl,-S"));
        assert_eq!(release_debug_stripping_argument("windows"), None);
        assert_eq!(
            dead_code_stripping_arguments("windows", true),
            ["/Gy", "/Gw", "/link", "/OPT:REF"]
        );
    }

    #[test]
    fn clang_cl_uses_msvc_optimization_flags() {
        let mut debug = process::Command::new("clang-cl");
        configure_clang_optimization(&mut debug, "clang-cl", false);
        assert_eq!(debug.get_args().collect::<Vec<_>>(), ["/Od"]);

        let mut release = process::Command::new("clang-cl.exe");
        configure_clang_optimization(&mut release, "clang-cl.exe", true);
        assert_eq!(release.get_args().collect::<Vec<_>>(), ["/O2", "/DNDEBUG"]);
    }

    fn program_output(captured: &str) -> String {
        captured
            .lines()
            .filter_map(|line| {
                let mut line = line.trim_start();
                if line.starts_with("test ") {
                    let (_, suffix) = line.rsplit_once(" ... ")?;
                    line = suffix.trim();
                    if matches!(line, "ok" | "FAILED" | "ignored") || line.starts_with("ignored, ")
                    {
                        return None;
                    }
                }
                if line.starts_with("running ")
                    || line.starts_with("test result:")
                    || line.is_empty()
                {
                    None
                } else {
                    Some(line)
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn program_output_preserves_a_value_appended_to_a_test_harness_prefix() {
        let captured =
            "test repl::tests::completer_suggests_console_members_with_optional_chain ... 42\n";
        assert_eq!(program_output(captured), "42");
        assert_eq!(program_output("test compile::tests::example ... ok\n"), "");
    }

    #[test]
    fn jit_fallback_notice_explains_the_native_compilation_fallback() {
        assert_eq!(
            jit_fallback_notice("error[jit]: process symbol lookup failed"),
            "JIT execution failed (error[jit]: process symbol lookup failed); falling back to native compilation"
        );
    }

    #[test]
    fn clang_cl_pgo_options_are_forwarded_to_the_clang_frontend() {
        let target_dir = std::env::temp_dir().join(format!(
            "tyrus-clang-cl-pgo-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut command = process::Command::new("clang-cl");
        configure_pgo(&mut command, PgoMode::Generate, &target_dir, "clang-cl").unwrap();
        let argument = command.get_args().next().unwrap().to_string_lossy();
        assert!(argument.starts_with("/clang:-fprofile-generate="));
        let profile_path = argument.strip_prefix("/clang:-fprofile-generate=").unwrap();
        assert_eq!(Path::new(profile_path), target_dir.join("pgo"));
        std::fs::remove_dir_all(target_dir).unwrap();
    }

    #[test]
    fn jit_basic_expression() {
        let _guard = acquire_jit_lock();
        let compiler = ReplCompiler::new();
        let dir = tempfile::TempDir::new().unwrap();
        let source = "const main = (): void => { console.log(42); };\n";
        let result = compiler.compile_and_run(source, dir.path());
        assert!(result.is_ok(), "JIT failed: {:?}", result.err());
        let raw = result.unwrap();
        assert_eq!(program_output(raw.trim()), "42", "raw capture was {raw:?}");
    }

    #[test]
    fn jit_variable_and_arithmetic() {
        let _guard = acquire_jit_lock();
        let compiler = ReplCompiler::new();
        let dir = tempfile::TempDir::new().unwrap();
        let source =
            "const main = (): void => { let x: i32 = 10; let y: i32 = 20; console.log(x + y); };\n";
        let result = compiler.compile_and_run(source, dir.path());
        assert!(result.is_ok(), "JIT failed: {:?}", result.err());
        let raw = result.unwrap();
        assert_eq!(program_output(raw.trim()), "30", "raw capture was {raw:?}");
    }

    #[test]
    fn jit_function_call() {
        let _guard = acquire_jit_lock();
        let compiler = ReplCompiler::new();
        let dir = tempfile::TempDir::new().unwrap();
        let source = "\
const add = (a: i32, b: i32): i32 => a + b;
const main = (): void => { console.log(add(40, 2)); };
";
        let result = compiler.compile_and_run(source, dir.path());
        assert!(result.is_ok(), "JIT failed: {:?}", result.err());
        let raw = result.unwrap();
        assert_eq!(program_output(raw.trim()), "42", "raw capture was {raw:?}");
    }

    #[test]
    fn jit_comptime_result_matches_the_same_runtime_call() {
        let _guard = acquire_jit_lock();
        let compiler = ReplCompiler::new();
        let dir = tempfile::TempDir::new().unwrap();
        let source = "\
comptime function double(value: i32): i32 { return value * 2; }
const main = (): void => {
    console.log(comptime(double(21)));
    console.log(double(21));
};
";
        let result = compiler.compile_and_run(source, dir.path());
        assert!(result.is_ok(), "JIT failed: {:?}", result.err());
        let raw = result.unwrap();
        assert_eq!(
            program_output(raw.trim()),
            "42\n42",
            "raw capture was {raw:?}"
        );
    }

    #[test]
    fn jit_comptime_loop_result_matches_the_same_runtime_call() {
        let _guard = acquire_jit_lock();
        let compiler = ReplCompiler::new();
        let dir = tempfile::TempDir::new().unwrap();
        let source = "\
comptime function sum(limit: i32): i32 {
    let total: i32 = 0;
    for (let value: i32 = 0; value < limit; value++) { total += value; }
    return total;
}
const main = (): void => {
    console.log(comptime(sum(10)));
    console.log(sum(10));
};
";
        let result = compiler.compile_and_run(source, dir.path());
        assert!(result.is_ok(), "JIT failed: {:?}", result.err());
        let raw = result.unwrap();
        assert_eq!(
            program_output(raw.trim()),
            "45\n45",
            "raw capture was {raw:?}"
        );
    }

    #[test]
    fn jit_compile_error_propagates() {
        let _guard = acquire_jit_lock();
        let compiler = ReplCompiler::new();
        let dir = tempfile::TempDir::new().unwrap();
        let source = "const main = (): void => { let x = ; };\n";
        let result = compiler.compile_and_run(source, dir.path());
        assert!(result.is_err(), "JIT should have failed but succeeded");
    }

    #[test]
    fn jit_double_print() {
        let _guard = acquire_jit_lock();
        let compiler = ReplCompiler::new();
        let dir = tempfile::TempDir::new().unwrap();
        let source =
            "const main = (): void => { console.log(\"hello\"); console.log(\"world\"); };\n";
        let result = compiler.compile_and_run(source, dir.path());
        assert!(result.is_ok(), "JIT failed: {:?}", result.err());
        let raw = result.unwrap();
        assert_eq!(
            program_output(raw.trim()),
            "hello\nworld",
            "raw capture was {raw:?}"
        );
    }

    #[test]
    fn jit_prints_unsigned_bytes_and_decimals() {
        let _guard = acquire_jit_lock();
        let compiler = ReplCompiler::new();
        let dir = tempfile::TempDir::new().unwrap();
        let source = "const main = (): void => { const byte = 10u8; const money = 5.0d; print(byte, money); };\n";
        let result = compiler.compile_and_run(source, dir.path());
        assert!(result.is_ok(), "JIT failed: {:?}", result.err());
        let raw = result.unwrap();
        assert_eq!(
            program_output(raw.trim()),
            "10 5",
            "raw capture was {raw:?}"
        );
    }

    #[test]
    fn jit_matches_algebraic_enum_payloads() {
        let _guard = acquire_jit_lock();
        let compiler = ReplCompiler::new();
        let dir = tempfile::TempDir::new().unwrap();
        let source = r#"
enum Command {
    Quit,
    Move(i32, i32),
    Write(string),
    ChangeColor { r: u8, g: u8, b: u8 }
}
interface CommandHandlers at Command {
    describe(cmd): i32 {
        match (cmd) {
        Command.Quit => { return 0; }
        Command.Move(x, y) => { return x + y; }
        Command.Write(_) => { return 1; }
        Command.ChangeColor { r, g, b } => { return r as i32 + g as i32 + b as i32; }
        }
    }
}
const main = (): void => {
    const move: Command = Command.Move(10, 20);
    const color: Command = Command.ChangeColor { r: 255u8, g: 0u8, b: 1u8 };
    console.log(move.describe());
    console.log(color.describe());
};
"#;
        let result = compiler.compile_and_run(source, dir.path());
        assert!(result.is_ok(), "JIT failed: {:?}", result.err());
        let raw = result.unwrap();
        assert_eq!(
            program_output(raw.trim()),
            "30\n256",
            "raw capture was {raw:?}"
        );
    }

    #[test]
    fn jit_native_runtime_symbols() {
        let _guard = acquire_jit_lock();
        let compiler = ReplCompiler::new();
        let dir = tempfile::TempDir::new().unwrap();
        let source = "const main = (): void => { console.log(\"hello\".includes(\"he\")); };\n";
        let result = compiler.jit_run(source, dir.path());
        assert!(
            result.is_ok(),
            "JIT runtime symbol lookup failed: {result:?}"
        );
        assert_eq!(result.unwrap().trim(), "true");
    }

    #[test]
    fn jit_async_main_runs_native_runtime_calls() {
        let _guard = acquire_jit_lock();
        let compiler = ReplCompiler::new();
        let dir = tempfile::TempDir::new().unwrap();
        let source = "\
async function main(): Promise<void> {
    await Promise.resolve<void>();
    console.log(\"hello\".includes(\"he\"));
}
";
        let result = compiler.jit_run(source, dir.path());
        assert!(result.is_ok(), "JIT async execution failed: {result:?}");
        assert_eq!(result.unwrap().trim(), "true");
    }

    /// JIT memory leak regression: compile and run a program via JIT 200
    /// times, exercising module/engine creation and destruction. Run under
    /// valgrind via:
    ///   cargo test --bin tyrus --no-run -- -- jit_leak_loop
    ///   valgrind --leak-check=full <test-binary> jit_leak_loop --nocapture
    #[test]
    fn jit_leak_loop() {
        let _guard = acquire_jit_lock();
        let compiler = ReplCompiler::new();
        let dir = tempfile::TempDir::new().unwrap();
        let source = "\
const add = (a: i32, b: i32): i32 => a + b;
const main = (): void => {
    let x: i32 = 10;
    console.log(add(x, 32));
    console.log(\"assert:\", 42);
};
";
        for i in 0..200 {
            let result = compiler.compile_and_run(source, dir.path());
            assert!(
                result.is_ok(),
                "JIT failed at iteration {i}: {:?}",
                result.err()
            );
            let output = result.unwrap();
            let lines: Vec<&str> = output.trim().lines().collect();
            assert_eq!(
                lines.len(),
                2,
                "iteration {i}: expected 2 lines, got {output:?}"
            );
            assert_eq!(
                lines[0], "42",
                "iteration {i}: expected 42, got {}",
                lines[0]
            );
            assert_eq!(lines[1], "assert: 42", "iteration {i}: wrong assert");
        }
    }
}
