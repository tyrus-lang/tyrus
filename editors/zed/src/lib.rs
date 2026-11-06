//! Zed extension that starts the tyrus language server. All language
//! intelligence (diagnostics, completion, rename, references, signature help,
//! semantic tokens, ...) comes from `tyrus lsp`; this crate only launches it.

use zed_extension_api as zed;

struct TyrusExtension;

impl zed::Extension for TyrusExtension {
    fn new() -> Self {
        TyrusExtension
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        // Use the `tyrus` binary from the project's environment / PATH.
        let path = worktree
            .which("tyrus")
            .ok_or_else(|| "`tyrus` was not found in PATH".to_string())?;
        Ok(zed::Command {
            command: path,
            args: vec!["lsp".to_string()],
            env: worktree.shell_env(),
        })
    }
}

zed::register_extension!(TyrusExtension);
