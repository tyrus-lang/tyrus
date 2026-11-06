#[path = "clients.rs"]
mod clients;
#[path = "docs.rs"]
mod docs;
#[path = "server.rs"]
mod server;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use docs::DocsCache;

const MCP_DIR_NAME: &str = "tyrus/mcp";

pub fn run(args: &[String]) {
    let Some(command) = args.first().map(String::as_str) else {
        print_help();
        return;
    };
    let result = match command {
        "install" => cache_root().and_then(|root| run_async(install(root, true))),
        "update" => cache_root().and_then(|root| run_async(install(root, false))),
        "status" => cache_root().and_then(|root| status(&root)),
        "uninstall" => cache_root().and_then(|root| uninstall(&root)),
        "serve" => serve_cache_root().and_then(server::run_server),
        "--help" | "-h" | "help" => {
            print_help();
            Ok(())
        }
        other => Err(format!(
            "unknown MCP command `{other}`\nRun `tyrus mcp --help` for usage."
        )),
    };
    if let Err(error) = result {
        fail(&error);
    }
}

fn print_help() {
    println!(
        "Tyrus MCP\n\nUsage:\n  tyrus mcp install    Download documentation and configure detected MCP clients\n  tyrus mcp update     Refresh documentation and update managed client entries\n  tyrus mcp status     Show the documentation cache and configured clients\n  tyrus mcp uninstall  Remove Tyrus-managed client entries and documentation cache\n  tyrus mcp serve      Run the MCP server over stdio (started by MCP clients)"
    );
}

fn run_async<F>(future: F) -> Result<(), String>
where
    F: std::future::Future<Output = Result<(), String>>,
{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("could not create the MCP runtime: {error}"))?;
    runtime.block_on(future)
}

async fn install(cache_root: PathBuf, allow_cached_docs: bool) -> Result<(), String> {
    let executable = env::current_exe()
        .ok()
        .and_then(|path| fs::canonicalize(path).ok())
        .ok_or_else(|| {
            "could not determine the absolute path of the Tyrus executable".to_string()
        })?;
    println!("Preparing Tyrus MCP documentation cache...");
    let cache = match DocsCache::download(&cache_root, env!("CARGO_PKG_VERSION")).await {
        Ok(cache) => cache,
        Err(error) if allow_cached_docs => match DocsCache::load(&cache_root) {
            Ok(cache) => {
                eprintln!("warning: documentation download failed: {error}");
                eprintln!("{}", cached_docs_warning(&cache.docs_version));
                cache
            }
            Err(_) => return Err(error),
        },
        Err(error) => return Err(error),
    };
    println!(
        "Documentation: {} ({} pages, {} sections)",
        cache.docs_version,
        cache.pages.len(),
        cache.sections.len()
    );
    println!("Checking the MCP stdio server...");
    server::probe(&executable, &cache_root).await?;
    println!("MCP server handshake: ok");
    for outcome in clients::install(&cache_root, &executable) {
        print_outcome(&outcome);
    }
    println!(
        "\nTyrus MCP is ready. Restart detected clients when requested, then ask your AI assistant about Tyrus."
    );
    Ok(())
}

fn cached_docs_warning(docs_version: &str) -> String {
    format!("warning: continuing with cached documentation {docs_version}")
}

fn status(cache_root: &Path) -> Result<(), String> {
    match DocsCache::load(cache_root) {
        Ok(cache) => {
            let summary = cache.summary();
            println!(
                "Documentation cache: {}",
                summary["docsVersion"].as_str().unwrap_or("unknown")
            );
            println!(
                "Compiler version: {}",
                summary["compilerVersion"].as_str().unwrap_or("unknown")
            );
            println!(
                "Updated: {}",
                summary["updatedAt"].as_str().unwrap_or("unknown")
            );
            println!("Pages: {}", summary["pageCount"].as_u64().unwrap_or(0));
            println!(
                "Indexed sections: {}",
                summary["sectionCount"].as_u64().unwrap_or(0)
            );
            if cache.docs_version.trim_start_matches('v') != cache.compiler_version {
                println!("Note: cached documentation and installed compiler versions differ.");
            }
        }
        Err(error) => println!("Documentation cache: unavailable ({error})"),
    }
    println!("\nMCP clients:");
    let outcomes = clients::status(cache_root);
    if outcomes.is_empty() {
        println!("  No supported client detected.");
    }
    for outcome in outcomes {
        print_outcome(&outcome);
    }
    println!("\nServer command: tyrus mcp serve (started by each configured client)");
    Ok(())
}

fn uninstall(cache_root: &PathBuf) -> Result<(), String> {
    println!("Removing Tyrus MCP client entries...");
    let outcomes = clients::uninstall(cache_root);
    for outcome in &outcomes {
        print_outcome(outcome);
    }
    let cache_dir = cache_root.join("cache");
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir)
            .map_err(|error| format!("could not remove the documentation cache: {error}"))?;
    }
    let pointer = cache_root.join("current.json");
    if pointer.exists() {
        fs::remove_file(pointer)
            .map_err(|error| format!("could not remove cache metadata: {error}"))?;
    }
    let has_retained_state =
        fs::metadata(cache_root.join("clients.json")).is_ok_and(|metadata| metadata.len() > 2);
    if !has_retained_state && cache_root.exists() {
        let state = cache_root.join("clients.json");
        if state.exists() {
            fs::remove_file(state)
                .map_err(|error| format!("could not remove MCP client state: {error}"))?;
        }
        match fs::remove_dir(cache_root) {
            Ok(()) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                ) => {}
            Err(error) => {
                return Err(format!(
                    "could not remove empty MCP state directory: {error}"
                ));
            }
        }
    }
    println!("Tyrus MCP cache removed.");
    if has_retained_state {
        println!(
            "Some client entries changed after installation and were preserved. Remove those entries manually, then run `tyrus mcp uninstall` again."
        );
    }
    Ok(())
}

fn print_outcome(outcome: &clients::ClientOutcome) {
    println!(
        "  {}: {} ({})",
        outcome.name, outcome.status, outcome.detail
    );
}

fn cache_root() -> Result<PathBuf, String> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| "could not determine the user's configuration directory".to_string())?;
    Ok(base.config_dir().join(MCP_DIR_NAME))
}

fn serve_cache_root() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("TYRUS_MCP_HOME") {
        return Ok(PathBuf::from(path));
    }
    cache_root()
}

fn fail(message: &str) -> ! {
    eprintln!("error: {message}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::cached_docs_warning;

    #[test]
    fn cached_docs_warning_preserves_the_version_prefix() {
        assert_eq!(
            cached_docs_warning("v0.1.0"),
            "warning: continuing with cached documentation v0.1.0"
        );
    }
}
