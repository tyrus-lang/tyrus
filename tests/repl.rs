#![cfg(unix)]

//! Integration tests for the interactive tyrus REPL.
//!
//! These tests spawn a real `tyrus repl` process inside a PTY and drive it
//! like a terminal. Reedline queries the terminal for its cursor position on
//! every prompt (`ESC[6n`, the DSR report), so the harness answers those
//! queries with a cursor report the way a real terminal would; plain rexpect
//! sessions never respond and would time out on every prompt.

use rexpect::process::PtyProcess;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::io::AsRawFd;
use std::process::Command;
use std::time::{Duration, Instant};

const PROMPT_TIMEOUT: Duration = Duration::from_secs(20);

/// Serializes pty creation so that a descriptor is always marked close-on-exec
/// before any other thread can `fork`.
static PTY_SPAWN_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Marks every pty master this process holds close-on-exec, so a later `exec`
/// does not carry it into the new program.
///
/// `rexpect` opens the master with `posix_openpt(O_RDWR)` and no `O_CLOEXEC`,
/// so every process started afterwards inherits a copy. With this file's tests
/// running in parallel that is not a stray descriptor or two: a single
/// `tyrus repl` child was measured holding **22** `/dev/ptmx` descriptors, one
/// for every other session alive when it was forked. Each keeps another test's
/// pty from being released when that test ends, and the count grows with the
/// parallelism until it meets the per-process descriptor limit.
///
/// It sweeps `/proc/self/fd` rather than marking the two descriptors of the
/// session just created, because only one of them is reachable: `PtyProcess`
/// keeps its master private and exposes no `AsRawFd`, so the copy handed out by
/// `get_file_handle` is all the API offers. Sweeping also makes the call
/// idempotent, which is what lets it run once per spawn and still cover
/// sessions created before it.
///
/// Best-effort on purpose: failing here would fail a test for a reason
/// unrelated to the REPL, and the leak it prevents is a resource problem rather
/// than a correctness one. On a platform without `/proc` it does nothing.
///
/// # What leaked, exactly
///
/// Not the child's own pty: `PtyProcess::new` closes the master in the forked
/// child before it execs. What it inherited was every *other* session's master,
/// which is why the count tracked the parallelism rather than staying at one.
/// With the sweep in place a sampled child holds none, measured at 22 before
/// and 0 after, over nine sampled children.
///
fn close_on_exec_every_pty_master() {
    let Ok(entries) = fs::read_dir("/proc/self/fd") else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(target) = fs::read_link(entry.path()) else {
            continue;
        };
        if target != std::path::Path::new("/dev/ptmx") {
            continue;
        }
        let Some(fd) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<std::os::unix::io::RawFd>().ok())
        else {
            continue;
        };
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFD);
            if flags >= 0 {
                libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
            }
        }
    }
}

/// How long the REPL must stay silent before the harness treats a prompt as
/// finished drawing. See [`ReplSession::settle`].
const QUIET_WINDOW: Duration = Duration::from_millis(120);

/// A PTY session that answers cursor-position queries (DSR `ESC[6n`) so the
/// reedline-based REPL stays responsive, exposing the small rexpect-like API
/// (`send_line`, `exp_string`, `exp_eof`) the tests use.
struct ReplSession {
    // Kept alive for the whole session so dropping the session kills the child.
    _process: PtyProcess,
    // Each child needs isolated history and saved-session files. Keeping the
    // temporary directory here also guarantees HOME outlives the child.
    _home: tempfile::TempDir,
    master: std::fs::File,
    buffer: Vec<u8>,
}

impl ReplSession {
    /// Reads one byte, waiting at most until `deadline`, and answers any
    /// cursor-position query the REPL sends.
    fn read_byte(&mut self, deadline: Instant) -> Result<(), String> {
        let mut pollfd = libc::pollfd {
            fd: self.master.as_raw_fd(),
            events: libc::POLLIN | libc::POLLHUP,
            revents: 0,
        };
        let remaining = deadline.saturating_duration_since(Instant::now());
        let millis = remaining.as_millis().min(i32::MAX as u128) as i32;
        let ready = unsafe { libc::poll(&mut pollfd, 1, millis) };
        if ready == 0 {
            return Err("timed out".to_string());
        }
        if ready < 0 {
            return Err(format!("poll failed: {}", std::io::Error::last_os_error()));
        }
        let mut byte = [0u8; 1];
        match self.master.read(&mut byte) {
            Ok(0) => Err("pty closed".to_string()),
            Ok(_) => {
                self.buffer.push(byte[0]);
                if self.buffer.ends_with(b"\x1b[6n") {
                    self.buffer.truncate(self.buffer.len() - 4);
                    self.master
                        .write_all(b"\x1b[1;1R")
                        .map_err(|e| e.to_string())?;
                }
                Ok(())
            }
            Err(_) => Err("pty closed".to_string()),
        }
    }

    /// Waits until the accumulated output contains `needle`. ANSI escape
    /// sequences are stripped before matching, so tests assert on the plain
    /// text even when the REPL syntax-highlights its output.
    fn exp_string(&mut self, needle: &str) -> Result<(), String> {
        let deadline = Instant::now() + PROMPT_TIMEOUT;
        let needle = needle.as_bytes();
        while !strip_ansi(&self.buffer)
            .as_bytes()
            .windows(needle.len())
            .any(|window| window == needle)
        {
            match self.read_byte(deadline) {
                Ok(()) => {}
                Err(e) => {
                    let got = String::from_utf8_lossy(&self.buffer);
                    return Err(format!("waiting for `{needle:?}` failed ({e}); got: {got}"));
                }
            }
        }
        Ok(())
    }

    /// Waits until both `first` and `second` appear in the output (in any
    /// order), then asserts `first` appears before `second`, so tests can
    /// verify interleaved output like an echoed line followed by its result.
    fn exp_before(&mut self, first: &str, second: &str) -> Result<(), String> {
        let deadline = Instant::now() + PROMPT_TIMEOUT;
        while strip_ansi(&self.buffer).find(first).is_none()
            || strip_ansi(&self.buffer).find(second).is_none()
        {
            match self.read_byte(deadline) {
                Ok(()) => {}
                Err(e) => {
                    let got = String::from_utf8_lossy(&self.buffer);
                    return Err(format!(
                        "waiting for `{first:?}`/`{second:?}` failed ({e}); got: {got}"
                    ));
                }
            }
        }
        let clean = strip_ansi(&self.buffer);
        let first_at = clean.find(first).unwrap();
        let second_at = clean.find(second).unwrap();
        if first_at >= second_at {
            return Err(format!(
                "expected `{first:?}` before `{second:?}`; got: {clean:?}"
            ));
        }
        Ok(())
    }

    /// Waits until `needle` appears after the LAST occurrence of `marker` in
    /// the accumulated output. A marker can appear many times (a `:restore`
    /// replay echoes the same code that was typed earlier), so tests use this
    /// to wait for output produced only after the last occurrence, e.g. the
    /// idle prompt that follows a command's replay.
    fn exp_after_last(&mut self, marker: &str, needle: &str) -> Result<(), String> {
        let deadline = Instant::now() + PROMPT_TIMEOUT;
        loop {
            let clean = strip_ansi(&self.buffer);
            let mark_at = clean.rfind(marker);
            let found = mark_at.is_some_and(|at| clean[at + marker.len()..].find(needle).is_some());
            if found {
                return Ok(());
            }
            match self.read_byte(deadline) {
                Ok(()) => {}
                Err(e) => {
                    let got = String::from_utf8_lossy(&self.buffer);
                    return Err(format!(
                        "waiting for `{needle:?}` after `{marker:?}` failed ({e}); got: {got}"
                    ));
                }
            }
        }
    }

    /// Reads and answers everything the REPL has to say until it falls silent
    /// for [`QUIET_WINDOW`], so the next keystroke arrives at a prompt that has
    /// finished drawing.
    ///
    /// This is not politeness, it is the difference between a test that passes
    /// and one that loses its input. Reedline asks the terminal for the cursor
    /// position (`ESC[6n`) while drawing a prompt and waits for the report
    /// before reading a key; only this harness answers, and only while it is
    /// inside `read_byte`. A test that writes the next line without reading
    /// first leaves the REPL blocked on a report nobody is sending, and the
    /// bytes it finally reads are consumed as part of that reply rather than as
    /// input, the line vanishes with no error anywhere.
    ///
    /// The symptom was `repl_clear_then_undefined_error` and `repl_bench_basic`
    /// timing out at `exp_eof` in roughly one run out of three, with a
    /// transcript that simply stops after some earlier line: every statement
    /// after the lost one, `:exit` included, was never seen, so the child never
    /// exited and the PTY never closed.
    fn settle(&mut self) {
        let hard_deadline = Instant::now() + PROMPT_TIMEOUT;
        while Instant::now() < hard_deadline {
            let deadline = (Instant::now() + QUIET_WINDOW).min(hard_deadline);
            if self.read_byte(deadline).is_err() {
                return;
            }
        }
    }

    /// Sends a line followed by Enter. The REPL runs the terminal in raw mode,
    /// where the Enter key arrives as `\r` (CR); `\n` (LF) would be read as a
    /// Ctrl+J character instead of submitting the line.
    ///
    /// Nothing here waits for the REPL to catch up, which is deliberate: these
    /// lines can all be sitting in the pty at once, and that is the shape that
    /// used to break the REPL under load. See `send_burst` for the same
    /// pressure applied in a single write.
    fn send_line(&mut self, line: &str) -> Result<(), String> {
        self.settle();
        write!(self.master, "{line}\r").map_err(|e| e.to_string())
    }

    /// Sends several LF-delimited lines in one write, matching a terminal
    /// paste without bracketed-paste support.
    fn send_burst(&mut self, lines: &[&str]) -> Result<(), String> {
        let mut burst = lines.join("\n");
        burst.push('\n');
        self.master
            .write_all(burst.as_bytes())
            .map_err(|e| e.to_string())
    }

    /// Sends raw bytes to the REPL without any trailing Enter, for tests that
    /// need to interleave keystrokes (auto-close, skip, backspace) with echoes.
    fn send(&mut self, bytes: &str) -> Result<(), String> {
        self.settle();
        write!(self.master, "{bytes}").map_err(|e| e.to_string())
    }

    /// Sends a bracketed-paste chunk followed by Enter, the way a terminal
    /// pastes multi-character input with paste bracketing enabled. The REPL
    /// turns it into a single literal insert, so it never auto-closes.
    fn paste_line(&mut self, body: &str) -> Result<(), String> {
        self.settle();
        write!(self.master, "\x1b[200~{body}\x1b[201~\r").map_err(|e| e.to_string())
    }

    /// Waits until the child process exits (closing the PTY).
    fn exp_eof(&mut self) -> Result<(), String> {
        let deadline = Instant::now() + PROMPT_TIMEOUT;
        loop {
            match self.read_byte(deadline) {
                Ok(()) => {}
                Err(e) if e == "pty closed" => return Ok(()),
                Err(e) => {
                    let got = String::from_utf8_lossy(&self.buffer);
                    return Err(format!("waiting for EOF failed ({e}); got: {got}"));
                }
            }
        }
    }
}

/// Removes ANSI escape sequences from terminal output so tests can match the
/// plain text regardless of the syntax highlighting the REPL applies. This
/// covers CSI (`ESC [ ... `), OSC (`ESC ] ... BEL/ST`) and two-byte escapes
/// such as `ESC 7`/`ESC 8` (save/restore cursor) that reedline emits around the
/// cursor glyph. The PTY line discipline also translates `\n` to `\r\n`, so CR
/// characters are dropped to keep needles that span lines (`"Thomas\n"`)
/// unambiguous.
fn strip_ansi(bytes: &[u8]) -> String {
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b {
            match bytes.get(i + 1) {
                Some(&b'[') => {
                    // CSI: skip up to and including the final byte (0x40..=0x7e).
                    i += 2;
                    while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
                        i += 1;
                    }
                    i += 1;
                }
                Some(&b']') => {
                    // OSC: skip up to the terminating BEL or ST (`ESC \`).
                    i += 2;
                    while i < bytes.len()
                        && bytes[i] != 0x07
                        && !(bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'\\'))
                    {
                        i += 1;
                    }
                    if i < bytes.len() && bytes[i] == 0x07 {
                        i += 1;
                    } else {
                        i += 2;
                    }
                }
                Some(_) => {
                    // Two-byte escapes such as `ESC 7` (save cursor) and
                    // `ESC 8` (restore cursor).
                    i += 2;
                }
                None => i += 1,
            }
        } else if bytes[i] == b'\r' {
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn tyrus_binary() -> &'static str {
    option_env!("CARGO_BIN_EXE_tyrus").unwrap_or("tyrus")
}

/// Spawns a real interactive `tyrus repl` session inside a PTY and waits for
/// its first prompt, so the terminal is already in raw mode and the DSR
/// handshake is done before the test sends anything.
///
/// `HOME` is pointed at a temp directory so the REPL history and the
/// saved-session manifest (`~/.tyrus/repl-sessions.json`) never touch the real
/// user home.
fn spawn_repl() -> ReplSession {
    let home = tempfile::Builder::new()
        .prefix("tyrus-repl-home-")
        .tempdir()
        .expect("the REPL test HOME should be created");
    let mut cmd = Command::new(tyrus_binary());
    cmd.arg("repl").env("HOME", home.path());
    // Creating the pty and marking it close-on-exec have to be one step with
    // respect to other spawns, see `close_on_exec`. A poisoned lock says
    // nothing about the descriptors, so it is recovered rather than propagated.
    let (mut process, master) = {
        let _serialized = PTY_SPAWN_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let process = PtyProcess::new(cmd).unwrap();
        // `get_file_handle` reports failure since rexpect 0.6 instead of
        // panicking inside itself; there is nothing to recover to here, so the
        // test fails with the reason rather than on a later unexplained read.
        let master = process
            .get_file_handle()
            .expect("the PTY master handle should be available");
        close_on_exec_every_pty_master();
        (process, master)
    };
    process.set_kill_timeout(Some(2000));
    let mut session = ReplSession {
        _process: process,
        _home: home,
        master,
        buffer: Vec::new(),
    };
    session
        .exp_string("tyrus> ")
        .expect("the REPL should reach its first prompt");
    session
}

#[test]
fn repl_banner_and_exit() {
    let mut p = spawn_repl();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_basic_expression() {
    let mut p = spawn_repl();
    p.send_line("let x = 10;").unwrap();
    p.send_line("let y = 20;").unwrap();
    p.send_line("x + y").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

/// An `export`-led unit goes to the REPL's module section, which does not echo a
/// value; `export protect` has to behave the same way rather than erroring on the
/// modifier, and the declared name has to stay callable.
#[test]
fn repl_accepts_the_protect_export_modifier() {
    let mut p = spawn_repl();
    p.send_line("export protect const helper = (): i32 => 5;")
        .unwrap();
    p.send_line("print(helper() + 1);").unwrap();
    p.exp_string("6").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_preserves_lf_delimited_burst_input() {
    let mut p = spawn_repl();
    p.send_burst(&["let x = 10;", "let y = 20;", "console.log(x + y);", ":exit"])
        .unwrap();
    p.exp_string("30").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_console_log() {
    let mut p = spawn_repl();
    p.send_line(r#"console.log("hello world");"#).unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_syntax_error_does_not_crash() {
    let mut p = spawn_repl();
    p.send_line("let x = ").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_parse_error_reports_immediately() {
    let mut p = spawn_repl();
    // A stray `;` inside an unclosed call is a genuine parse error and must be
    // reported right away, without waiting for Enter on a continuation line.
    p.send_line("console.log(numbers[1];").unwrap();
    p.exp_string("error:").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_incomplete_line_enters_continuation() {
    let mut p = spawn_repl();
    p.send_line("function add(a: i32, b: i32): i32 {").unwrap();
    p.exp_string(".... ").unwrap();
    p.send_line("    return a + b;").unwrap();
    p.send_line("}").unwrap();
    p.send_line("add(3, 4)").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_type_error_does_not_crash() {
    let mut p = spawn_repl();
    p.send_line(r#""hello" + 5"#).unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_undefined_variable_error() {
    let mut p = spawn_repl();
    p.send_line("x + 1").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_help() {
    let mut p = spawn_repl();
    p.send_line(":help").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_q_alias() {
    let mut p = spawn_repl();
    p.send_line(":q").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_quit_alias() {
    let mut p = spawn_repl();
    p.send_line(":quit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_clear_then_undefined_error() {
    let mut p = spawn_repl();
    p.send_line("let a = 42;").unwrap();
    p.send_line(":clear").unwrap();
    p.send_line("a + 1").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_reset_then_undefined_error() {
    let mut p = spawn_repl();
    p.send_line("let b = 99;").unwrap();
    p.send_line(":reset").unwrap();
    p.send_line("b + 1").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_multi_line_function() {
    let mut p = spawn_repl();
    p.send_line("function add(a: i32, b: i32): i32 {").unwrap();
    p.send_line("    return a + b;").unwrap();
    p.send_line("}").unwrap();
    p.send_line("add(3, 4)").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_load_file() {
    let dir = std::env::temp_dir().join(format!("tyrus-repl-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("helper.tyr"),
        "let greeting: string = \"hello from load\";\n",
    )
    .unwrap();

    let mut p = spawn_repl();
    p.send_line(&format!(":load {}", dir.join("helper.tyr").display()))
        .unwrap();
    p.send_line("greeting").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn repl_load_file_not_found() {
    let mut p = spawn_repl();
    p.send_line(":load /nonexistent/path.tyr").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_function_declaration_no_wrap() {
    let mut p = spawn_repl();
    p.send_line("function sum(x: i32, y: i32): i32 { return x + y; }")
        .unwrap();
    p.send_line("sum(10, 20)").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_multiple_statements() {
    let mut p = spawn_repl();
    p.send_line("let a = 1; let b = 2; let c = a + b;").unwrap();
    p.send_line("c").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_bench_basic() {
    let start = Instant::now();
    let mut p = spawn_repl();
    for i in 0..10 {
        let _ = p.send_line(&format!("let x{i} = {i};"));
    }
    p.send_line("x0 + x9").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
    let elapsed = start.elapsed();
    eprintln!("repl_bench_basic: 10 statements + expr in {elapsed:?}");
}

#[test]
fn repl_type_integer() {
    let mut p = spawn_repl();
    p.send_line(":type 42").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_type_string() {
    let mut p = spawn_repl();
    p.send_line(r#":type "hello""#).unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_type_variable() {
    let mut p = spawn_repl();
    p.send_line("let msg: string = \"hi\";").unwrap();
    p.send_line(":type msg").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_type_missing_expr_shows_error() {
    let mut p = spawn_repl();
    p.send_line(":type").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_doc_variable() {
    let mut p = spawn_repl();
    p.send_line("/** A counter value */ let count: i32 = 0;")
        .unwrap();
    p.send_line(":doc count").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_doc_missing_symbol_shows_error() {
    let mut p = spawn_repl();
    p.send_line(":doc nonexistent").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_doc_missing_name_shows_error() {
    let mut p = spawn_repl();
    p.send_line(":doc").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_save_and_restore() {
    let mut p = spawn_repl();
    p.send_line("let a = 100;").unwrap();
    p.send_line(":save it-save.tyr").unwrap();
    p.send_line(":reset").unwrap();
    p.send_line(":restore it-save").unwrap();
    p.exp_string("Restored 1 lines").unwrap();
    // Wait until the replay fully finishes (the echo line followed by the idle
    // prompt) before sending the next input: a send that races with the restore
    // can have its Enter swallowed by the prompt's DSR handshake. `:type` then
    // prints the inferred type, which also synchronizes with the REPL.
    p.exp_after_last("Restored 1 lines", ";\ntyrus> ").unwrap();
    p.send_line(":type a").unwrap();
    p.exp_string("i32").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_restore_echoes_the_code() {
    let mut p = spawn_repl();
    p.send_line(r#"let name = "TitenQ";"#).unwrap();
    p.send_line(":save it-code.tyr").unwrap();
    p.send_line(":reset").unwrap();
    p.send_line(":restore it-code").unwrap();
    p.exp_string("tyrus> let name = \"TitenQ\";").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_restore_interleaves_output_after_invocation() {
    let mut p = spawn_repl();
    p.send_line(r#"let name = "Thomas";"#).unwrap();
    p.send_line("console.log(name);").unwrap();
    p.send_line(r#"console.log(`${name}2`);"#).unwrap();
    p.send_line(":save it-io.tyr").unwrap();
    p.send_line(":reset").unwrap();
    p.send_line(":restore it-io").unwrap();
    // Each invocation is echoed, then its output appears right below it: echo
    // of console.log(name), then "Thomas" before the second echo is shown, then
    // "Thomas2" after it. "Thomas\n" uniquely matches the output line (the echo
    // of the declaration line has `"Thomas";` after it).
    p.exp_string("tyrus> console.log(name);").unwrap();
    p.exp_before("Thomas\n", "tyrus> console.log(`${name}2`);")
        .unwrap();
    p.exp_string("Thomas2").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_list_shows_saved_sessions() {
    let mut p = spawn_repl();
    p.send_line("let a = 100;").unwrap();
    p.send_line(":save it-list.tyr").unwrap();
    p.send_line(":list").unwrap();
    p.exp_string("Saved sessions:").unwrap();
    p.exp_string("it-list.tyr").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_delete_removes_saved_session() {
    let mut p = spawn_repl();
    p.send_line("let a = 100;").unwrap();
    p.send_line(":save it-del.tyr").unwrap();
    p.send_line(":delete it-del").unwrap();
    p.exp_string("File it-del.tyr deleted successfully.")
        .unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_delete_missing_shows_error() {
    let mut p = spawn_repl();
    p.send_line(":delete nonexistent.tyr").unwrap();
    p.exp_string("error:").unwrap();
    p.exp_string("not found").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_save_requires_tyr_extension() {
    let mut p = spawn_repl();
    p.send_line(":save noext").unwrap();
    p.exp_string("error:").unwrap();
    p.exp_string(".tyr").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_save_missing_name_shows_error() {
    let mut p = spawn_repl();
    p.send_line(":save").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_restore_nonexistent_shows_error() {
    let mut p = spawn_repl();
    p.send_line(":restore /nonexistent/session.tyr").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_edit_on_empty_buffer_shows_error() {
    let mut p = spawn_repl();
    p.send_line(":edit").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_auto_close_pair_and_skip() {
    let mut p = spawn_repl();
    // Typing `(` auto-inserts the matching `)`; the hand-typed `)` then skips
    // over the pending one instead of doubling it, so the line evaluates
    // cleanly. A doubling would leave `console.log(40 + 2))` and fail to parse.
    p.send_line("console.log(40 + 2)").unwrap();
    p.exp_string("42").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_enter_removes_pending_closer() {
    let mut p = spawn_repl();
    // Typing `(` shows the auto-inserted `)` in the echoed buffer; Enter must
    // drop it so the unclosed call stays incomplete and the REPL asks for a
    // continuation line instead of submitting `console.log(40 + 2)` with a
    // stray closer (which would fail to parse as an incomplete `()`).
    p.send("console.log(40 + 2").unwrap();
    p.exp_string("tyrus> console.log(40 + 2)").unwrap();
    p.send("\r").unwrap();
    p.exp_string(".... ").unwrap();
    p.send_line(")").unwrap();
    p.exp_string("42").unwrap();
    p.exp_after_last("42", "tyrus> ").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_backspace_undoes_empty_pair() {
    let mut p = spawn_repl();
    // `(` auto-closes to `(1)`; backspace first removes the content `1`, then
    // the second backspace undoes the empty pair instead of leaving a stray `)`.
    p.send("(1").unwrap();
    p.exp_string("tyrus> (1)").unwrap();
    p.send("\x7f").unwrap();
    p.exp_string("tyrus> ()").unwrap();
    p.send("\x7f").unwrap();
    p.send("console.log(40 + 2)").unwrap();
    p.send("\r").unwrap();
    p.exp_string("42").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_no_auto_close_inside_string() {
    let mut p = spawn_repl();
    // `(` inside a string literal is content, not an opener, so it must not be
    // auto-closed; only the call parens are paired and skipped.
    p.send(r#"console.log("a(b"#).unwrap();
    p.exp_string(r#"tyrus> console.log("a(b"#).unwrap();
    p.send("x").unwrap();
    p.send(r#"""#).unwrap();
    p.send(")").unwrap();
    p.send("\r").unwrap();
    p.exp_string("a(bx").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_paste_inserts_as_literal_chunk() {
    let mut p = spawn_repl();
    // Bracketed paste arrives as a single literal chunk, bypassing auto-close
    // entirely; the pasted body must evaluate as-is.
    p.paste_line(r#"console.log("a(b")"#).unwrap();
    p.exp_string("a(b").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_auto_close_double_quote_nested_in_call() {
    let mut p = spawn_repl();
    // `"` typed inside an auto-closed call opens a nested string pair; it must
    // not be mistaken for a closer (which would drop the pending `)`).
    p.send(r#"console.log(""#).unwrap();
    p.exp_string(r#"tyrus> console.log("")"#).unwrap();
    p.send("hello").unwrap();
    p.exp_string(r#"tyrus> console.log("hello")"#).unwrap();
    p.send(r#"""#).unwrap();
    p.send(")").unwrap();
    p.send("\r").unwrap();
    p.exp_string("hello").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}

#[test]
fn repl_auto_close_single_quote_and_backtick() {
    let mut p = spawn_repl();
    // `'` and `` ` `` auto-close and skip like the double quote: `'abc'` and
    // `` `def` `` evaluate cleanly instead of leaving a dangling closer.
    p.send_line("console.log('abc')").unwrap();
    p.send("\r").unwrap();
    p.exp_string("abc").unwrap();
    p.send_line("console.log(`def`)").unwrap();
    p.send("\r").unwrap();
    p.exp_string("def").unwrap();
    p.send_line(":exit").unwrap();
    p.exp_eof().unwrap();
}
