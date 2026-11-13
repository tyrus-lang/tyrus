use super::*;
use std::path::PathBuf;

fn usage() -> ! {
    eprintln!(
        r#"tyrus, TypeScript syntax, Rust semantics

Usage:
  tyrus new [name] [--target script|cli|api] [--database none|sqlite|postgresql|mysql|mongodb]
                           Create a new tyrus project (prompts for name/target when omitted)
  tyrus --init              Add missing default keys to tyrus.json (keeps existing values)
  tyrus --default           Reset tyrus.json to project defaults (removes custom values)
  tyrus info                Show project metadata
  tyrus clean [--cache] [--dry-run] [--force]
                            Remove project build output and optionally the global cache
  tyrus db introspect       Create or refresh the SQLite schema snapshot
  tyrus db check            Compare the SQLite schema with its snapshot
  tyrus db diff             Show Model, snapshot, and database differences
  tyrus db explain-sql [SQL] Normalize SQL and show inferred parameter types
  tyrus db migrate create <name>  Create a numbered SQL migration file for the configured backend
  tyrus db migrate generate <name> Generate a SQLite or initial PostgreSQL/MySQL migration from @model declarations
  tyrus db migrate status         Show applied and pending SQLite migrations
  tyrus db migrate apply          Apply pending SQLite migrations atomically
  tyrus db seed status [name]     Show applied and pending Tyrus seeds
  tyrus db apply <name.tyr>       Run one Tyrus seed from db/seeds/
  tyrus mcp install               Configure MCP clients and download Tyrus documentation
  tyrus mcp update                Refresh MCP documentation and client configuration
  tyrus mcp status                Show MCP cache and configured clients
  tyrus mcp uninstall             Remove Tyrus MCP configuration and cache
  tyrus mcp serve                 Run the MCP server over stdio (started by clients)
  tyrus build [<path>] [--release] [--rotate-seal-key] [--sandbox] [--compile-target native|wasm]
               [--pgo-generate | --pgo-use]
                            Compile the project in the current directory
                            --sandbox skips env loading (for playground use)
                            --pgo-generate builds an instrumented binary that writes a profile
                            --pgo-use builds against the merged profile (needs --release)
  tyrus pgo-merge           Merge the profiles an instrumented run wrote into one file
  tyrus check               Type-check and borrow-check the project without generating code
  tyrus test [<path>]       Run every `*.test.tyr` file in the project and report pass/fail
                           <path> runs one test file, or every test file under a directory
  tyrus run [<path>|<script>] [--release] [--rotate-seal-key] [-- <args>...]
                           Run a named tyrus.json script or compile and run a Tyrus entry
                           Arguments after `--` are passed to the script or program
  tyrus watch [--run | --test | --command <cmd>] [--debounce <ms>] [--clear]
                           Rebuild whenever a `.tyr` file or tyrus.json changes
                           --run starts the program after each successful build
                           --test runs the tests; --command runs a shell command instead
  tyrus dev [--debounce <ms>] [--clear]
                           Rebuild and restart the project after every source change
  tyrus fmt [--check] [--stdout] [--init] [--default] [<path>...]
                           Format source files in the project
                           --init adds any missing options to tyrus-fmt.json (keeps existing)
                           --default resets tyrus-fmt.json to default options
  tyrus lint [--fix] [--format json] [--max-warnings <n>] [--explain <rule>] [--init] [--default] [<path>...]
                           Lint source files in the project
                           --init adds any missing rules to tyrus-lint.json (keeps existing)
                           --default resets tyrus-lint.json to default severities
  tyrus explain <error-code>
                           Explain a public compiler diagnostic code such as E0308
  tyrus repl                 Start an interactive REPL session
  tyrus lsp                  Start the Language Server over stdio
  tyrus --version, -v       Print version
  tyrus --help, -h          Print this help
"#
    );
    process::exit(0);
}

fn validate_build_args(args: &[String]) -> Result<Option<PathBuf>, String> {
    let mut index = 0;
    let mut entry = None;
    while index < args.len() {
        match args[index].as_str() {
            "--release" | "--rotate-seal-key" | "--sandbox" | "--pgo-generate" | "--pgo-use" => {
                index += 1
            }
            "--compile-target" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("error: `--compile-target` requires `native` or `wasm`".into());
                };
                if !matches!(value.as_str(), "native" | "wasm") {
                    return Err(format!(
                        "error: unknown compile target `{value}` (expected `native` or `wasm`)"
                    ));
                }
                index += 2;
            }
            flag if flag.starts_with('-') => {
                return Err(format!("error: unknown option `{flag}`"));
            }
            value => {
                if entry.replace(PathBuf::from(value)).is_some() {
                    return Err(format!("error: unexpected argument `{value}`"));
                }
                index += 1;
            }
        }
    }
    Ok(entry)
}

pub(super) fn run() {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(std::string::String::as_str) {
        Some("new") => {
            let target_arg = args
                .windows(2)
                .find(|w| w[0] == "--target")
                .map(|w| w[1].clone());
            let database_arg = args
                .windows(2)
                .find(|w| w[0] == "--database")
                .map(|w| w[1].clone());
            let name_arg = args.iter().enumerate().skip(2).find_map(|(index, arg)| {
                if arg.starts_with('-')
                    || index > 2 && matches!(args[index - 1].as_str(), "--target" | "--database")
                {
                    None
                } else {
                    Some(arg.clone())
                }
            });
            cmd_new(
                name_arg.as_deref(),
                target_arg.as_deref(),
                database_arg.as_deref(),
            );
        }
        Some("build") => {
            let entry = validate_build_args(&args[2..]).unwrap_or_else(|error| {
                eprintln!("{error}\nRun `tyrus --help` for usage.");
                process::exit(1);
            });
            let dir = env::current_dir().unwrap();
            let compile_target = parse_compile_target(&args[2..]);
            let sandbox = args.iter().any(|arg| arg == "--sandbox");
            let release = args.iter().any(|arg| arg == "--release");
            let pgo = parse_pgo_mode(&args[2..], release);
            cmd_build(
                &dir,
                release,
                args.iter().any(|arg| arg == "--rotate-seal-key"),
                false,
                entry.as_deref(),
                compile_target,
                sandbox,
                pgo,
            );
        }
        Some("pgo-merge") => {
            let dir = env::current_dir().unwrap();
            match compile::merge_pgo_profiles(&dir.join("target")) {
                Ok(path) => println!("  Merged profile → {}", path.display()),
                Err(message) => {
                    eprintln!("{message}");
                    process::exit(1);
                }
            }
        }
        Some("check") => {
            let dir = env::current_dir().unwrap();
            cmd_check(&dir);
        }
        Some("test") => {
            let dir = env::current_dir().unwrap();
            cmd_test(&dir, &args[2..]);
        }
        Some("db") => {
            let dir = env::current_dir().unwrap();
            db::run(&dir, &args[2..]);
        }
        Some("mcp") => mcp::run(&args[2..]),
        Some("info") => {
            let dir = env::current_dir().unwrap();
            cmd_info(&dir);
        }
        Some("clean") => super::clean::run(&args[2..]),
        Some("--default") => {
            let dir = env::current_dir().unwrap();
            cmd_default(&dir);
        }
        Some("--init") => {
            let dir = env::current_dir().unwrap();
            cmd_init(&dir);
        }
        Some("run") => {
            let dir = env::current_dir().unwrap();
            if let Some(script_name) = args.get(2) {
                match run_project_script(&dir, script_name, args.get(3..).unwrap_or(&[]))
                    .unwrap_or_else(|error| {
                        eprintln!("error: {error}");
                        process::exit(1);
                    }) {
                    Some(status) if status.success() => return,
                    Some(status) => process::exit(status.code().unwrap_or(1)),
                    None => {}
                }
            }
            let compile_target = parse_compile_target(&args[2..]);
            let (entry, release, rotate_seal_key, forwarded) = parse_run_args(&args[2..])
                .unwrap_or_else(|error| {
                    eprintln!("error: {error}\nRun `tyrus --help` for usage.");
                    process::exit(1);
                });
            let binary = cmd_build(
                &dir,
                release,
                rotate_seal_key,
                false,
                entry.as_deref(),
                compile_target,
                false,
                compile::PgoMode::Off,
            );
            cmd_run(&binary, &forwarded);
        }
        Some("fmt") => {
            let dir = env::current_dir().unwrap();
            if args.iter().any(|arg| arg == "--default") {
                cmd_fmt_default(&dir);
            } else if args.iter().any(|arg| arg == "--init") {
                cmd_fmt_init(&dir);
            } else {
                let check = args.iter().any(|arg| arg == "--check");
                let stdout = args.iter().any(|arg| arg == "--stdout");
                let files: Vec<&str> = args
                    .iter()
                    .skip(2)
                    .filter(|a| !a.starts_with('-'))
                    .map(std::string::String::as_str)
                    .collect();
                cmd_fmt(&dir, &files, check, stdout);
            }
        }
        Some("lint") => {
            let dir = env::current_dir().unwrap();
            let args: Vec<String> = args.iter().skip(2).cloned().collect();
            cmd_lint(&dir, &args);
        }
        Some("watch") => {
            let dir = env::current_dir().unwrap();
            let options = watch::parse_watch_args(&args[2..]).unwrap_or_else(|error| {
                eprintln!("error: {error}\nRun `tyrus --help` for usage.");
                process::exit(1);
            });
            watch::run(&dir, options);
        }
        Some("dev") => {
            let dir = env::current_dir().unwrap();
            let options = watch::parse_dev_args(&args[2..]).unwrap_or_else(|error| {
                eprintln!("error: {error}\nRun `tyrus --help` for usage.");
                process::exit(1);
            });
            watch::run(&dir, options);
        }
        Some("explain") => cmd_explain(args.get(2).map(String::as_str)),
        Some("repl") => repl::run(),
        Some("lsp") => tyrus_lsp::run(),
        Some("--version" | "-v") => {
            println!("tyrus 0.1.0");
        }
        Some("--help" | "-h") | None => usage(),
        Some(other) => {
            eprintln!("error: unknown command `{other}`\nRun `tyrus --help` for usage.");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_build_args;
    use std::path::PathBuf;

    fn strings(args: &[&str]) -> Vec<String> {
        args.iter().map(|arg| (*arg).to_string()).collect()
    }

    #[test]
    fn build_args_accept_release_and_compile_target() {
        assert_eq!(
            validate_build_args(&strings(&["--release", "--compile-target", "native"])).unwrap(),
            None
        );
    }

    #[test]
    fn build_args_accept_entry_path() {
        assert_eq!(
            validate_build_args(&strings(&["src/main.tyr"])).unwrap(),
            Some(PathBuf::from("src/main.tyr"))
        );
    }

    #[test]
    fn build_args_reject_typoed_release_flag() {
        let error = validate_build_args(&strings(&["--realease"]))
            .expect_err("unknown build flags should fail");
        assert!(error.contains("unknown option `--realease`"));
    }

    #[test]
    fn build_args_reject_missing_compile_target() {
        let error = validate_build_args(&strings(&["--compile-target"]))
            .expect_err("compile target requires a value");
        assert!(error.contains("requires `native` or `wasm`"));
    }
}
