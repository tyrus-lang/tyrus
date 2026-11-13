use std::path::{Path, PathBuf};
use std::process::Command;

mod llvm_tool_path;

pub(crate) fn runtime_library_candidates(directory: &Path, name: &str) -> Vec<PathBuf> {
    runtime_library_candidates_for(directory, name, cfg!(target_env = "msvc"))
}

fn runtime_library_candidates_for(directory: &Path, name: &str, msvc: bool) -> Vec<PathBuf> {
    let native = if msvc {
        directory.join(format!("{name}.lib"))
    } else {
        directory.join(format!("lib{name}.a"))
    };
    vec![native]
}

pub(crate) fn missing_runtime_library_error(optimized: bool) -> String {
    let archive = if cfg!(target_env = "msvc") {
        "tyrus_runtime.lib"
    } else {
        "libtyrus_runtime.a"
    };
    let profile = if optimized { "release" } else { "debug" };
    format!(
        "error: required Tyrus runtime library `{archive}` was not found for the {profile} profile\nnote: build it with `cargo build {release_flag}-p tyrus-runtime` from the Tyrus workspace, or set `TYRUS_RUNTIME_LIB` to a compatible {archive} file",
        release_flag = if optimized { "--release " } else { "" }
    )
}

pub(crate) fn is_hashed_runtime_library(path: &Path, name: &str) -> bool {
    is_hashed_runtime_library_for(path, name, cfg!(target_env = "msvc"))
}

fn is_hashed_runtime_library_for(path: &Path, name: &str, msvc: bool) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if msvc {
        file_name.starts_with(&format!("{name}-")) && file_name.ends_with(".lib")
    } else {
        file_name.starts_with(&format!("lib{name}-")) && file_name.ends_with(".a")
    }
}

pub(crate) fn executable_path(path: &Path) -> PathBuf {
    executable_path_for(path, cfg!(windows))
}

fn executable_path_for(path: &Path, windows: bool) -> PathBuf {
    if !windows {
        return path.to_path_buf();
    }
    if path
        .extension()
        .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("exe"))
    {
        return path.to_path_buf();
    }
    let mut executable = path.as_os_str().to_os_string();
    executable.push(".exe");
    executable.into()
}

pub(crate) fn configure_runtime_system_libraries(
    command: &mut Command,
    clang: &str,
    runtime: &Path,
) -> Result<(), String> {
    let (arguments, manifest_directory) = native_link_arguments()?;
    let library_directories = native_library_directories(runtime, manifest_directory.as_deref());
    if clang_driver_is_clang_cl(clang) {
        for directory in &library_directories {
            command.arg(format!("/LIBPATH:{}", directory.display()));
        }
        for argument in arguments {
            if let Some(path) = native_link_library_path(&argument, &library_directories) {
                command.arg(path);
            } else {
                command.arg(native_link_argument_for_driver(&argument, clang)?);
            }
        }
    } else if cfg!(target_env = "msvc") {
        for directory in &library_directories {
            command.arg("-L").arg(directory);
        }
        for argument in arguments {
            if let Some(path) = native_link_library_path(&argument, &library_directories) {
                command.arg(path);
            } else if argument.to_ascii_lowercase().ends_with(".lib")
                && !argument.starts_with('-')
                && !argument.starts_with('/')
            {
                command.arg(format!("-l{}", &argument[..argument.len() - 4]));
            } else if argument.starts_with('/') {
                command.arg("-Xlinker").arg(argument);
            } else {
                command.arg(argument);
            }
        }
    } else {
        command.args(arguments);
    }
    Ok(())
}

fn native_library_directories(runtime: &Path, manifest_directory: Option<&Path>) -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(directory) = runtime.parent() {
        append_profile_directories(&mut directories, directory);
        let is_profile_directory = |path: &Path| {
            path.file_name().is_some_and(|name| {
                name == std::ffi::OsStr::new("debug") || name == std::ffi::OsStr::new("release")
            })
        };
        if directory
            .file_name()
            .is_some_and(|name| name == std::ffi::OsStr::new("deps"))
            && let Some(profile_directory) = directory.parent()
        {
            directories.push(profile_directory.to_path_buf());
        }
        if is_profile_directory(directory) {
            if let Some(target_directory) = directory.parent() {
                let sibling = target_directory.join(
                    if directory
                        .file_name()
                        .is_some_and(|name| name == std::ffi::OsStr::new("debug"))
                    {
                        "release"
                    } else {
                        "debug"
                    },
                );
                append_profile_directories(&mut directories, &sibling);
            }
        }
    }
    if let Some(directory) = manifest_directory {
        append_profile_directories(&mut directories, directory);
    }
    directories.sort();
    directories.dedup();
    directories
}

fn append_profile_directories(directories: &mut Vec<PathBuf>, directory: &Path) {
    directories.push(directory.to_path_buf());
    directories.push(directory.join("deps"));
    if directory
        .file_name()
        .is_some_and(|name| name == std::ffi::OsStr::new("debug"))
        && let Some(target_directory) = directory.parent()
    {
        directories.push(target_directory.join("release"));
        directories.push(target_directory.join("release/deps"));
    }
    if directory
        .file_name()
        .is_some_and(|name| name == std::ffi::OsStr::new("release"))
        && let Some(target_directory) = directory.parent()
    {
        directories.push(target_directory.join("debug"));
        directories.push(target_directory.join("debug/deps"));
    }
}

fn native_link_library_path(argument: &str, directories: &[PathBuf]) -> Option<PathBuf> {
    let library = Path::new(argument);
    if !library
        .extension()
        .is_some_and(|extension| extension.to_string_lossy().eq_ignore_ascii_case("lib"))
    {
        return None;
    }
    if library.is_file() {
        return Some(library.to_path_buf());
    }
    let file_name = library.file_name()?;
    directories
        .iter()
        .map(|directory| directory.join(file_name))
        .find(|candidate| candidate.is_file())
}

fn native_link_argument_for_driver(argument: &str, clang: &str) -> Result<String, String> {
    if !clang_driver_is_clang_cl(clang) {
        return Ok(argument.to_string());
    }
    if let Some(library) = argument.strip_prefix("-l") {
        if library.is_empty() {
            return Err("native linker manifest contains an empty library name".to_string());
        }
        return Ok(if let Some(exact_name) = library.strip_prefix(':') {
            exact_name.to_string()
        } else {
            format!("{library}.lib")
        });
    }
    if argument == "-pthread"
        || argument == "-framework"
        || argument.starts_with("-L")
        || argument.starts_with("-Wl,")
        || argument == "-Xlinker"
    {
        return Err(format!(
            "native linker argument `{argument}` is not supported by the clang-cl driver"
        ));
    }
    Ok(argument.to_string())
}

pub(crate) fn clang_driver_is_clang_cl(driver: &str) -> bool {
    driver.rsplit(['/', '\\']).next().is_some_and(|name| {
        name.eq_ignore_ascii_case("clang-cl") || name.eq_ignore_ascii_case("clang-cl.exe")
    })
}

pub(crate) fn configure_clang_output(command: &mut Command, clang: &str, output: &std::ffi::OsStr) {
    if clang_driver_is_clang_cl(clang) {
        let mut argument = std::ffi::OsString::from("/Fe:");
        argument.push(output);
        command.arg(argument);
    } else {
        command.arg("-o").arg(output);
    }
}

pub(crate) fn native_link_failure_note() -> &'static str {
    native_link_toolchain_requirement(std::env::consts::OS)
}

pub(crate) fn clang_link_failure_message(clang: &str) -> String {
    format!(
        "error: Clang driver `{clang}` failed to compile the generated IR; inspect its diagnostics above\nnote: {}",
        native_link_failure_note()
    )
}

pub(crate) fn clang_link_failure_message_with_diagnostics(
    clang: &str,
    diagnostics: &[u8],
) -> String {
    let mut message = clang_link_failure_message(clang);
    if let Some(library) = missing_native_library(diagnostics) {
        message.push_str(&format!(
            "\nnote: native system library `{library}` was not found; install its development package or add the library to the platform linker search path"
        ));
    }
    message
}

pub(crate) fn run_clang_link(command: &mut Command, clang: &str) -> Result<(), String> {
    let output = command.output().map_err(|error| {
        format!(
            "error: could not start Clang driver `{clang}` ({error}); install LLVM/Clang 23.1.1 and set `LLVM_SYS_231_PREFIX` to its installation directory or `TYRUS_CLANG` to the driver executable"
        )
    })?;
    use std::io::Write;
    std::io::stdout()
        .lock()
        .write_all(&output.stdout)
        .map_err(|error| format!("error: could not forward Clang output: {error}"))?;
    std::io::stderr()
        .lock()
        .write_all(&output.stderr)
        .map_err(|error| format!("error: could not forward Clang diagnostics: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(clang_link_failure_message_with_diagnostics(
            clang,
            &output.stderr,
        ))
    }
}

fn missing_native_library(diagnostics: &[u8]) -> Option<String> {
    let diagnostics = String::from_utf8_lossy(diagnostics);
    for line in diagnostics.lines() {
        if let Some(missing) = line
            .split_once("cannot find -l")
            .map(|(_, missing)| missing)
            .or_else(|| {
                line.split_once("library not found for -l")
                    .map(|(_, missing)| missing)
            })
            .or_else(|| {
                line.split_once("unable to find library -l")
                    .map(|(_, missing)| missing)
            })
        {
            let name = missing
                .split(|character: char| character.is_whitespace() || character == ':')
                .next()
                .unwrap_or_default();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
        if line.contains("LNK1104")
            && let Some((_, quoted)) = line.split_once("cannot open file '")
            && let Some((name, _)) = quoted.split_once('\'')
            && !name.is_empty()
        {
            return Some(name.to_string());
        }
        if line.contains("LNK1181")
            && let Some((_, quoted)) = line.split_once("cannot open input file '")
            && let Some((name, _)) = quoted.split_once('\'')
            && !name.is_empty()
        {
            return Some(name.to_string());
        }
        if let Some((_, framework)) = line.split_once("framework not found ") {
            let name = framework
                .trim()
                .trim_matches(['\'', '"'])
                .split_whitespace()
                .next()
                .unwrap_or_default();
            if !name.is_empty() {
                return Some(format!("{name}.framework"));
            }
        }
    }
    None
}

fn native_link_toolchain_requirement(os: &str) -> &'static str {
    match os {
        "linux" => {
            "if linker or system library diagnostics appear, install a GNU-compatible linker and glibc development files"
        }
        "macos" => {
            "if linker or system library diagnostics appear, install Xcode Command Line Tools for the Apple linker and macOS SDK"
        }
        "windows" => {
            "if linker or system library diagnostics appear, install Visual Studio Build Tools with the C++ workload and Windows SDK"
        }
        _ => {
            "if linker or system library diagnostics appear, install a linker and system libraries compatible with the host target"
        }
    }
}

fn native_link_arguments() -> Result<(Vec<String>, Option<PathBuf>), String> {
    if let Some(path) = std::env::var_os("TYRUS_NATIVE_LIBS_FILE") {
        let path = PathBuf::from(path);
        let arguments = read_native_link_arguments(&path)?;
        return Ok((arguments, path.parent().map(Path::to_path_buf)));
    }
    let executable_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    if let Some(directory) = executable_dir {
        let candidates = [
            directory.join("tyrus-native-libs.json"),
            directory.join("../lib/tyrus/tyrus-native-libs.json"),
            directory.join("../tyrus-native-libs.json"),
        ];
        for candidate in candidates {
            if candidate.is_file() {
                let arguments = read_native_link_arguments(&candidate)?;
                return Ok((arguments, candidate.parent().map(Path::to_path_buf)));
            }
        }
    }
    if let Some(arguments) = fallback_native_link_arguments(std::env::consts::OS) {
        return Ok((
            arguments
                .iter()
                .map(|argument| (*argument).to_string())
                .collect(),
            None,
        ));
    }
    Err(format!(
        "native linker arguments are unavailable for `{}`; set `TYRUS_NATIVE_LIBS_FILE` to the target-matched JSON manifest generated from `rustc --print=native-static-libs` (the release workflow generates this file automatically)",
        native_target_triple()
    ))
}

fn fallback_native_link_arguments(os: &str) -> Option<&'static [&'static str]> {
    (os == "linux").then_some(&["-lpthread", "-ldl", "-lrt", "-lutil", "-lm"])
}

fn read_native_link_arguments(path: &Path) -> Result<Vec<String>, String> {
    let contents = std::fs::read_to_string(path).map_err(|error| {
        format!(
            "could not read native linker arguments from `{}`: {error}",
            path.display()
        )
    })?;
    let manifest: NativeLinkManifest = serde_json::from_str(&contents).map_err(|error| {
        format!(
            "native linker argument file `{}` must contain a target and a JSON string array: {error}",
            path.display()
        )
    })?;
    if manifest.target != native_target_triple() {
        return Err(format!(
            "native linker argument file `{}` targets `{}`, but this compiler targets `{}`",
            path.display(),
            manifest.target,
            native_target_triple()
        ));
    }
    let arguments = manifest.arguments;
    if arguments.is_empty()
        || arguments.len() > 256
        || arguments
            .iter()
            .any(|argument| argument.is_empty() || argument.len() > 4096 || argument.contains('\0'))
    {
        return Err(format!(
            "native linker argument file `{}` contains an invalid argument list",
            path.display()
        ));
    }
    Ok(arguments)
}

#[derive(serde::Deserialize)]
struct NativeLinkManifest {
    target: String,
    arguments: Vec<String>,
}

fn native_target_triple() -> &'static str {
    env!("TYRUS_TARGET_TRIPLE")
}

pub(crate) fn find_llvm_tool(
    environment_override: Option<std::ffi::OsString>,
    configured_path: Option<&str>,
    candidates: &[&str],
) -> String {
    if let Some(path) = nonempty_tool_override(environment_override) {
        return path.to_string_lossy().into_owned();
    }
    let sibling = std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(Path::to_path_buf))
        .and_then(|directory| sibling_llvm_tool(&directory, candidates));
    let prefixed = nonempty_tool_override(std::env::var_os("LLVM_SYS_231_PREFIX"))
        .and_then(|prefix| prefixed_llvm_tool(&prefix, candidates));
    if let Some(path) = select_bundled_llvm_tool(
        sibling,
        prefixed,
        configured_path
            .map(PathBuf::from)
            .filter(|path| path.is_file()),
    ) {
        return path.to_string_lossy().into_owned();
    }
    for candidate in candidates {
        if Command::new(candidate)
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
        {
            return (*candidate).to_string();
        }
        #[cfg(windows)]
        {
            let executable = format!("{candidate}.exe");
            if Command::new(&executable)
                .arg("--version")
                .output()
                .is_ok_and(|output| output.status.success())
            {
                return executable;
            }
        }
    }
    candidates.first().copied().unwrap_or("clang").to_string()
}

pub(crate) fn nonempty_tool_override(
    path: Option<std::ffi::OsString>,
) -> Option<std::ffi::OsString> {
    path.filter(|path| !path.is_empty())
}

fn select_bundled_llvm_tool(
    sibling: Option<PathBuf>,
    prefixed: Option<PathBuf>,
    configured: Option<PathBuf>,
) -> Option<PathBuf> {
    sibling.or(prefixed).or(configured)
}

pub(crate) fn sibling_llvm_tool(directory: &Path, candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().find_map(|candidate| {
        let path = directory.join(candidate);
        if path.is_file() {
            return Some(path);
        }
        #[cfg(windows)]
        {
            let path = directory.join(format!("{candidate}.exe"));
            if path.is_file() {
                return Some(path);
            }
        }
        None
    })
}

fn prefixed_llvm_tool(prefix: &std::ffi::OsStr, candidates: &[&str]) -> Option<PathBuf> {
    candidates
        .iter()
        .find_map(|candidate| llvm_tool_path::prefixed_tool(Some(prefix), candidate, cfg!(windows)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clang_cl_driver_is_detected_from_windows_and_unix_paths() {
        assert!(clang_driver_is_clang_cl("C:\\LLVM\\bin\\clang-cl.exe"));
        assert!(clang_driver_is_clang_cl("/opt/llvm/bin/clang-cl"));
        assert!(!clang_driver_is_clang_cl("clang"));
    }

    #[test]
    fn clang_cl_native_link_arguments_use_msvc_library_names() {
        assert_eq!(
            native_link_argument_for_driver("-lws2_32", "clang-cl.exe").unwrap(),
            "ws2_32.lib"
        );
        assert_eq!(
            native_link_argument_for_driver("-l:custom.lib", "clang-cl").unwrap(),
            "custom.lib"
        );
        assert_eq!(
            native_link_argument_for_driver("kernel32.lib", "clang-cl").unwrap(),
            "kernel32.lib"
        );
        assert!(native_link_argument_for_driver("-pthread", "clang-cl").is_err());
        assert!(native_link_argument_for_driver("-Lvendor/lib", "clang-cl").is_err());
        assert_eq!(
            native_link_argument_for_driver("-lcrypto", "clang"),
            Ok("-lcrypto".to_string())
        );
    }

    #[test]
    fn clang_cl_output_uses_a_msvc_style_argument_and_preserves_spaces() {
        let mut command = Command::new("clang-cl");
        configure_clang_output(
            &mut command,
            "clang-cl",
            std::ffi::OsStr::new("out dir/program.exe"),
        );
        assert_eq!(
            command.get_args().next(),
            Some(std::ffi::OsStr::new("/Fe:out dir/program.exe"))
        );
    }

    #[test]
    fn llvm_tool_lookup_supports_the_llvm_sys_prefix() {
        let prefix = std::env::temp_dir().join(format!(
            "tyrus-llvm-prefix-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let bin = prefix.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let executable = executable_path(&bin.join("clang-23"));
        std::fs::write(&executable, b"").unwrap();

        assert_eq!(
            prefixed_llvm_tool(prefix.as_os_str(), &["clang-23"]),
            Some(executable)
        );

        std::fs::remove_dir_all(prefix).unwrap();
    }

    #[test]
    fn empty_tool_overrides_are_treated_as_unset() {
        assert_eq!(nonempty_tool_override(Some("".into())), None);
        assert_eq!(
            nonempty_tool_override(Some("/opt/llvm/bin/clang".into())),
            Some("/opt/llvm/bin/clang".into())
        );
    }

    #[test]
    fn llvm_tool_prefix_ignores_an_empty_environment_value() {
        assert_eq!(
            llvm_tool_path::prefixed_tool(Some(std::ffi::OsStr::new("")), "llvm-ar", false),
            None
        );
    }

    #[test]
    fn llvm_tool_prefix_resolves_windows_executable_names() {
        let prefix = std::env::temp_dir().join(format!(
            "tyrus-llvm-windows-prefix-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let bin = prefix.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let executable = bin.join("llvm-ar.exe");
        std::fs::write(&executable, b"").unwrap();

        assert_eq!(
            llvm_tool_path::prefixed_tool(Some(prefix.as_os_str()), "llvm-ar", true),
            Some(executable)
        );

        std::fs::remove_dir_all(prefix).unwrap();
    }

    #[test]
    fn missing_llvm_tool_message_names_installation_and_path_options() {
        let message =
            llvm_tool_path::missing_tool_message("llvm-ar", "extract the runtime archive for MSVC");
        assert!(message.contains("LLVM 23 `llvm-ar`"));
        assert!(message.contains("LLVM_SYS_231_PREFIX"));
        assert!(message.contains("PATH"));
    }

    #[test]
    fn native_link_argument_files_require_a_bounded_nonempty_string_array() {
        let path = std::env::temp_dir().join(format!(
            "tyrus-native-link-args-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(
            &path,
            format!(
                r#"{{"target":"{}","arguments":["-framework","Security","kernel32.lib"]}}"#,
                native_target_triple()
            ),
        )
        .unwrap();
        assert_eq!(
            read_native_link_arguments(&path).unwrap(),
            ["-framework", "Security", "kernel32.lib"]
        );
        std::fs::write(
            &path,
            format!(
                r#"{{"target":"{}","arguments":[]}}"#,
                native_target_triple()
            ),
        )
        .unwrap();
        assert!(read_native_link_arguments(&path).is_err());
        std::fs::write(&path, r#"{"target":"other-target","arguments":["-lfoo"]}"#).unwrap();
        assert!(read_native_link_arguments(&path).is_err());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn native_link_libraries_are_resolved_from_profile_and_dependency_directories() {
        let root = std::env::temp_dir().join(format!(
            "tyrus-native-library-paths-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let debug = root.join("target/debug");
        let release_dependencies = root.join("target/release/deps");
        std::fs::create_dir_all(&debug).unwrap();
        std::fs::create_dir_all(&release_dependencies).unwrap();
        let dependency = release_dependencies.join("windows.0.52.0.lib");
        std::fs::write(&dependency, b"library").unwrap();

        let directories =
            native_library_directories(&debug.join("tyrus_runtime.lib"), Some(&debug));
        assert_eq!(
            native_link_library_path("windows.0.52.0.lib", &directories),
            Some(dependency)
        );

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_target_triple_matches_the_cargo_build_target() {
        assert_eq!(native_target_triple(), env!("TYRUS_TARGET_TRIPLE"));
        assert!(!native_target_triple().is_empty());
    }

    #[test]
    fn native_link_fallback_is_defined_only_for_linux_and_includes_math() {
        assert_eq!(
            fallback_native_link_arguments("linux"),
            Some(["-lpthread", "-ldl", "-lrt", "-lutil", "-lm"].as_slice())
        );
        assert_eq!(fallback_native_link_arguments("macos"), None);
        assert_eq!(fallback_native_link_arguments("windows"), None);
    }

    #[test]
    fn native_link_toolchain_notes_name_requirements_for_supported_hosts() {
        assert!(native_link_toolchain_requirement("linux").contains("glibc development files"));
        assert!(native_link_toolchain_requirement("macos").contains("Xcode Command Line Tools"));
        assert!(native_link_toolchain_requirement("windows").contains("Windows SDK"));
        assert!(native_link_toolchain_requirement("other").contains("host target"));

        let message = clang_link_failure_message("clang-cl");
        assert!(message.contains("Clang driver `clang-cl`"));
        assert!(message.contains("diagnostics above"));
        assert!(message.contains("if linker or system library diagnostics appear"));
    }

    #[test]
    fn missing_native_libraries_are_named_in_link_failure_guidance() {
        for diagnostics in [
            &b"ld: cannot find -lssl: No such file or directory"[..],
            &b"ld: library not found for -lz"[..],
            &b"ld.lld: error: unable to find library -lcrypto"[..],
            &b"fatal error LNK1104: cannot open file 'sqlite3.lib'"[..],
            &b"ld: framework not found Security"[..],
            &b"LINK : fatal error LNK1181: cannot open input file 'bcrypt.lib'"[..],
        ] {
            let message = clang_link_failure_message_with_diagnostics("clang", diagnostics);
            assert!(message.contains("native system library"), "{message}");
            assert!(message.contains("development package"), "{message}");
        }
        assert_eq!(
            missing_native_library(b"ld: cannot find -lssl: No such file or directory"),
            Some("ssl".to_string())
        );
        assert_eq!(
            missing_native_library(b"ld: library not found for -lz"),
            Some("z".to_string())
        );
        assert_eq!(
            missing_native_library(b"ld.lld: error: unable to find library -lcrypto"),
            Some("crypto".to_string())
        );
        assert_eq!(
            missing_native_library(b"fatal error LNK1104: cannot open file 'sqlite3.lib'"),
            Some("sqlite3.lib".to_string())
        );
        assert_eq!(
            missing_native_library(b"ld: framework not found Security"),
            Some("Security.framework".to_string())
        );
        assert_eq!(
            missing_native_library(
                b"LINK : fatal error LNK1181: cannot open input file 'bcrypt.lib'"
            ),
            Some("bcrypt.lib".to_string())
        );
        assert_eq!(
            missing_native_library(b"undefined symbol: tyrus_print"),
            None
        );
    }

    #[test]
    fn runtime_archive_lookup_prefers_the_native_abi_format() {
        let candidates = runtime_library_candidates(Path::new("runtime"), "tyrus_runtime");
        let native_name = if cfg!(target_env = "msvc") {
            "tyrus_runtime.lib"
        } else {
            "libtyrus_runtime.a"
        };
        assert_eq!(
            candidates.first(),
            Some(&Path::new("runtime").join(native_name))
        );
    }

    #[test]
    fn runtime_archive_names_cover_gnu_and_msvc_formats() {
        assert_eq!(
            runtime_library_candidates_for(Path::new("runtime"), "tyrus_runtime", false),
            vec![Path::new("runtime/libtyrus_runtime.a").to_path_buf()]
        );
        assert_eq!(
            runtime_library_candidates_for(Path::new("runtime"), "tyrus_runtime", true),
            vec![Path::new("runtime/tyrus_runtime.lib").to_path_buf()]
        );
        assert!(is_hashed_runtime_library_for(
            Path::new("runtime/deps/libtyrus_runtime-abc.a"),
            "tyrus_runtime",
            false
        ));
        assert!(is_hashed_runtime_library_for(
            Path::new("runtime/deps/tyrus_runtime-abc.lib"),
            "tyrus_runtime",
            true
        ));
        assert!(!is_hashed_runtime_library_for(
            Path::new("runtime/deps/libtyrus_runtime-abc.a"),
            "tyrus_runtime",
            true
        ));
        assert!(!is_hashed_runtime_library_for(
            Path::new("runtime/deps/tyrus_runtime-abc.lib"),
            "tyrus_runtime",
            false
        ));
    }

    #[test]
    fn missing_runtime_library_error_names_the_abi_and_profile() {
        let message = missing_runtime_library_error(true);
        let expected_archive = if cfg!(target_env = "msvc") {
            "tyrus_runtime.lib"
        } else {
            "libtyrus_runtime.a"
        };
        assert!(message.contains(expected_archive));
        assert!(message.contains("release profile"));
        assert!(message.contains("cargo build --release -p tyrus-runtime"));
        assert!(message.contains("TYRUS_RUNTIME_LIB"));
    }

    #[test]
    fn development_archive_filter_matches_the_target_abi_format() {
        let archive_name = if cfg!(target_env = "msvc") {
            "tyrus_runtime-abc123.lib"
        } else {
            "libtyrus_runtime-abc123.a"
        };
        assert!(is_hashed_runtime_library(
            Path::new("runtime/deps").join(archive_name).as_path(),
            "tyrus_runtime"
        ));
        let other_abi_archive = if cfg!(target_env = "msvc") {
            "runtime/deps/libtyrus_runtime-abc123.a"
        } else {
            "runtime/deps/tyrus_runtime-abc123.lib"
        };
        assert!(!is_hashed_runtime_library(
            Path::new(other_abi_archive),
            "tyrus_runtime"
        ));
    }

    #[test]
    fn native_executable_gets_the_platform_extension() {
        let path = executable_path(Path::new("target/app"));
        let expected = if cfg!(windows) {
            Path::new("target/app.exe")
        } else {
            Path::new("target/app")
        };
        assert_eq!(path, expected);
    }

    #[test]
    fn native_executable_extensions_cover_windows_and_unix() {
        assert_eq!(
            executable_path_for(Path::new("target/app"), false),
            Path::new("target/app")
        );
        assert_eq!(
            executable_path_for(Path::new("target/app"), true),
            Path::new("target/app.exe")
        );
        assert_eq!(
            executable_path_for(Path::new("target/app.EXE"), true),
            Path::new("target/app.EXE")
        );
    }

    #[test]
    fn llvm_tools_can_be_resolved_next_to_the_installed_compiler() {
        let directory = std::env::temp_dir().join(format!(
            "tyrus-llvm-tools-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let tool_name = if cfg!(windows) { "clang.exe" } else { "clang" };
        let tool = directory.join(tool_name);
        std::fs::write(&tool, b"bundled tool").unwrap();
        assert_eq!(sibling_llvm_tool(&directory, &["clang"]), Some(tool));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn installed_llvm_tools_take_precedence_over_build_machine_paths() {
        let root = std::env::temp_dir().join(format!(
            "tyrus-llvm-tool-precedence-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let sibling = root.join("package/bin/clang");
        let prefixed = root.join("llvm/bin/clang");
        let configured = root.join("build-machine/llvm/bin/clang");
        for path in [&sibling, &prefixed, &configured] {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"tool").unwrap();
        }

        assert_eq!(
            select_bundled_llvm_tool(Some(sibling.clone()), Some(prefixed), Some(configured)),
            Some(sibling)
        );

        std::fs::remove_dir_all(root).unwrap();
    }
}
