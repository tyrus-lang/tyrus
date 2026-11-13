use std::env;
#[cfg(test)]
use std::fs;
#[cfg(test)]
use std::path::{Path, PathBuf};
use std::process;

use tyrus_codegen as codegen;
use tyrus_core::{diagnostics, project};
#[cfg(any(target_os = "macos", target_env = "msvc"))]
use tyrus_runtime as _;

#[path = "compile/compile.rs"]
mod compile;
mod db;
#[path = "mcp/mcp.rs"]
mod mcp;
#[path = "platform/platform.rs"]
mod platform;
#[path = "repl/repl.rs"]
mod repl;
#[path = "repl_transform/repl_transform.rs"]
mod repl_transform;
#[path = "sealed_env/sealed_env.rs"]
mod sealed_env;
#[path = "watch/watch.rs"]
mod watch;

fn cmd_explain(argument: Option<&str>) {
    let Some(code) = argument else {
        eprintln!("error: missing error code\nUsage: tyrus explain <error-code>");
        process::exit(1);
    };
    let Some(diagnostic) = diagnostics::find(code) else {
        eprintln!("error: unknown public compiler diagnostic `{code}`");
        process::exit(1);
    };

    println!("{}: {}", diagnostic.code, diagnostic.title);
    println!("  Category: {}", diagnostic.category.label());
    println!(
        "  Documentation: https://tyruslang.org/language/errors#{}",
        diagnostic.documentation_anchor
    );
}

#[path = "main/build/build.rs"]
mod build_commands;
#[path = "main/clean/clean.rs"]
mod clean;
#[path = "main/cli/cli.rs"]
mod cli;
#[path = "main/format/format.rs"]
mod format_commands;
#[path = "main/project/project.rs"]
mod project_commands;
#[path = "main/test/test.rs"]
mod test_commands;

use build_commands::*;
use format_commands::*;
use project_commands::*;
use test_commands::*;

#[cfg(test)]
#[path = "main/tests.rs"]
mod tests;

fn main() {
    cli::run();
}
