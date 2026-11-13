use super::{cmd_build, codegen, compile};
use std::env;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct TestOptions {
    pub(super) entry: Option<PathBuf>,
}

pub(super) fn test_options_from(args: &[String]) -> Result<TestOptions, String> {
    let mut options = TestOptions::default();
    for arg in args {
        let arg = arg.as_str();
        match arg {
            other if other.starts_with('-') => {
                return Err(format!("error: unknown option `{other}` for `tyrus test`"));
            }
            path => {
                if options.entry.is_some() {
                    return Err(format!(
                        "error: `tyrus test` takes at most one path, and `{path}` is the second"
                    ));
                }
                options.entry = Some(PathBuf::from(path));
            }
        }
    }
    Ok(options)
}

pub(super) fn discover_test_files(dir: &Path) -> Vec<PathBuf> {
    fn walk(path: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if kind.is_dir() {
                if name.starts_with('.') || name == "target" {
                    continue;
                }
                walk(&entry.path(), found);
            } else if kind.is_file() && tyrus_core::test::is_test_file(&entry.path()) {
                found.push(entry.path());
            }
        }
    }

    let mut found = Vec::new();
    walk(dir, &mut found);
    found.sort();
    found
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct TestCounts {
    pub(super) passed: u64,
    pub(super) failed: u64,
    pub(super) skipped: u64,
    pub(super) todo: u64,
}

pub(super) const TEST_SUMMARY_SEPARATOR: &str = "----------------------------------------";

pub(super) fn format_test_count(count: u64, label: &str, color_code: &str, color: bool) -> String {
    let number = if color && count > 0 {
        format!("\u{1b}[1;{color_code}m{count}\u{1b}[0m")
    } else {
        count.to_string()
    };
    format!("{number} {label}")
}

pub(super) fn format_test_counts(counts: TestCounts, color: bool) -> String {
    let mut summary = format!(
        "Tests: {}, {}",
        format_test_count(counts.passed, "passed", "32", color),
        format_test_count(counts.failed, "failed", "31", color)
    );
    if counts.skipped > 0 {
        summary.push_str(&format!(
            ", {}",
            format_test_count(counts.skipped, "skipped", "33", color)
        ));
    }
    if counts.todo > 0 {
        summary.push_str(&format!(
            ", {}",
            format_test_count(counts.todo, "todo", "90", color)
        ));
    }
    summary
}

pub(super) fn test_summary_color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

pub(super) fn parse_test_report(text: &str) -> Option<TestCounts> {
    let mut fields = text
        .split_whitespace()
        .map(|field| field.parse::<u64>().ok());
    let counts = TestCounts {
        passed: fields.next()??,
        failed: fields.next()??,
        skipped: fields.next()??,
        todo: fields.next()??,
    };
    fields.next().is_none().then_some(counts)
}

/// Build the project in test mode and run the resulting binaries.
///
/// Every test file is its own binary and its own process. A file that crashes
/// (a segfault, an abort from a runtime check) takes down only its own run,
/// is reported as a failed file, and the remaining files still run.
pub(super) fn cmd_test(dir: &Path, args: &[String]) {
    let options = test_options_from(args).unwrap_or_else(|message| {
        eprintln!("{message}");
        process::exit(1);
    });

    if !dir.join("tyrus.json").exists() {
        eprintln!(
            "error: `tyrus.json` not found in `{}`\nhelp: run `tyrus new <name>` to create a project, or run `tyrus test` from the project root",
            dir.display()
        );
        process::exit(1);
    }

    let entries = match &options.entry {
        Some(entry) => {
            let resolved = dir.join(entry);
            if resolved.is_dir() {
                discover_test_files(&resolved)
            } else if !resolved.exists() {
                eprintln!("error: `{}` not found", entry.display());
                process::exit(1);
            } else if !tyrus_core::test::is_test_file(&resolved) {
                let stem = resolved
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("example");
                eprintln!(
                    "error: `{}` is not a test file\nhelp: test files are named `*.test.tyr`, for example `__tests__/{stem}.test.tyr`",
                    entry.display()
                );
                process::exit(1);
            } else {
                vec![resolved]
            }
        }
        None => discover_test_files(dir),
    };
    if entries.is_empty() {
        eprintln!(
            "No tests found.\nhelp: a test file is any file named `*.test.tyr`; `__tests__/` is the conventional folder for them"
        );
        process::exit(1);
    }

    let mut environment: Vec<(&str, String)> = Vec::new();
    environment.push((
        "TYRUS_TEST_ROOT",
        dir.canonicalize()
            .unwrap_or_else(|_| dir.to_path_buf())
            .to_string_lossy()
            .into_owned(),
    ));

    let jobs = effective_jobs(entries.len());
    if jobs > 1 {
        run_test_files_in_parallel(dir, &entries, jobs);
        return;
    }

    let inherited_report = env::var_os("TYRUS_TEST_REPORT").map(PathBuf::from);
    let is_child = env::var_os("TYRUS_TEST_CHILD").is_some();
    let report_path = inherited_report.clone().unwrap_or_else(|| {
        dir.join("target")
            .join(format!(".tyrus-test-report-{}", process::id()))
    });
    environment.push((
        "TYRUS_TEST_REPORT",
        report_path.to_string_lossy().into_owned(),
    ));
    let single = entries.len() == 1;

    let mut total = TestCounts::default();
    let mut files_passed = 0u64;
    let mut files_failed = 0u64;
    let started = std::time::Instant::now();
    for entry in &entries {
        let display = entry
            .strip_prefix(dir)
            .unwrap_or(entry)
            .display()
            .to_string()
            .replace('\\', "/");
        let binary = build_test_entry(dir, entry);
        environment.retain(|(name, _)| *name != "TYRUS_TEST_SOURCE");
        let source = entry.canonicalize().unwrap_or_else(|_| entry.clone());
        environment.push(("TYRUS_TEST_SOURCE", source.to_string_lossy().into_owned()));
        if !single {
            println!("\n{display}");
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }

        let _ = fs::remove_file(&report_path);
        let status = run_test_binary(&binary, &environment);
        let counts = fs::read_to_string(&report_path)
            .ok()
            .as_deref()
            .and_then(parse_test_report);
        if inherited_report.is_none() {
            let _ = fs::remove_file(&report_path);
        }

        let succeeded = status.is_some_and(|status| status.success());
        match counts {
            Some(counts) => {
                total.passed += counts.passed;
                total.failed += counts.failed;
                total.skipped += counts.skipped;
                total.todo += counts.todo;
            }
            None if succeeded => {}
            None => eprintln!("error: `{display}` exited before reporting its results"),
        }
        if succeeded {
            files_passed += 1;
        } else {
            files_failed += 1;
        }
    }

    let elapsed = started.elapsed();
    if total.passed + total.failed + total.skipped + total.todo == 0
        && files_failed == 0
        && !is_child
    {
        eprintln!(
            "No tests found.\nhelp: register tests with `test(\"name\", () => {{ ... }})` from `tyrus:test` in a `*.test.tyr` file"
        );
        process::exit(1);
    }
    if single {
        if files_failed > 0 {
            process::exit(1);
        }
        return;
    }
    let color = test_summary_color_enabled();
    println!("\n{TEST_SUMMARY_SEPARATOR}");
    println!(
        "Test files: {}, {} ({})",
        format_test_count(files_passed, "passed", "32", color),
        format_test_count(files_failed, "failed", "31", color),
        entries.len(),
    );
    let tests = format_test_counts(total, color);
    println!("{tests} ({:.2}s)", elapsed.as_secs_f64());
    if files_failed > 0 {
        process::exit(1);
    }
}

pub(super) fn effective_jobs(files: usize) -> usize {
    std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .clamp(1, files.max(1))
}

pub(super) struct FileRun {
    output: Option<process::Output>,
    counts: Option<TestCounts>,
}

pub(super) fn run_test_files_in_parallel(dir: &Path, entries: &[PathBuf], jobs: usize) {
    let color = test_summary_color_enabled();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let runs: Vec<std::sync::Mutex<Option<FileRun>>> = (0..entries.len())
        .map(|_| std::sync::Mutex::new(None))
        .collect();
    let started = std::time::Instant::now();

    std::thread::scope(|scope| {
        for _ in 0..jobs {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(entry) = entries.get(index) else {
                        break;
                    };
                    let report = dir
                        .join("target")
                        .join(format!(".tyrus-test-report-{}-{index}", process::id()));
                    let _ = fs::remove_file(&report);
                    let output = run_test_file_child(dir, entry, &report, color);
                    let counts = fs::read_to_string(&report)
                        .ok()
                        .as_deref()
                        .and_then(parse_test_report);
                    let _ = fs::remove_file(&report);
                    *runs[index].lock().unwrap() = Some(FileRun { output, counts });
                }
            });
        }
    });

    let mut total = TestCounts::default();
    let mut files_passed = 0u64;
    let mut files_failed = 0u64;
    for (entry, run) in entries.iter().zip(runs) {
        let display = entry
            .strip_prefix(dir)
            .unwrap_or(entry)
            .display()
            .to_string()
            .replace('\\', "/");
        println!("\n{display}");
        let Some(run) = run.into_inner().unwrap() else {
            eprintln!("error: `{display}` was never run");
            files_failed += 1;
            continue;
        };
        let succeeded = match &run.output {
            Some(output) => {
                print!("{}", String::from_utf8_lossy(&output.stdout));
                eprint!("{}", String::from_utf8_lossy(&output.stderr));
                output.status.success()
            }
            None => {
                eprintln!("error: could not run `{display}`");
                false
            }
        };
        match run.counts {
            Some(counts) => {
                total.passed += counts.passed;
                total.failed += counts.failed;
                total.skipped += counts.skipped;
                total.todo += counts.todo;
            }
            None if succeeded => {}
            None => eprintln!("error: `{display}` exited before reporting its results"),
        }
        if succeeded {
            files_passed += 1;
        } else {
            files_failed += 1;
        }
    }

    let elapsed = started.elapsed();
    if total.passed + total.failed + total.skipped + total.todo == 0 && files_failed == 0 {
        eprintln!(
            "No tests found.\nhelp: register tests with `test(\"name\", () => {{ ... }})` from `tyrus:test` in a `*.test.tyr` file"
        );
        process::exit(1);
    }
    println!("\n{TEST_SUMMARY_SEPARATOR}");
    println!(
        "Test files: {}, {} ({})",
        format_test_count(files_passed, "passed", "32", color),
        format_test_count(files_failed, "failed", "31", color),
        entries.len(),
    );
    let tests = format_test_counts(total, color);
    println!("{tests} ({:.2}s)", elapsed.as_secs_f64());
    if files_failed > 0 {
        process::exit(1);
    }
}

pub(super) fn run_test_file_child(
    dir: &Path,
    entry: &Path,
    report: &Path,
    color: bool,
) -> Option<process::Output> {
    let executable = env::current_exe().ok()?;
    let relative = entry.strip_prefix(dir).unwrap_or(entry);
    let mut command = process::Command::new(executable);
    command.arg("test").arg(relative).current_dir(dir);
    command.env("TYRUS_TEST_COLOR", if color { "always" } else { "never" });
    command.env("TYRUS_TEST_CHILD", "1");
    command.env("TYRUS_TEST_REPORT", report);
    command.output().ok()
}

pub(super) fn build_test_entry(dir: &Path, entry: &Path) -> PathBuf {
    cmd_build(
        dir,
        false,
        false,
        true,
        Some(entry),
        codegen::CompileTarget::Native,
        false,
        compile::PgoMode::Off,
    )
}

pub(super) fn run_test_binary(
    binary: &Path,
    environment: &[(&str, String)],
) -> Option<process::ExitStatus> {
    let mut command = process::Command::new(binary);
    for (name, value) in environment {
        command.env(name, value);
    }
    match command.status() {
        Ok(status) => Some(status),
        Err(error) => {
            eprintln!("error: could not run `{}`: {error}", binary.display());
            None
        }
    }
}

pub(super) fn cmd_run(binary: &Path, forwarded: &[String]) {
    cmd_run_with_env(binary, forwarded, &[]);
}

pub(super) fn cmd_run_with_env(
    binary: &Path,
    forwarded: &[String],
    environment: &[(&str, String)],
) {
    let mut command = process::Command::new(binary);
    command.args(forwarded);
    for (name, value) in environment {
        command.env(name, value);
    }
    let status = command.status().unwrap_or_else(|e| {
        eprintln!("error: could not run `{}`: {e}", binary.display());
        process::exit(1);
    });

    if !status.success() {
        process::exit(status.code().unwrap_or(1));
    }
}
