#![cfg(windows)]

use std::ffi::OsStr;
use std::iter::once;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use std::process::Command;
use std::ptr::{null, null_mut};
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Storage::FileSystem::{ReadFile, WriteFile};
use windows_sys::Win32::System::Console::{COORD, ClosePseudoConsole, CreatePseudoConsole, HPCON};
use windows_sys::Win32::System::Pipes::{CreatePipe, PeekNamedPipe};
use windows_sys::Win32::System::Threading::{
    CreateProcessW, DeleteProcThreadAttributeList, EXTENDED_STARTUPINFO_PRESENT,
    GetExitCodeProcess, InitializeProcThreadAttributeList, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE,
    PROCESS_INFORMATION, STARTF_USESTDHANDLES, STARTUPINFOEXW, TerminateProcess,
    UpdateProcThreadAttribute, WaitForSingleObject,
};

const TIMEOUT: Duration = Duration::from_secs(30);

struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CloseHandle(self.0) };
        }
    }
}

struct ChildProcess {
    handle: HANDLE,
    exited: bool,
}

impl Drop for ChildProcess {
    fn drop(&mut self) {
        if !self.exited {
            unsafe {
                TerminateProcess(self.handle, 1);
                WaitForSingleObject(self.handle, 5_000);
            }
        }
        unsafe { CloseHandle(self.handle) };
    }
}

struct PseudoConsole(HPCON);

impl Drop for PseudoConsole {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe { ClosePseudoConsole(self.0) };
        }
    }
}

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(once(0)).collect()
}

fn pipe() -> (Handle, Handle) {
    let mut read = null_mut();
    let mut write = null_mut();
    let ok = unsafe { CreatePipe(&mut read, &mut write, null::<SECURITY_ATTRIBUTES>(), 0) };
    assert_ne!(ok, 0, "CreatePipe failed with Win32 error {}", unsafe {
        GetLastError()
    });
    (Handle(read), Handle(write))
}

fn write_input(input: HANDLE, bytes: &[u8]) {
    let mut written = 0;
    let ok = unsafe {
        WriteFile(
            input,
            bytes.as_ptr(),
            bytes.len() as u32,
            &mut written,
            null_mut(),
        )
    };
    assert_ne!(ok, 0, "WriteFile failed with Win32 error {}", unsafe {
        GetLastError()
    });
    assert_eq!(written as usize, bytes.len());
}

fn pump_output(output: HANDLE, input: HANDLE, buffer: &mut Vec<u8>) -> bool {
    let mut available = 0;
    let ok = unsafe {
        PeekNamedPipe(
            output,
            null_mut(),
            0,
            null_mut(),
            &mut available,
            null_mut(),
        )
    };
    if ok == 0 {
        return false;
    }
    if available == 0 {
        return true;
    }
    let mut chunk = vec![0; available.min(4096) as usize];
    let mut read = 0;
    let ok = unsafe {
        ReadFile(
            output,
            chunk.as_mut_ptr(),
            chunk.len() as u32,
            &mut read,
            null_mut(),
        )
    };
    if ok == 0 {
        return false;
    }
    chunk.truncate(read as usize);
    buffer.extend_from_slice(&chunk);
    while let Some(position) = buffer.windows(4).position(|window| window == b"\x1b[6n") {
        buffer.drain(position..position + 4);
        write_input(input, b"\x1b[1;1R");
    }
    true
}

fn wait_for_output(
    process: HANDLE,
    output: HANDLE,
    input: HANDLE,
    buffer: &mut Vec<u8>,
    needle: &[u8],
) {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {:?}; received {:?}",
            String::from_utf8_lossy(needle),
            String::from_utf8_lossy(buffer)
        );
        assert!(
            pump_output(output, input, buffer),
            "REPL output pipe closed"
        );
        if buffer.windows(needle.len()).any(|window| window == needle) {
            return;
        }
        let state = unsafe { WaitForSingleObject(process, 0) };
        let mut exit_code = u32::MAX;
        let exit_code_result = unsafe { GetExitCodeProcess(process, &mut exit_code) };
        let exit_status = if exit_code_result == 0 {
            format!("unavailable (Win32 error {})", unsafe { GetLastError() })
        } else {
            format!("{exit_code:#010x}")
        };
        assert_ne!(
            state,
            WAIT_OBJECT_0,
            "REPL exited before producing {:?}: {:?}; exit status {exit_status}",
            String::from_utf8_lossy(needle),
            String::from_utf8_lossy(buffer)
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
#[ignore = "requires a Windows ConPTY session; run through the Windows ConPTY diagnostic workflow"]
fn windows_conpty_repl_evaluates_a_native_runtime_call_and_exits() {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_tyrus"));
    assert!(
        executable.is_file(),
        "CLI binary not found at {}",
        executable.display()
    );
    let preflight = Command::new(&executable)
        .arg("--version")
        .output()
        .expect("the CLI binary should start outside ConPTY");
    assert!(
        preflight.status.success(),
        "CLI failed before ConPTY startup: status {:?}, stdout {:?}, stderr {:?}",
        preflight.status,
        String::from_utf8_lossy(&preflight.stdout),
        String::from_utf8_lossy(&preflight.stderr)
    );

    let (input_read, input_write) = pipe();
    let (output_read, output_write) = pipe();
    let mut pseudo_console = 0;
    let result = unsafe {
        CreatePseudoConsole(
            COORD { X: 100, Y: 30 },
            input_read.0,
            output_write.0,
            0,
            &mut pseudo_console,
        )
    };
    assert_eq!(
        result, 0,
        "CreatePseudoConsole failed with HRESULT {result:#x}"
    );
    let pseudo_console = PseudoConsole(pseudo_console);

    let mut attribute_bytes = 0;
    unsafe {
        InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut attribute_bytes);
    }
    assert!(
        attribute_bytes > 0,
        "attribute-list sizing returned zero bytes"
    );
    let mut attribute_storage = vec![0usize; attribute_bytes.div_ceil(size_of::<usize>())];
    let attributes = attribute_storage.as_mut_ptr().cast();
    assert_ne!(
        unsafe { InitializeProcThreadAttributeList(attributes, 1, 0, &mut attribute_bytes) },
        0,
        "InitializeProcThreadAttributeList failed with Win32 error {}",
        unsafe { GetLastError() }
    );
    assert_ne!(
        unsafe {
            UpdateProcThreadAttribute(
                attributes,
                0,
                PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE as usize,
                pseudo_console.0 as *const std::ffi::c_void,
                size_of::<HPCON>(),
                null_mut(),
                null(),
            )
        },
        0,
        "UpdateProcThreadAttribute failed with Win32 error {}",
        unsafe { GetLastError() }
    );

    let executable_wide = wide(executable.as_os_str());
    let mut command_line = wide(OsStr::new(&format!("\"{}\" repl", executable.display())));
    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = size_of::<STARTUPINFOEXW>() as u32;
    startup.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
    startup.lpAttributeList = attributes;
    let mut process_info = PROCESS_INFORMATION::default();
    let created = unsafe {
        CreateProcessW(
            executable_wide.as_ptr(),
            command_line.as_mut_ptr(),
            null(),
            null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT,
            null(),
            null(),
            &startup.StartupInfo,
            &mut process_info,
        )
    };
    drop(input_read);
    drop(output_write);
    unsafe { DeleteProcThreadAttributeList(attributes) };
    assert_ne!(
        created,
        0,
        "CreateProcessW failed with Win32 error {}",
        unsafe { GetLastError() }
    );
    let mut process = ChildProcess {
        handle: process_info.hProcess,
        exited: false,
    };
    let _thread = Handle(process_info.hThread);

    let mut output = Vec::new();
    wait_for_output(
        process.handle,
        output_read.0,
        input_write.0,
        &mut output,
        b"tyrus> ",
    );
    write_input(input_write.0, b"console.log(1 + 2)\r");
    wait_for_output(
        process.handle,
        output_read.0,
        input_write.0,
        &mut output,
        b"3",
    );
    assert!(
        !String::from_utf8_lossy(&output).contains("warning: JIT execution failed"),
        "REPL fell back from JIT to native compilation: {:?}",
        String::from_utf8_lossy(&output)
    );
    write_input(input_write.0, b":exit\r");
    let deadline = Instant::now() + TIMEOUT;
    loop {
        assert!(Instant::now() < deadline, "REPL did not exit after :exit");
        let state = unsafe { WaitForSingleObject(process.handle, 10) };
        if state == WAIT_OBJECT_0 {
            process.exited = true;
            break;
        }
        pump_output(output_read.0, input_write.0, &mut output);
    }
    let mut exit_code = u32::MAX;
    assert_ne!(
        unsafe { GetExitCodeProcess(process.handle, &mut exit_code) },
        0,
        "GetExitCodeProcess failed with Win32 error {}",
        unsafe { GetLastError() }
    );
    assert_eq!(exit_code, 0, "REPL exited with status {exit_code}");
    drop(pseudo_console);
}
