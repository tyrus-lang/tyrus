use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct WatchProcess(Child);

impl Drop for WatchProcess {
    fn drop(&mut self) {
        if self.0.try_wait().is_ok_and(|status| status.is_none()) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

enum OutputLine {
    Stderr(String),
}

#[cfg(unix)]
#[test]
fn unix_signal_hook_reports_sigint_as_pending() {
    let mut signals = signal_hook::iterator::Signals::new([signal_hook::consts::SIGINT]).unwrap();
    let result = unsafe { libc::kill(libc::getpid(), libc::SIGINT) };
    assert_eq!(result, 0);
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if signals.pending().next().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("the signal hook must expose SIGINT to the watch loop");
}

fn spawn_reader<R: std::io::Read + Send + 'static>(
    reader: R,
    sender: mpsc::Sender<OutputLine>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else {
                return;
            };
            if sender.send(OutputLine::Stderr(line)).is_err() {
                return;
            }
        }
    })
}

fn send_interrupt(child: &Child) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGINT) };
        if result == 0 {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    #[cfg(windows)]
    {
        let sent = unsafe {
            windows_sys::Win32::System::Console::GenerateConsoleCtrlEvent(
                windows_sys::Win32::System::Console::CTRL_BREAK_EVENT,
                child.id(),
            )
        } != 0;
        if sent {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = child;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "watch interrupt testing is unsupported on this target",
        ))
    }
}

#[test]
fn watch_captures_interrupt_and_stops_the_running_program() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "Tyrus Watch 東京 with spaces-{}-{suffix}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("tyrus.json"),
        r#"{"project":{"name":"watch-interrupt-test","version":"0.1.0","authors":[],"description":"watch interrupt test"},"dependencies":{}}"#,
    )
    .unwrap();
    fs::write(
        root.join("src/main.tyr"),
        r#"import { sleepAsync } from "tyrus:task";

async function main(): Promise<void> {
    console.log("watch child started");
    await sleepAsync(60000);
}
"#,
    )
    .unwrap();

    let mut command = Command::new(env!("CARGO_BIN_EXE_tyrus"));
    command
        .args(["watch", "--run", "--debounce", "50"])
        .current_dir(&root)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP);
    }
    let mut child = WatchProcess(command.spawn().unwrap());
    let (sender, receiver) = mpsc::channel();
    let stderr_reader = spawn_reader(child.0.stderr.take().unwrap(), sender);

    let deadline = Instant::now() + Duration::from_secs(90);
    let mut stderr = String::new();
    let mut watcher_ready = false;
    while Instant::now() < deadline && !watcher_ready {
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(OutputLine::Stderr(line)) => {
                watcher_ready |= line.contains("[Waiting]");
                stderr.push_str(&line);
                stderr.push('\n');
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Some(status) = child.0.try_wait().unwrap() {
                    panic!("watch exited before starting its program: {status}; stderr: {stderr}");
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    assert!(
        watcher_ready,
        "watch did not start the program before the deadline; stderr: {stderr}"
    );

    send_interrupt(&child.0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    let status = loop {
        if let Some(status) = child.0.try_wait().unwrap() {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "watch did not exit after Ctrl+C; stderr: {stderr}"
        );
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(OutputLine::Stderr(line)) => {
                stderr.push_str(&line);
                stderr.push('\n');
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break child.0.wait().unwrap(),
        }
    };
    stderr_reader.join().unwrap();
    for line in receiver.try_iter() {
        let OutputLine::Stderr(line) = line;
        stderr.push_str(&line);
        stderr.push('\n');
    }
    assert!(
        status.success(),
        "watch exited with {status}; stderr: {stderr}"
    );
    assert!(
        !stderr.contains("graceful shutdown exceeded"),
        "watch had to force-terminate its child: {stderr}"
    );
    let _ = fs::remove_dir_all(root);
}
