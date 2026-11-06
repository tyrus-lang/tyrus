use std::path::{Path, PathBuf};
use std::process::{self, Child, Command};
use std::sync::mpsc;
use std::time::{Duration, Instant};

const DEFAULT_DEBOUNCE_MILLIS: u64 = 300;
const RESTART_GRACE_MILLIS: u64 = 5_000;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum WatchAction {
    Build,
    Run,
    Test,
    Command(String),
}

enum WatchMessage {
    Filesystem(notify::Result<notify::Event>),
    Shutdown,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct WatchOptions {
    pub(crate) action: WatchAction,
    pub(crate) debounce: Duration,
    pub(crate) clear: bool,
}

pub(crate) fn parse_watch_args(args: &[String]) -> Result<WatchOptions, String> {
    let mut action = WatchAction::Build;
    let mut chosen: Option<&str> = None;
    let mut debounce = Duration::from_millis(DEFAULT_DEBOUNCE_MILLIS);
    let mut clear = false;
    let mut index = 0;
    let choose = |flag: &'static str, chosen: &mut Option<&str>| -> Result<(), String> {
        match chosen {
            Some(previous) => Err(format!(
                "`{flag}` cannot be combined with `{previous}`; choose one action for `tyrus watch`"
            )),
            None => {
                *chosen = Some(flag);
                Ok(())
            }
        }
    };
    while index < args.len() {
        match args[index].as_str() {
            "--run" => {
                choose("--run", &mut chosen)?;
                action = WatchAction::Run;
            }
            "--test" => {
                choose("--test", &mut chosen)?;
                action = WatchAction::Test;
            }
            "--command" => {
                choose("--command", &mut chosen)?;
                index += 1;
                let command = args
                    .get(index)
                    .filter(|command| !command.trim().is_empty())
                    .ok_or(
                        "`--command` expects the command to run, as in `--command \"tyrus lint\"`",
                    )?;
                action = WatchAction::Command(command.clone());
            }
            "--debounce" => {
                index += 1;
                let millis = args
                    .get(index)
                    .and_then(|value| value.parse::<u64>().ok())
                    .ok_or(
                        "`--debounce` expects a number of milliseconds, as in `--debounce 500`",
                    )?;
                debounce = Duration::from_millis(millis);
            }
            "--clear" => clear = true,
            other => return Err(format!("unknown `tyrus watch` option `{other}`")),
        }
        index += 1;
    }
    Ok(WatchOptions {
        action,
        debounce,
        clear,
    })
}

pub(crate) fn parse_dev_args(args: &[String]) -> Result<WatchOptions, String> {
    let mut forwarded = Vec::with_capacity(args.len() + 1);
    forwarded.push("--run".to_string());
    forwarded.extend(args.iter().cloned());
    parse_watch_args(&forwarded)
}

pub(crate) struct IgnoreRules {
    entries: Vec<String>,
}

impl IgnoreRules {
    pub(crate) fn load(root: &Path) -> Self {
        let entries = std::fs::read_to_string(root.join(".tyrusignore"))
            .unwrap_or_default()
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.trim_matches('/').to_string())
            .filter(|line| !line.is_empty())
            .collect();
        Self { entries }
    }

    pub(crate) fn ignores(&self, relative: &Path) -> bool {
        let text = relative.to_string_lossy().replace('\\', "/");
        let always = ["target", ".git"];
        let components: Vec<&str> = text.split('/').collect();
        if components
            .iter()
            .any(|component| always.contains(component))
        {
            return true;
        }
        self.entries.iter().any(|entry| {
            text == *entry
                || text.starts_with(&format!("{entry}/"))
                || (!entry.contains('/') && components.contains(&entry.as_str()))
        })
    }
}

pub(crate) fn is_relevant(root: &Path, path: &Path, ignore: &IgnoreRules) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let watched = path.extension().is_some_and(|extension| extension == "tyr")
        || relative == Path::new("tyrus.json");
    watched && !ignore.ignores(relative)
}

fn changed_paths(
    event: notify::Result<notify::Event>,
    root: &Path,
    ignore: &IgnoreRules,
) -> Vec<PathBuf> {
    let Ok(event) = event else {
        return Vec::new();
    };
    if matches!(event.kind, notify::EventKind::Access(_)) {
        return Vec::new();
    }
    event
        .paths
        .into_iter()
        .filter(|path| is_relevant(root, path, ignore))
        .collect()
}

fn count_sources(directory: &Path, root: &Path, ignore: &IgnoreRules) -> usize {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .map(|path| {
            if path.is_dir() {
                let relative = path.strip_prefix(root).unwrap_or(&path);
                if ignore.ignores(relative) {
                    0
                } else {
                    count_sources(&path, root, ignore)
                }
            } else {
                usize::from(is_relevant(root, &path, ignore))
            }
        })
        .sum()
}

fn project_name(root: &Path) -> Option<String> {
    let manifest = std::fs::read_to_string(root.join("tyrus.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&manifest).ok()?;
    value
        .get("project")?
        .get("name")?
        .as_str()
        .map(str::to_string)
}

fn build_profile(root: &Path) -> &'static str {
    let manifest = std::fs::read_to_string(root.join("tyrus.json")).ok();
    let optimized = manifest
        .and_then(|manifest| serde_json::from_str::<serde_json::Value>(&manifest).ok())
        .and_then(|value| value.get("build")?.get("optimize")?.as_bool())
        .unwrap_or(false);
    if optimized { "release" } else { "debug" }
}

fn stop(running: &mut Option<Child>) {
    let Some(mut child) = running.take() else {
        return;
    };

    #[cfg(unix)]
    let graceful = {
        let sent = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) } == 0;
        sent && wait_for_graceful_exit(&mut child)
    };

    #[cfg(windows)]
    let graceful = {
        let sent = unsafe {
            windows_sys::Win32::System::Console::GenerateConsoleCtrlEvent(
                windows_sys::Win32::System::Console::CTRL_BREAK_EVENT,
                child.id(),
            )
        } != 0;
        sent && wait_for_graceful_exit(&mut child)
    };

    #[cfg(not(any(unix, windows)))]
    let graceful = false;

    if !graceful {
        if cfg!(any(unix, windows)) {
            eprintln!(
                "[Restart] graceful shutdown exceeded {RESTART_GRACE_MILLIS}ms; forcing the previous process"
            );
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn wait_for_graceful_exit(child: &mut Child) -> bool {
    let deadline = Instant::now() + Duration::from_millis(RESTART_GRACE_MILLIS);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(20));
            }
            Ok(None) | Err(_) => return false,
        }
    }
}

#[cfg(unix)]
type ShutdownSignals = signal_hook::iterator::Signals;

#[cfg(not(unix))]
type ShutdownSignals = ();

#[cfg(unix)]
fn register_shutdown_signals() -> Option<ShutdownSignals> {
    signal_hook::iterator::Signals::new([signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM])
        .ok()
}

fn receive_watch_message(
    receiver: &mpsc::Receiver<WatchMessage>,
    signals: &mut Option<ShutdownSignals>,
    timeout: Duration,
) -> Result<WatchMessage, mpsc::RecvTimeoutError> {
    #[cfg(unix)]
    if signals
        .as_mut()
        .is_some_and(|signals| signals.pending().next().is_some())
    {
        return Ok(WatchMessage::Shutdown);
    }

    receiver.recv_timeout(timeout.min(Duration::from_millis(100)))
}

fn shell(command: &str) -> Command {
    #[cfg(windows)]
    {
        let mut shell = Command::new("cmd");
        shell.args(["/C", command]);
        shell
    }
    #[cfg(not(windows))]
    {
        let mut shell = Command::new("sh");
        shell.args(["-c", command]);
        shell
    }
}

fn seconds(started: Instant) -> String {
    format!("{:.1}s", started.elapsed().as_secs_f64())
}

fn cycle(
    root: &Path,
    options: &WatchOptions,
    running: &mut Option<Child>,
    graceful_interrupt: bool,
) {
    stop(running);
    if options.clear {
        eprint!("\x1b[2J\x1b[H");
    }
    let tyrus = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("tyrus"));
    let started = Instant::now();
    match &options.action {
        WatchAction::Build | WatchAction::Run => {
            let built = Command::new(&tyrus)
                .arg("build")
                .current_dir(root)
                .status()
                .is_ok_and(|status| status.success());
            if !built {
                eprintln!("[Build]    failed ({})", seconds(started));
            } else {
                eprintln!("[Build]    OK ({})", seconds(started));
                if options.action == WatchAction::Run {
                    match project_name(root) {
                        Some(name) => {
                            let executable = crate::platform::executable_path(
                                &root.join("target").join(build_profile(root)).join(&name),
                            );
                            match watched_program_command(&executable, graceful_interrupt)
                                .current_dir(root)
                                .spawn()
                            {
                                Ok(child) => *running = Some(child),
                                Err(error) => {
                                    eprintln!("[Run]      could not start `{name}`: {error}");
                                }
                            }
                        }
                        None => eprintln!("[Run]      `tyrus.json` has no `project.name`"),
                    }
                }
            }
        }
        WatchAction::Test => {
            let passed = Command::new(&tyrus)
                .arg("test")
                .current_dir(root)
                .status()
                .is_ok_and(|status| status.success());
            let outcome = if passed { "passed" } else { "failed" };
            eprintln!("[Test]     {outcome} ({})", seconds(started));
        }
        WatchAction::Command(command) => match shell(command).current_dir(root).status() {
            Ok(status) if status.success() => {
                eprintln!("[Command]  OK ({})", seconds(started));
            }
            Ok(status) => eprintln!(
                "[Command]  exited with {} ({})",
                status
                    .code()
                    .map_or_else(|| "a signal".to_string(), |code| code.to_string()),
                seconds(started)
            ),
            Err(error) => eprintln!("[Command]  could not start: {error}"),
        },
    }
    eprintln!("[Waiting]  Press Ctrl+C to stop...");
}

#[cfg(windows)]
fn watched_program_command(executable: &Path, graceful_interrupt: bool) -> Command {
    let mut command = Command::new(executable);
    use std::os::windows::process::CommandExt;
    if graceful_interrupt {
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP);
    }
    command
}

#[cfg(not(windows))]
fn watched_program_command(executable: &Path, _graceful_interrupt: bool) -> Command {
    Command::new(executable)
}

#[cfg(not(unix))]
fn spawn_shutdown_listener(sender: mpsc::Sender<WatchMessage>) -> bool {
    let (ready_sender, ready_receiver) = mpsc::sync_channel(0);
    let listener = std::thread::Builder::new()
        .name("tyrus-watch-shutdown".to_string())
        .spawn(move || {
            #[cfg(windows)]
            {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                let Ok(runtime) = runtime else {
                    let _ = ready_sender.send(false);
                    return;
                };
                runtime.block_on(async move {
                    let (Ok(mut interrupt), Ok(mut break_signal)) = (
                        tokio::signal::windows::ctrl_c(),
                        tokio::signal::windows::ctrl_break(),
                    ) else {
                        let _ = ready_sender.send(false);
                        return;
                    };
                    let _ = ready_sender.send(true);
                    tokio::select! {
                        _ = interrupt.recv() => {},
                        _ = break_signal.recv() => {},
                    }
                });
            }
            #[cfg(not(windows))]
            {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                let Ok(runtime) = runtime else {
                    let _ = ready_sender.send(false);
                    return;
                };
                runtime.block_on(async move {
                    let _ = ready_sender.send(true);
                    let _ = tokio::signal::ctrl_c().await;
                });
            }
            let _ = sender.send(WatchMessage::Shutdown);
        });
    if listener.is_err() {
        return false;
    }
    ready_receiver.recv().unwrap_or(false)
}

pub(crate) fn run(root: &Path, options: WatchOptions) {
    if !root.join("tyrus.json").is_file() {
        eprintln!(
            "error: no `tyrus.json` in `{}`; run `tyrus watch` from a project directory",
            root.display()
        );
        process::exit(1);
    }
    let ignore = IgnoreRules::load(root);
    let (sender, receiver) = mpsc::channel();
    let mut watcher: Box<dyn notify::Watcher> = match notify::recommended_watcher({
        let sender = sender.clone();
        move |event| {
            let _ = sender.send(WatchMessage::Filesystem(event));
        }
    }) {
        Ok(watcher) => Box::new(watcher),
        Err(native_error) => {
            eprintln!(
                "warning: native file watching is unavailable ({native_error}); falling back to polling"
            );
            let config = notify::Config::default().with_poll_interval(Duration::from_millis(250));
            let poll_sender = sender.clone();
            match notify::PollWatcher::new(
                move |event| {
                    let _ = poll_sender.send(WatchMessage::Filesystem(event));
                },
                config,
            ) {
                Ok(watcher) => Box::new(watcher),
                Err(error) => {
                    eprintln!("error: could not start watching files: {error}");
                    process::exit(1);
                }
            }
        }
    };

    let mut directories = vec![root.join("src")];
    if options.action == WatchAction::Test {
        directories.push(root.join("tests"));
    }
    let directories: Vec<PathBuf> = directories
        .into_iter()
        .filter(|path| path.is_dir())
        .collect();
    for directory in &directories {
        if let Err(error) = watcher.watch(directory, notify::RecursiveMode::Recursive) {
            eprintln!("error: could not watch `{}`: {error}", directory.display());
            process::exit(1);
        }
    }
    let _ = watcher.watch(
        &root.join("tyrus.json"),
        notify::RecursiveMode::NonRecursive,
    );

    let sources: usize = directories
        .iter()
        .map(|directory| count_sources(directory, root, &ignore))
        .sum();
    let names = directories
        .iter()
        .map(|directory| {
            format!(
                "{}/",
                directory.strip_prefix(root).unwrap_or(directory).display()
            )
        })
        .collect::<Vec<_>>()
        .join(" and ");
    if sources == 0 {
        eprintln!("warning: no `.tyr` files found; watching for new ones");
    }
    eprintln!("[Watching] {sources} files in {names}");

    let mut running = None;
    #[cfg(unix)]
    let mut shutdown_signals = register_shutdown_signals();
    #[cfg(not(unix))]
    let mut shutdown_signals = None;
    #[cfg(unix)]
    let graceful_interrupt = shutdown_signals.is_some();
    #[cfg(not(unix))]
    let graceful_interrupt = spawn_shutdown_listener(sender);
    if !graceful_interrupt {
        eprintln!(
            "warning: could not install a graceful shutdown listener; Ctrl+C will terminate watched processes directly"
        );
    }
    cycle(root, &options, &mut running, graceful_interrupt);
    loop {
        let event = match receive_watch_message(
            &receiver,
            &mut shutdown_signals,
            Duration::from_millis(100),
        ) {
            Ok(WatchMessage::Filesystem(event)) => event,
            Ok(WatchMessage::Shutdown) => {
                eprintln!("[Stopping] interrupt received; stopping the watched program...");
                stop(&mut running);
                return;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                stop(&mut running);
                return;
            }
        };
        let mut changed = changed_paths(event, root, &ignore);
        if changed.is_empty() {
            continue;
        }
        let deadline = Instant::now() + options.debounce;
        while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            match receive_watch_message(&receiver, &mut shutdown_signals, remaining) {
                Ok(WatchMessage::Filesystem(event)) => {
                    changed.extend(changed_paths(event, root, &ignore));
                }
                Ok(WatchMessage::Shutdown) => {
                    eprintln!("[Stopping] interrupt received; stopping the watched program...");
                    stop(&mut running);
                    return;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => break,
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    stop(&mut running);
                    return;
                }
            }
        }
        changed.sort();
        changed.dedup();
        for path in &changed {
            eprintln!(
                "[Changed]  {}",
                path.strip_prefix(root).unwrap_or(path).display()
            );
        }
        cycle(root, &options, &mut running, graceful_interrupt);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values
            .iter()
            .map(std::string::ToString::to_string)
            .collect()
    }

    #[test]
    fn watch_defaults_to_building_with_the_default_debounce() {
        assert_eq!(
            parse_watch_args(&[]).unwrap(),
            WatchOptions {
                action: WatchAction::Build,
                debounce: Duration::from_millis(DEFAULT_DEBOUNCE_MILLIS),
                clear: false,
            }
        );
    }

    #[test]
    fn watch_reads_its_action_debounce_and_clear_options() {
        assert_eq!(
            parse_watch_args(&args(&[
                "--command",
                "tyrus lint",
                "--debounce",
                "500",
                "--clear"
            ]))
            .unwrap(),
            WatchOptions {
                action: WatchAction::Command("tyrus lint".to_string()),
                debounce: Duration::from_millis(500),
                clear: true,
            }
        );
        assert_eq!(
            parse_watch_args(&args(&["--run"])).unwrap().action,
            WatchAction::Run
        );
        assert_eq!(
            parse_watch_args(&args(&["--test"])).unwrap().action,
            WatchAction::Test
        );
    }

    #[test]
    fn dev_reuses_the_run_watcher_and_rejects_other_actions() {
        assert_eq!(
            parse_dev_args(&args(&["--debounce", "150", "--clear"])).unwrap(),
            WatchOptions {
                action: WatchAction::Run,
                debounce: Duration::from_millis(150),
                clear: true,
            }
        );
        assert!(
            parse_dev_args(&args(&["--test"]))
                .unwrap_err()
                .contains("cannot be combined")
        );
        assert!(
            parse_dev_args(&args(&["--command", "echo nope"]))
                .unwrap_err()
                .contains("cannot be combined")
        );
    }

    #[test]
    fn watch_refuses_two_actions_a_bad_debounce_and_unknown_options() {
        assert!(
            parse_watch_args(&args(&["--run", "--test"]))
                .unwrap_err()
                .contains("cannot be combined")
        );
        assert!(
            parse_watch_args(&args(&["--debounce", "soon"]))
                .unwrap_err()
                .contains("milliseconds")
        );
        assert!(
            parse_watch_args(&args(&["--command"]))
                .unwrap_err()
                .contains("expects the command")
        );
        assert!(
            parse_watch_args(&args(&["--watch-everything"]))
                .unwrap_err()
                .contains("--watch-everything")
        );
    }

    #[test]
    fn watch_reacts_to_sources_and_the_manifest_outside_ignored_paths() {
        let root = std::env::temp_dir().join(format!("tyrus-watch-rules-{}", process::id()));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join(".tyrusignore"),
            "# generated code is rebuilt by its own tool\nsrc/generated/\nfixtures\n",
        )
        .unwrap();
        let ignore = IgnoreRules::load(&root);
        let _ = std::fs::remove_dir_all(&root);

        assert!(is_relevant(&root, &root.join("src/main.tyr"), &ignore));
        assert!(is_relevant(&root, &root.join("tyrus.json"), &ignore));
        assert!(!is_relevant(&root, &root.join("src/notes.txt"), &ignore));
        assert!(!is_relevant(
            &root,
            &root.join("src/generated/api.tyr"),
            &ignore
        ));
        assert!(!is_relevant(
            &root,
            &root.join("tests/fixtures/case.tyr"),
            &ignore
        ));
        assert!(!is_relevant(
            &root,
            &root.join("target/debug/main.tyr"),
            &ignore
        ));
        assert!(!is_relevant(
            &root,
            &root.join("src/nested/tyrus.json"),
            &ignore
        ));
    }
}
