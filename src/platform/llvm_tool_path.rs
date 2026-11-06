use std::ffi::OsStr;
use std::path::PathBuf;

pub(crate) fn prefixed_tool(
    prefix: Option<&OsStr>,
    name: &str,
    windows_host: bool,
) -> Option<PathBuf> {
    let prefix = prefix.filter(|prefix| !prefix.is_empty())?;
    let bin = PathBuf::from(prefix).join("bin");
    let candidate = bin.join(name);
    if candidate.is_file() {
        return Some(candidate);
    }
    if windows_host {
        let candidate = bin.join(format!("{name}.exe"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[allow(dead_code)]
pub(crate) fn missing_tool_message(tool: &str, purpose: &str) -> String {
    format!(
        "LLVM 23 `{tool}` is required to {purpose}; install LLVM 23.1.1 and set `LLVM_SYS_231_PREFIX` to its installation directory or add `{tool}` to `PATH`"
    )
}
