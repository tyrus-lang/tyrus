//! What cancelling a task means, end to end.
//!
//! Three properties make the difference between a cancellation that can be
//! relied on and one that merely settles a promise, and each of them was
//! missing before this contract existed:
//!
//! **Cleanup runs.** A cancelled task used to be abandoned at its suspension
//! point, so the `finally` holding its file, its lock or its connection never
//! ran. Nothing that acquires a resource across an `await` could be cancelled
//! safely, which is why streaming, WebSocket and spilling a multipart part to
//! disk all waited on this.
//!
//! **Cleanup is not a `catch`.** A `catch` is written for the failures a
//! program expects from its own work. Letting "you were told to stop" arrive as
//! one of them means a handler that logs and carries on, or retries, starts
//! fresh work on a task that is already ending.
//!
//! **The wait is for the cleanup, not for the request.** Awaiting a cancelled
//! task returns once it has finished unwinding, so a caller that stops a worker
//! and then reuses its port, its file or its connection is not racing the
//! release.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Tells this binary's scratch projects apart. A name built only from a
/// timestamp collided whenever two tests reached it inside the same tick, and
/// since `Drop` removes the whole directory, the loser's binary was gone by the
/// time it tried to run: `NotFound` from a build that had just succeeded. The
/// process id and this counter make the name unique by construction, and the
/// timestamp stays to keep a directory left behind by a crashed run, under a
/// process id the system later reused, from being adopted as if it were ours.
static PROJECT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

struct Project {
    root: PathBuf,
    name: String,
}

impl Project {
    fn new(source: &str) -> Self {
        let suffix = format!(
            "{}-{}-{}",
            std::process::id(),
            PROJECT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let name = format!("cancellation-{suffix}");
        let root = std::env::temp_dir().join(&name);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("tyrus.json"),
            format!(
                r#"{{
  "project": {{
    "name": "{name}",
    "version": "0.1.0",
    "authors": [],
    "description": "Cancellation contract test"
  }},
  "dependencies": {{}}
}}
"#
            ),
        )
        .unwrap();
        fs::write(root.join("src/main.tyr"), source).unwrap();
        Self { root, name }
    }

    fn run(&self) -> (String, Duration) {
        let build = Command::new(env!("CARGO_BIN_EXE_tyrus"))
            .arg("build")
            .current_dir(&self.root)
            .output()
            .unwrap();
        assert!(
            build.status.success(),
            "build failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&build.stdout),
            String::from_utf8_lossy(&build.stderr)
        );
        let started = Instant::now();
        let output: Output = Command::new(self.root.join("target/debug").join(&self.name))
            .output()
            .unwrap();
        let elapsed = started.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(
            output.status.success(),
            "program failed:\nstdout:\n{stdout}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        (stdout, elapsed)
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn a_cancelled_task_runs_its_cleanup_and_not_its_catch() {
    let (output, _) = Project::new(
        r#"
import { cancel, sleepAsync } from "tyrus:task";

const worker = async (): Promise<void> => {
    try {
        console.log("acquired");
        await sleepAsync(4000);
        console.log("finished normally");
    } catch (error: Error) {
        console.log("catch ran");
    } finally {
        console.log("released");
    }
    console.log("after the try");
};

const main = async (): Promise<void> => {
    const work: Promise<void> = worker();
    await sleepAsync(50);
    cancel(work, "stop");
    try {
        await work;
    } catch (error: Error) {
        console.log(`observed: ${error.message}`);
    }
};
"#,
    )
    .run();

    // `released` without `catch ran` is the whole contract in two lines. And
    // nothing after the `try`: a cancelled task unwinds and leaves, so the
    // statements following the block are work it was already told not to do.
    assert_eq!(output, "acquired\nreleased\nobserved: stop\n");
}

/// The caller learns the work stopped only once it has finished stopping.
///
/// Settling at the moment of the request would wake the caller while the task
/// is still inside its `finally`, having released nothing yet, and reusing the
/// resource right after would be a race that shows up as a rare failure rather
/// than a wrong answer.
#[test]
fn awaiting_a_cancelled_task_waits_for_its_cleanup() {
    let (output, elapsed) = Project::new(
        r#"
import { cancel, sleepAsync } from "tyrus:task";

const worker = async (): Promise<void> => {
    try {
        await sleepAsync(4000);
    } catch (error: Error) {
        console.log("catch ran");
    } finally {
        console.log("cleanup: started");
        await sleepAsync(400);
        console.log("cleanup: finished");
    }
};

const main = async (): Promise<void> => {
    const work: Promise<void> = worker();
    await sleepAsync(50);
    cancel(work, "stop");
    try {
        await work;
    } catch (error: Error) {
        console.log(`observed: ${error.message}`);
    }
};
"#,
    )
    .run();

    assert_eq!(
        output, "cleanup: started\ncleanup: finished\nobserved: stop\n",
        "cleanup must be allowed to await, and the caller must wait for it"
    );
    assert!(
        elapsed >= Duration::from_millis(400),
        "the caller returned before the cleanup finished: {elapsed:?}"
    );
}

/// Cleanup that will not finish is cut, and the caller is told that is why.
///
/// Not the default: cleanup still making progress is not something to interrupt
/// on a guess, so a caller with somewhere to be says how long it will wait.
#[test]
fn a_deadline_cuts_cleanup_that_overruns_it() {
    let (output, elapsed) = Project::new(
        r#"
import { cancel, sleepAsync } from "tyrus:task";

const worker = async (): Promise<void> => {
    try {
        await sleepAsync(4000);
    } catch (error: Error) {
        console.log("catch ran");
    } finally {
        console.log("cleanup: started");
        await sleepAsync(4000);
        console.log("cleanup: finished");
    }
};

const main = async (): Promise<void> => {
    const work: Promise<void> = worker();
    await sleepAsync(50);
    cancel(work, "stop", 300);
    try {
        await work;
    } catch (error: Error) {
        console.log(`observed: ${error.message}`);
    }
};
"#,
    )
    .run();

    assert_eq!(
        output, "cleanup: started\nobserved: cancellation cleanup exceeded its deadline\n",
        "the cut has to be visible: a caller cannot otherwise tell a clean stop \
         from one that abandoned work half-released"
    );
    assert!(
        elapsed < Duration::from_millis(2000),
        "the deadline did not cut the cleanup: {elapsed:?}"
    );
}

/// Being cancelled and awaiting something cancelled are different things.
///
/// Only the first skips the `catch`. Conflating them would make
/// `cancel(work, "stop"); await work;` impossible to write, which is the very
/// pattern the contract exists to support.
#[test]
fn a_task_can_still_catch_a_cancellation_it_did_not_receive() {
    let (output, _) = Project::new(
        r#"
import { cancel, sleepAsync } from "tyrus:task";

const worker = async (): Promise<void> => {
    await sleepAsync(4000);
};

const observer = async (work: Promise<void>): Promise<void> => {
    try {
        await work;
        console.log("not reached");
    } catch (error: Error) {
        console.log(`observer caught: ${error.message}`);
    }
};

const main = async (): Promise<void> => {
    const work: Promise<void> = worker();
    await sleepAsync(50);
    cancel(work, "stop");
    await observer(work);
    console.log("done");
};
"#,
    )
    .run();

    assert_eq!(output, "observer caught: stop\ndone\n");
}

/// A loop with no suspension in it can stop itself.
///
/// Cancellation is cooperative: it arrives at an `await`, so a task in the
/// middle of a long computation never reaches one. That is a consequence of the
/// design rather than a hole in it, and `isCancelled` is what the loop uses
/// instead.
#[test]
fn a_computation_can_ask_whether_it_was_cancelled() {
    let (output, _) = Project::new(
        r#"
import { cancel, isCancelled, sleepAsync } from "tyrus:task";

const compute = async (): Promise<i64> => {
    let total: i64 = 0;
    for (let round: i32 = 0; round < 500; round += 1) {
        for (let i: i64 = 0; i < 200000; i += 1) {
            total += i;
        }
        if (isCancelled()) {
            console.log("stopped itself");
            return total;
        }
        await sleepAsync(1);
    }
    console.log("ran every round");
    return total;
};

const main = async (): Promise<void> => {
    const work: Promise<i64> = compute();
    // Asked before the task ever suspends, so the first check sees it.
    cancel(work, "stop");
    try {
        const value: i64 = await work;
        console.log(`returned ${value}`);
    } catch (error: Error) {
        console.log(`observed: ${error.message}`);
    }
};
"#,
    )
    .run();

    // The task returned a value of its own, and the promise is cancelled all
    // the same: a task that was told to stop ends stopped whatever it managed
    // to produce on the way out.
    assert_eq!(output, "stopped itself\nobserved: stop\n");
}

/// A request that arrives before the task has begun still interrupts its first
/// wait.
///
/// The request lands once, but a task goes on reaching new suspension points. A
/// task cancelled before it ran was suspended on nothing at all, and used to
/// wait out the full duration of whatever it suspended on next.
#[test]
fn a_task_cancelled_before_it_starts_does_not_wait_out_its_first_suspension() {
    let (output, elapsed) = Project::new(
        r#"
import { spawn, sleepAsync } from "tyrus:task";

const main = async (): Promise<void> => {
    const handle = spawn(async (): Promise<i32> => {
        await sleepAsync(4000);
        return 42;
    });
    handle.abort();
    try {
        const value = await handle.join();
        console.log(value);
    } catch (error: Error) {
        console.log(error.message);
    }
};
"#,
    )
    .run();

    assert_eq!(output, "Aborted by caller\n");
    assert!(
        elapsed < Duration::from_millis(1500),
        "the task waited out a suspension it entered after being cancelled: {elapsed:?}"
    );
}

/// Cancelling reaches work the runtime is doing on the task's behalf.
///
/// This is what "cancellation propagates" buys, and the reason it is measured
/// from the server rather than the client: nothing on the client side
/// distinguishes a transfer that stopped from one that is still running into a
/// buffer nobody will read. Before the contract, cancelling the task settled
/// its promise and the download carried on holding the connection until the
/// process ended.
#[test]
fn cancelling_a_task_stops_the_transfer_it_was_waiting_on() {
    use std::io::{Read, Write};

    let listener = match std::net::TcpListener::bind(("127.0.0.1", 0)) {
        Ok(listener) => listener,
        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!(
                "SKIP: loopback cancellation testing requires permission to bind a local TCP listener ({error})."
            );
            return;
        }
        Err(error) => panic!("could not bind the loopback cancellation test listener: {error}"),
    };
    let port = listener.local_addr().unwrap().port();
    let (report, delivered) = std::sync::mpsc::channel();
    let (accepted, connected) = std::sync::mpsc::channel();
    let give_up = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let server_give_up = std::sync::Arc::clone(&give_up);
    let server = std::thread::spawn(move || {
        let Some(Ok(mut stream)) = listener.incoming().next() else {
            return;
        };
        // Woken by the test itself when the program never connected: the
        // cancellation arrived before the request left, so there is no
        // transfer to measure.
        if server_give_up.load(std::sync::atomic::Ordering::Acquire) {
            let _ = report.send(0);
            return;
        }
        let _ = accepted.send(());
        let mut request = [0_u8; 1024];
        let _ = stream.read(&mut request);
        let _ = stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 200000\r\nConnection: close\r\n\r\n");
        // Dribbled out slowly, so the client is still reading when it is
        // cancelled. A client that keeps reading takes the whole 20 rounds.
        let mut sent = 0_usize;
        for _ in 0..20 {
            if stream.write_all(&[b'x'; 10000]).is_err() {
                break;
            }
            sent += 10000;
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = report.send(sent);
    });

    let (output, _) = Project::new(&format!(
        r#"
import {{ fetch, HttpClientResponse, HttpError }} from "tyrus:http";
import {{ cancel, sleepAsync }} from "tyrus:task";

const download = async (): Promise<void> => {{
    const response: HttpClientResponse = await fetch.get("http://127.0.0.1:{port}/slow");
    console.log(`read ${{response.text().length}}`);
    response.close();
}};

const main = async (): Promise<void> => {{
    const work: Promise<void> = download();
    await sleepAsync(300);
    cancel(work, "stop");
    try {{
        await work;
    }} catch (error: Error) {{
        console.log(`observed: ${{error.message}}`);
    }}
    // Long enough that a transfer still running would send the rest.
    await sleepAsync(1500);
    console.log("done");
}};
"#
    ))
    .run();

    // The program waits 300 ms before cancelling. On a machine saturated by the
    // whole test suite that can pass before the request has even connected,
    // and then the cancel correctly means no transfer ever starts. The server
    // would still be waiting to accept, so it is released here and the transfer
    // counts as zero bytes, which is exactly what the contract requires.
    let sent = match connected.try_recv() {
        Ok(()) => delivered
            .recv_timeout(Duration::from_secs(30))
            .expect("the server never finished"),
        Err(_) => {
            give_up.store(true, std::sync::atomic::Ordering::Release);
            let _ = std::net::TcpStream::connect(("127.0.0.1", port));
            delivered
                .recv_timeout(Duration::from_secs(30))
                .expect("the server never finished")
        }
    };
    let _ = server.join();

    assert_eq!(output, "observed: stop\ndone\n");
    assert!(
        sent < 200_000,
        "the transfer ran to completion after the task was cancelled: {sent} bytes"
    );
}
