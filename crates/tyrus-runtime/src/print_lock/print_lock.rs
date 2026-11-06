//! Single-lock-per-line helpers for `print`/`console.log` and friends.
//!
//! `printf`/`dprintf` already lock internally per call, but a multi-argument
//! line is several calls, so two threads logging at once can still interleave
//! mid-line. The fix is to hold one lock for the whole line instead of one
//! per call -- but only ever around the *writes*: the codegen for
//! `print`/`console.log` (`Codegen::emit_print_line` in `tyrus-codegen`)
//! evaluates every argument's value *before* calling
//! `tyrus_stdout_lock_begin`/`tyrus_stderr_lock_begin`, and only the
//! already-computed values are printed while the lock is held. This matters
//! because tyrus's `throw` unwinds via `longjmp` (see
//! `crates/tyrus-codegen/src/statements/statements.rs`), which can jump straight out of
//! the current function -- if a `throw` happened *while the lock was held*,
//! the matching `tyrus_stdout_lock_end` would never run, deadlocking every
//! other thread's next print forever. Because the lock is only ever acquired
//! after argument evaluation is done, a `throw` triggered by evaluating an
//! argument (e.g. `print("a", risky(), "b")`) always happens before the lock
//! is taken, so there is never a lock left behind to leak.

use std::cell::RefCell;
use std::sync::{Mutex, MutexGuard};
use tyrus_string::TyrusString;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());
static STDERR_LOCK: Mutex<()> = Mutex::new(());

thread_local! {
    static STDOUT_GUARD: RefCell<Option<MutexGuard<'static, ()>>> = const { RefCell::new(None) };
    static STDERR_GUARD: RefCell<Option<MutexGuard<'static, ()>>> = const { RefCell::new(None) };
}

fn lock_begin(
    lock: &'static Mutex<()>,
    slot: &'static std::thread::LocalKey<RefCell<Option<MutexGuard<'static, ()>>>>,
) {
    let guard = lock
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    slot.with(|cell| *cell.borrow_mut() = Some(guard));
}

fn lock_end(slot: &'static std::thread::LocalKey<RefCell<Option<MutexGuard<'static, ()>>>>) {
    slot.with(|cell| {
        cell.borrow_mut().take();
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_stdout_lock_begin() {
    lock_begin(&STDOUT_LOCK, &STDOUT_GUARD);
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_stdout_lock_end() {
    lock_end(&STDOUT_GUARD);
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_stderr_lock_begin() {
    lock_begin(&STDERR_LOCK, &STDERR_GUARD);
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_stderr_lock_end() {
    lock_end(&STDERR_GUARD);
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_stdout_is_terminal() -> bool {
    static IS_TERMINAL: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *IS_TERMINAL.get_or_init(|| std::io::IsTerminal::is_terminal(&std::io::stdout()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_print_str(value: TyrusString, fd: i32) {
    unsafe {
        let bytes = value.byte_len();
        if value.is_null() || bytes == 0 {
            return;
        }
        tyrus_stdio_write(value.as_raw_ptr().cast(), bytes, fd);
    }
}

pub(crate) fn write_stdout(text: &str) {
    if text.is_empty() {
        return;
    }
    unsafe {
        tyrus_stdio_write(text.as_ptr().cast(), text.len(), 1);
    }
}

pub(crate) fn flush_stdout() {
    unsafe {
        tyrus_stdio_flush(1);
    }
}

unsafe extern "C" {
    fn tyrus_stdio_write(buffer: *const libc::c_void, bytes: usize, descriptor: i32) -> usize;
    fn tyrus_stdio_flush(descriptor: i32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn stdout_lock_serializes_concurrent_writers() {
        let buffer = Arc::new(Mutex::new(String::new()));
        let started = Arc::new(AtomicUsize::new(0));
        let threads: Vec<_> = (0..16)
            .map(|i| {
                let buffer = Arc::clone(&buffer);
                let started = Arc::clone(&started);
                std::thread::spawn(move || {
                    started.fetch_add(1, Ordering::SeqCst);
                    while started.load(Ordering::SeqCst) < 16 {
                        std::thread::yield_now();
                    }
                    tyrus_stdout_lock_begin();
                    let mut guard = buffer.lock().unwrap();
                    guard.push_str(&format!("[{i}-a]"));
                    std::thread::yield_now();
                    guard.push_str(&format!("[{i}-b]"));
                    drop(guard);
                    tyrus_stdout_lock_end();
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        let result = buffer.lock().unwrap();
        for i in 0..16 {
            let a = result.find(&format!("[{i}-a]")).unwrap();
            let b = result.find(&format!("[{i}-b]")).unwrap();
            assert_eq!(
                b,
                a + format!("[{i}-a]").len(),
                "thread {i}'s two writes must be adjacent, not interleaved with another thread: {result}"
            );
        }
    }

    #[test]
    fn lock_end_without_begin_does_not_panic() {
        tyrus_stdout_lock_end();
        tyrus_stderr_lock_end();
    }
}
