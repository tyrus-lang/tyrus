use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use tyrus_core::{env as tyrus_env, module_loader, project};

pub(super) fn cmd_fmt_default(dir: &Path) {
    let path = dir.join("tyrus-fmt.json");
    let existed = path.exists();
    fs::write(&path, tyrus_fmt::config::default_config_json()).unwrap_or_else(|e| {
        eprintln!("error: could not write tyrus-fmt.json: {e}");
        process::exit(1);
    });
    println!(
        "  {} tyrus-fmt.json with default options",
        if existed { "Reset" } else { "Created" }
    );
}

pub(super) fn cmd_fmt_init(dir: &Path) {
    let path = dir.join("tyrus-fmt.json");
    let existed = path.exists();
    let contents = match fs::read_to_string(&path) {
        Ok(existing) => tyrus_fmt::config::merge_config_json(&existing).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            process::exit(1);
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            tyrus_fmt::config::default_config_json()
        }
        Err(error) => {
            eprintln!("error: could not read tyrus-fmt.json: {error}");
            process::exit(1);
        }
    };
    fs::write(&path, contents).unwrap_or_else(|e| {
        eprintln!("error: could not write tyrus-fmt.json: {e}");
        process::exit(1);
    });
    println!(
        "  {} tyrus-fmt.json",
        if existed { "Updated" } else { "Created" }
    );
}

pub(super) fn cmd_fmt(dir: &Path, files: &[&str], check: bool, stdout: bool) {
    if files.is_empty() {
        let results = tyrus_fmt::format_project(dir).unwrap_or_else(|e| {
            eprintln!("{e}");
            process::exit(1);
        });

        let mut changed = false;
        for (path, formatted) in &results {
            let original = std::fs::read_to_string(path).unwrap_or_default();
            if original != *formatted {
                if check {
                    println!("  Would format {path}");
                    changed = true;
                } else {
                    std::fs::write(path, formatted).unwrap();
                    println!("  Formatted {path}");
                }
            }
        }

        if check && changed {
            process::exit(1);
        }
    } else {
        for file_path in files {
            let path = if Path::new(file_path).is_absolute() {
                PathBuf::from(file_path)
            } else {
                dir.join(file_path)
            };

            let original = std::fs::read_to_string(&path).unwrap_or_else(|e| {
                eprintln!("error: could not read `{file_path}`: {e}");
                process::exit(1);
            });

            let result = tyrus_fmt::format_file(&path, Some(dir));
            let formatted = match result {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("{e}");
                    process::exit(1);
                }
            };

            if stdout {
                print!("{formatted}");
            } else if original != formatted {
                if check {
                    println!("  Would format {file_path}");
                    process::exit(1);
                } else {
                    std::fs::write(&path, &formatted).unwrap();
                    println!("  Formatted {file_path}");
                }
            }
        }
    }
}

pub(super) fn cmd_lint(dir: &Path, args: &[String]) {
    let explain_pos = args.iter().position(|a| a == "--explain");
    let init = args.iter().any(|a| a == "--init");
    let reset_default = args.iter().any(|a| a == "--default");
    let fix = args.iter().any(|a| a == "--fix");
    let format_json = args
        .windows(2)
        .any(|w| w[0] == "--format" && w[1] == "json");
    let max_warnings: Option<usize> = args
        .windows(2)
        .find(|w| w[0] == "--max-warnings")
        .and_then(|w| w[1].parse().ok());

    if let Some(pos) = explain_pos {
        let rule_name = args.get(pos + 1).map_or("", std::string::String::as_str);
        let meta = tyrus_lint::config::rule_meta(rule_name);
        match meta {
            Some(meta) => {
                println!("{} ({})", meta.name, meta.category.label());
                println!("  Default severity: {}", meta.default.label());
                println!("  Fixable: {}", if meta.fixable { "yes" } else { "no" });
                println!("  {}", meta.description);
            }
            None => {
                eprintln!("error: unknown lint rule `{rule_name}`");
                process::exit(1);
            }
        }
        return;
    }

    if reset_default {
        let path = dir.join("tyrus-lint.json");
        let existed = path.exists();
        fs::write(&path, tyrus_lint::config::default_config_json()).unwrap_or_else(|e| {
            eprintln!("error: could not write tyrus-lint.json: {e}");
            process::exit(1);
        });
        println!(
            "  {} tyrus-lint.json with default severities",
            if existed { "Reset" } else { "Created" }
        );
        return;
    }

    if init {
        let path = dir.join("tyrus-lint.json");
        let contents = match fs::read_to_string(&path) {
            Ok(existing) => tyrus_lint::config::merge_config_json(&existing).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                process::exit(1);
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tyrus_lint::config::default_config_json()
            }
            Err(error) => {
                eprintln!("error: could not read tyrus-lint.json: {error}");
                process::exit(1);
            }
        };
        let existed = path.exists();
        fs::write(&path, contents).unwrap_or_else(|e| {
            eprintln!("error: could not write tyrus-lint.json: {e}");
            process::exit(1);
        });
        println!(
            "  {} tyrus-lint.json",
            if existed { "Updated" } else { "Created" }
        );
        return;
    }

    let config = tyrus_lint::config::LintConfig::load(dir).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });

    let mut skip_next = false;
    let explicit: Vec<&str> = args
        .iter()
        .filter(|a| {
            if skip_next {
                skip_next = false;
                return false;
            }
            if *a == "--format" || *a == "--max-warnings" || *a == "--explain" {
                skip_next = true;
                return false;
            }
            !a.starts_with('-')
        })
        .map(std::string::String::as_str)
        .collect();

    let reports = tyrus_lint::lint_paths(dir, &explicit, &config).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });

    if fix {
        for report in &reports {
            for diagnostic in &report.diagnostics {
                if let Some(lint_fix) = &diagnostic.fix {
                    let source = std::fs::read_to_string(&report.path).unwrap_or_default();
                    if lint_fix.range.end <= source.len() {
                        let mut bytes: Vec<u8> = source.bytes().collect();
                        bytes.splice(lint_fix.range.clone(), lint_fix.replacement.bytes());
                        std::fs::write(&report.path, bytes).unwrap_or_else(|e| {
                            eprintln!("error: could not write `{}`: {e}", report.path);
                        });
                        println!("  Fixed {} in {}", lint_fix.message, report.path);
                    }
                }
            }
        }
    }

    let final_reports = if fix {
        tyrus_lint::lint_paths(dir, &explicit, &config).unwrap_or_else(|e| {
            eprintln!("{e}");
            process::exit(1);
        })
    } else {
        reports
    };

    let mut has_errors = false;
    let mut warning_count = 0usize;

    let sources: Vec<(String, String)> = final_reports
        .iter()
        .filter_map(|report| {
            let source = std::fs::read_to_string(&report.path).ok()?;
            Some((report.path.clone(), source))
        })
        .collect();

    if format_json {
        println!("{}", tyrus_lint::render_json(&final_reports, &sources));
    } else {
        for report in &final_reports {
            let source = sources
                .iter()
                .find(|(path, _)| path == &report.path)
                .map_or("", |(_, source)| source.as_str());
            for diagnostic in &report.diagnostics {
                eprintln!(
                    "{}",
                    tyrus_lint::render_pretty(&report.path, source, diagnostic)
                );
                eprintln!();
                match diagnostic.severity {
                    tyrus_lint::Severity::Error => has_errors = true,
                    tyrus_lint::Severity::Warn => warning_count += 1,
                    _ => {}
                }
            }
        }
    }

    if let Some(max) = max_warnings
        && warning_count > max
    {
        has_errors = true;
    }

    if has_errors {
        process::exit(1);
    }
}

pub(super) fn cmd_check(dir: &Path) {
    let config = project::load_project(dir).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });
    let entry = dir.join("src/main.tyr");
    if !entry.exists() {
        eprintln!("error: required entry file `src/main.tyr` not found");
        process::exit(1);
    }
    println!(
        "  Checking {} v{}",
        config.project.name, config.project.version
    );

    let env_schema = tyrus_env::load_env_schema(dir, &config).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        process::exit(1);
    });
    let env_schema = match env_schema {
        Some((schema, diagnostics)) => {
            if !diagnostics.is_empty() {
                let rendered = diagnostics
                    .iter()
                    .map(|diagnostic| {
                        tyrus_env::render_env_diagnostic(
                            &schema.file_path,
                            &schema.source,
                            &schema.vars,
                            diagnostic,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                eprintln!("{rendered}");
                process::exit(1);
            }
            Some(schema)
        }
        None => None,
    };

    module_loader::load_program(&entry, env_schema.as_ref()).unwrap_or_else(|e| {
        eprintln!("{e}");
        process::exit(1);
    });

    println!("  Finished, no errors");
}
