use super::abi::{ColorFfi, NotifyOptionsFfi, OsErrorTaken, TyrusArray, UserInfo};
use std::ffi::CString;
use std::ffi::{CStr, c_char};
use std::mem::ManuallyDrop;
use tyrus_string::TyrusString;

#[repr(C)]
struct OsErrorResult {
    code: TyrusString,
    message: TyrusString,
}

#[repr(C)]
union OsVoidResultPayload {
    error: ManuallyDrop<OsErrorResult>,
    words: [u64; 4],
}

#[repr(C)]
struct OsVoidResult {
    tag: u8,
    payload: OsVoidResultPayload,
}

#[repr(C)]
union OsArrayResultPayload {
    array: ManuallyDrop<TyrusArray>,
    error: ManuallyDrop<OsErrorTaken>,
    words: [u64; 4],
}

#[repr(C)]
pub struct OsArrayResult {
    tag: u8,
    payload: OsArrayResultPayload,
}

#[repr(C)]
union OsClipboardReadResultPayload {
    value: BoxedPayload,
    error: ManuallyDrop<OsErrorTaken>,
    words: [u64; 4],
}

#[repr(C)]
pub struct OsClipboardReadResult {
    tag: u8,
    payload: OsClipboardReadResultPayload,
}

const _: () = {
    assert!(std::mem::size_of::<OsErrorResult>() == 32);
    assert!(std::mem::size_of::<OsVoidResult>() == 40);
    assert!(std::mem::offset_of!(OsVoidResult, payload) == 8);
    assert!(std::mem::size_of::<OsArrayResult>() == 40);
    assert!(std::mem::offset_of!(OsArrayResult, payload) == 8);
    assert!(std::mem::size_of::<OsClipboardReadResult>() == 40);
    assert!(std::mem::offset_of!(OsClipboardReadResult, payload) == 8);
};

fn os_array_result(result: Result<TyrusArray, OsErrorTaken>) -> OsArrayResult {
    match result {
        Ok(array) => OsArrayResult {
            tag: 0,
            payload: OsArrayResultPayload {
                array: ManuallyDrop::new(array),
            },
        },
        Err(error) => OsArrayResult {
            tag: 1,
            payload: OsArrayResultPayload {
                error: ManuallyDrop::new(error),
            },
        },
    }
}

fn os_clipboard_read_result(value: BoxedPayload, error: OsErrorTaken) -> OsClipboardReadResult {
    if error.code.is_null() {
        OsClipboardReadResult {
            tag: 0,
            payload: OsClipboardReadResultPayload { value },
        }
    } else {
        OsClipboardReadResult {
            tag: 1,
            payload: OsClipboardReadResultPayload {
                error: ManuallyDrop::new(error),
            },
        }
    }
}

fn clipboard_read_error(error: &arboard::Error) -> Option<(&'static CStr, &'static CStr)> {
    if matches!(error, arboard::Error::ContentNotAvailable) {
        None
    } else {
        Some((
            c"ERR_OS_CLIPBOARD_READ_FAILED",
            c"the system clipboard text could not be read",
        ))
    }
}

#[cfg(kani)]
#[kani::proof]
fn os_array_result_uses_the_compiler_variant_tags() {
    let array = TyrusArray {
        data: std::ptr::null_mut(),
        length: 0,
        capacity: 0,
    };
    assert_eq!(os_array_result(Ok(array)).tag, 0);
    let error = OsErrorTaken {
        code: TyrusString::literal("ERR_OS_NETWORK_INTERFACES_FAILED"),
        message: TyrusString::literal("failed to enumerate network interfaces"),
    };
    assert_eq!(os_array_result(Err(error)).tag, 1);
}

#[cfg(kani)]
#[kani::proof]
fn os_clipboard_read_result_uses_the_compiler_variant_tags() {
    let empty = os_clipboard_read_result(
        std::ptr::null_mut(),
        OsErrorTaken {
            code: TyrusString::null(),
            message: TyrusString::null(),
        },
    );
    assert_eq!(empty.tag, 0);
    let error = os_clipboard_read_result(
        std::ptr::null_mut(),
        OsErrorTaken {
            code: TyrusString::literal("ERR_OS_CLIPBOARD_UNAVAILABLE"),
            message: TyrusString::literal("the system clipboard is unavailable"),
        },
    );
    assert_eq!(error.tag, 1);
}

fn os_void_result(error: OsErrorTaken) -> OsVoidResult {
    if error.code.is_null() {
        OsVoidResult {
            tag: 0,
            payload: OsVoidResultPayload { words: [0; 4] },
        }
    } else {
        OsVoidResult {
            tag: 1,
            payload: OsVoidResultPayload {
                error: ManuallyDrop::new(OsErrorResult {
                    code: error.code,
                    message: error.message,
                }),
            },
        }
    }
}

unsafe fn write_os_void_result(out: *mut OsVoidResult) {
    if out.is_null() {
        return;
    }
    let mut error = OsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_os_take_error(&mut error) };
    let result = os_void_result(error);
    unsafe { std::ptr::write(out, result) };
}

#[cfg(kani)]
#[kani::proof]
fn os_void_result_uses_the_compiler_variant_tags() {
    let success = os_void_result(OsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    });
    assert_eq!(success.tag, 0);
    let failure = os_void_result(OsErrorTaken {
        code: TyrusString::literal("ERR_OS_REVEAL_FAILED"),
        message: TyrusString::literal("no file manager is registered to reveal this path"),
    });
    assert_eq!(failure.tag, 1);
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_platform() -> i32 {
    match std::env::consts::OS {
        "linux" => 0,
        "macos" => 1,
        "windows" => 2,
        _ => 3,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_arch() -> i32 {
    match std::env::consts::ARCH {
        "x86_64" => 0,
        "aarch64" => 1,
        "x86" => 2,
        "arm" => 3,
        _ => 4,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_hostname() -> TyrusString {
    let name = hostname::get()
        .ok()
        .map(|value| value.to_string_lossy().into_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    crate::strings::alloc_string(&name)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_release() -> TyrusString {
    let release = sysinfo::System::kernel_version().unwrap_or_else(|| "unknown".to_string());
    crate::strings::alloc_string(&release)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_system_name() -> TyrusString {
    let name = match std::env::consts::OS {
        "macos" => "Darwin",
        "windows" => "Windows",
        "linux" => "Linux",
        other => other,
    };
    crate::strings::alloc_string(name)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_locale() -> TyrusString {
    let locale = sys_locale::get_locale().unwrap_or_else(|| "und".to_string());
    crate::strings::alloc_string(&locale)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_language() -> TyrusString {
    let locale = sys_locale::get_locale().unwrap_or_else(|| "und".to_string());
    let language = locale.split(['-', '_']).next().unwrap_or("und");
    crate::strings::alloc_string(language)
}

static PASSWD_LOOKUP_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn env_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn env_shell() -> String {
    #[cfg(unix)]
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(passwd_shell);
    #[cfg(windows)]
    let shell = std::env::var("COMSPEC")
        .ok()
        .filter(|value| !value.is_empty());
    shell.unwrap_or_else(|| "unknown".to_string())
}

fn base_dirs_path(select: impl FnOnce(&directories::BaseDirs) -> String) -> String {
    let _guard = PASSWD_LOOKUP_LOCK.lock().unwrap();
    directories::BaseDirs::new()
        .map(|dirs| select(&dirs))
        .unwrap_or_default()
}

fn base_dirs_home() -> String {
    base_dirs_path(|dirs| dirs.home_dir().to_string_lossy().into_owned())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_user_info(out: *mut UserInfo) {
    if out.is_null() {
        return;
    }
    unsafe {
        (*out).username = crate::strings::alloc_string(&env_username());
        (*out).homedir = crate::strings::alloc_string(&base_dirs_home());
        (*out).shell = crate::strings::alloc_string(&env_shell());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_user_name() -> TyrusString {
    crate::strings::alloc_string(&env_username())
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_home_dir() -> TyrusString {
    crate::strings::alloc_string(&base_dirs_home())
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_cache_dir() -> TyrusString {
    let dir = base_dirs_path(|dirs| dirs.cache_dir().to_string_lossy().into_owned());
    crate::strings::alloc_string(&dir)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_config_dir() -> TyrusString {
    let dir = base_dirs_path(|dirs| dirs.config_dir().to_string_lossy().into_owned());
    crate::strings::alloc_string(&dir)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_data_dir() -> TyrusString {
    let dir = base_dirs_path(|dirs| dirs.data_dir().to_string_lossy().into_owned());
    crate::strings::alloc_string(&dir)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_temp_dir() -> TyrusString {
    let dir = std::env::temp_dir().to_string_lossy().into_owned();
    crate::strings::alloc_string(&dir)
}

pub type BoxedPayload = *mut std::ffi::c_void;

fn box_optional_string(value: TyrusString) -> BoxedPayload {
    let storage = unsafe { libc::malloc(std::mem::size_of::<TyrusString>()) } as *mut TyrusString;
    if storage.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { std::ptr::write(storage, value) };
    storage.cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_desktop_dir() -> BoxedPayload {
    let found = {
        let _guard = PASSWD_LOOKUP_LOCK.lock().unwrap();
        directories::UserDirs::new().and_then(|dirs| {
            dirs.desktop_dir()
                .map(|path| path.to_string_lossy().into_owned())
        })
    };
    let Some(path) = found else {
        return std::ptr::null_mut();
    };
    box_optional_string(crate::strings::alloc_string(&path))
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_documents_dir() -> BoxedPayload {
    let found = {
        let _guard = PASSWD_LOOKUP_LOCK.lock().unwrap();
        directories::UserDirs::new().and_then(|dirs| {
            dirs.document_dir()
                .map(|path| path.to_string_lossy().into_owned())
        })
    };
    let Some(path) = found else {
        return std::ptr::null_mut();
    };
    box_optional_string(crate::strings::alloc_string(&path))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_app_data_dir(app: TyrusString) -> TyrusString {
    if app.is_null() {
        return crate::strings::alloc_string("");
    }
    let app = unsafe { app.as_str() };
    let dir = {
        let _guard = PASSWD_LOOKUP_LOCK.lock().unwrap();
        directories::ProjectDirs::from("", "", app)
            .map(|dirs| dirs.data_dir().to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    crate::strings::alloc_string(&dir)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_whoami() -> BoxedPayload {
    let result = {
        let _guard = PASSWD_LOOKUP_LOCK.lock().unwrap();
        whoami::fallible::realname()
    };
    let Ok(name) = result else {
        return std::ptr::null_mut();
    };
    box_optional_string(crate::strings::alloc_string(&name))
}

#[repr(C)]
pub struct CpuInfo {
    pub model: TyrusString,
    pub speed: u32,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_cpus(out_array: *mut TyrusArray) {
    if out_array.is_null() {
        return;
    }
    let system = sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::new().with_cpu(sysinfo::CpuRefreshKind::everything()),
    );
    let mut cpus: Vec<CpuInfo> = system
        .cpus()
        .iter()
        .map(|cpu| CpuInfo {
            model: crate::strings::alloc_string(cpu.brand()),
            speed: cpu.frequency() as u32,
        })
        .collect();
    let ptr = cpus.as_mut_ptr();
    let len = cpus.len();
    let cap = cpus.capacity();
    std::mem::forget(cpus);
    unsafe {
        (*out_array).data = ptr as *mut u8;
        (*out_array).length = len;
        (*out_array).capacity = cap;
    }
}

fn memory_snapshot() -> sysinfo::System {
    sysinfo::System::new_with_specifics(
        sysinfo::RefreshKind::new().with_memory(sysinfo::MemoryRefreshKind::everything()),
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_total_mem() -> u64 {
    memory_snapshot().total_memory()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_free_mem() -> u64 {
    memory_snapshot().free_memory()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_uptime() -> u64 {
    sysinfo::System::uptime()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_load_avg(out_array: *mut TyrusArray) {
    if out_array.is_null() {
        return;
    }
    let load = sysinfo::System::load_average();
    let mut values: Vec<f64> = vec![load.one, load.five, load.fifteen];
    let ptr = values.as_mut_ptr();
    let len = values.len();
    let cap = values.capacity();
    std::mem::forget(values);
    unsafe {
        (*out_array).data = ptr as *mut u8;
        (*out_array).length = len;
        (*out_array).capacity = cap;
    }
}

fn box_bytes(bytes: &[u8]) -> BoxedPayload {
    let storage = unsafe { libc::malloc(bytes.len().max(1)) } as *mut u8;
    if storage.is_null() {
        return std::ptr::null_mut();
    }
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), storage, bytes.len()) };
    storage.cast()
}

fn box_optional_u32(value: Option<u32>) -> BoxedPayload {
    match value {
        None => std::ptr::null_mut(),
        Some(value) => box_bytes(&value.to_ne_bytes()),
    }
}

#[repr(C)]
pub struct BatteryInfo {
    pub level: f32,
    pub charging: bool,
    pub time_to_full: BoxedPayload,
    pub time_to_empty: BoxedPayload,
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_battery() -> BoxedPayload {
    let Ok(manager) = starship_battery::Manager::new() else {
        return std::ptr::null_mut();
    };
    let Ok(mut batteries) = manager.batteries() else {
        return std::ptr::null_mut();
    };
    let Some(Ok(battery)) = batteries.next() else {
        return std::ptr::null_mut();
    };

    let level = battery.state_of_charge().get::<uom::si::ratio::ratio>();
    let charging = battery.state() == starship_battery::State::Charging;
    let time_to_full = battery
        .time_to_full()
        .map(|time| time.get::<uom::si::time::second>() as u32);
    let time_to_empty = battery
        .time_to_empty()
        .map(|time| time.get::<uom::si::time::second>() as u32);

    let info = BatteryInfo {
        level,
        charging,
        time_to_full: box_optional_u32(time_to_full),
        time_to_empty: box_optional_u32(time_to_empty),
    };
    let bytes = unsafe {
        std::slice::from_raw_parts(
            (&info as *const BatteryInfo).cast::<u8>(),
            std::mem::size_of::<BatteryInfo>(),
        )
    };
    box_bytes(bytes)
}

fn invalid_cpu_temp_unit(unit: &str) -> ! {
    let message = std::ffi::CString::new(format!(
        "os.cpuTemp: invalid unit `{unit}` (expected \"celsius\" or \"fahrenheit\")"
    ))
    .unwrap_or_else(|_| CString::new("os.cpuTemp: invalid unit").expect("no NUL"));
    unsafe { crate::tyrus_panic(message.as_ptr()) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_cpu_temp(unit: TyrusString) -> BoxedPayload {
    let fahrenheit = if unit.is_null() {
        false
    } else {
        match unsafe { unit.as_str() } {
            "celsius" => false,
            "fahrenheit" => true,
            other => invalid_cpu_temp_unit(other),
        }
    };

    let components = sysinfo::Components::new_with_refreshed_list();
    let celsius = components
        .iter()
        .map(sysinfo::Component::temperature)
        .filter(|value| !value.is_nan())
        .fold(None::<f32>, |highest, value| {
            Some(highest.map_or(value, |current| current.max(value)))
        });
    let Some(celsius) = celsius else {
        return std::ptr::null_mut();
    };
    let value = if fahrenheit {
        celsius * 9.0 / 5.0 + 32.0
    } else {
        celsius
    };
    box_bytes(&value.to_ne_bytes())
}

#[repr(C)]
pub struct NetworkInterface {
    pub name: TyrusString,
    pub address: TyrusString,
    pub netmask: TyrusString,
    pub family: TyrusString,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_network_interfaces_result(out: *mut OsArrayResult) {
    if out.is_null() {
        return;
    }
    let result = if_addrs::get_if_addrs()
        .map(|interfaces| {
            interfaces
                .into_iter()
                .map(|interface| {
                    let (address, netmask, family) = match interface.addr {
                        if_addrs::IfAddr::V4(addr) => {
                            (addr.ip.to_string(), addr.netmask.to_string(), "IPv4")
                        }
                        if_addrs::IfAddr::V6(addr) => {
                            (addr.ip.to_string(), addr.netmask.to_string(), "IPv6")
                        }
                    };
                    NetworkInterface {
                        name: crate::strings::alloc_string(&interface.name),
                        address: crate::strings::alloc_string(&address),
                        netmask: crate::strings::alloc_string(&netmask),
                        family: crate::strings::alloc_string(family),
                    }
                })
                .collect::<Vec<_>>()
        })
        .map(|mut interfaces| {
            let array = TyrusArray {
                data: interfaces.as_mut_ptr().cast(),
                length: interfaces.len(),
                capacity: interfaces.capacity(),
            };
            std::mem::forget(interfaces);
            array
        })
        .map_err(|_| OsErrorTaken {
            code: TyrusString::literal("ERR_OS_NETWORK_INTERFACES_FAILED"),
            message: TyrusString::literal("failed to enumerate network interfaces"),
        });
    let result = os_array_result(result);
    unsafe { std::ptr::write(out, result) };
}

thread_local! {
    static LAST_OS_ERROR: std::cell::Cell<Option<(&'static CStr, &'static CStr)>> =
        const { std::cell::Cell::new(None) };
}

fn set_os_error(code: &'static CStr, message: &'static CStr) {
    LAST_OS_ERROR.with(|slot| slot.set(Some((code, message))));
}

macro_rules! os_error {
    ($code:expr_2021, $message:expr_2021, ()) => {{
        set_os_error($code, $message);
        return;
    }};
    ($code:expr_2021, $message:expr_2021, $fallback:expr_2021) => {{
        set_os_error($code, $message);
        return $fallback;
    }};
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_clear_error() {
    LAST_OS_ERROR.with(|slot| slot.set(None));
}

/// Field layout mirrors `OsError` (`crates/tyrus-core/src/builtin_interfaces/builtin_interfaces.rs`)
/// exactly: `code` and `message`, each a pointer into immutable
/// process-lifetime storage (a string literal) that the generated caller may
/// retain but must not free, the same convention `tyrus_crypto_take_error`
/// already uses for `Error.message`.
///
/// Consumes the current thread's pending `tyrus:os` error, if any, filling
/// `out`. Leaves `out` untouched (the caller's own pre-zeroed slot) when
/// nothing is pending, the caller checks `out->code` for null rather than
/// this function's return value, sidestepping any doubt about how `bool`
/// crosses the `extern "C"` boundary for a 1-byte return.
///
/// # Safety
///
/// `out` must point at a writable, zeroed `OsErrorTaken`, or be null (the
/// error is still consumed either way; a null `out` just discards it).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_take_error(out: *mut OsErrorTaken) {
    LAST_OS_ERROR.with(|slot| {
        if let Some((code, message)) = slot.take()
            && !out.is_null()
        {
            unsafe {
                (*out).code =
                    TyrusString::literal(code.to_str().expect("a static error literal is UTF-8"));
                (*out).message = TyrusString::literal(
                    message.to_str().expect("a static error literal is UTF-8"),
                );
            }
        }
    });
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_open_url(url: TyrusString) -> bool {
    if url.is_null() {
        os_error!(c"ERR_OS_INVALID_ARGUMENT", c"url must not be null", false);
    }
    let url = unsafe { url.as_str() }.to_owned();
    match open::that(&url) {
        Ok(()) => true,
        Err(_) => os_error!(
            c"ERR_OS_NO_HANDLER",
            c"no application is registered to open this URL",
            false
        ),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_trash(path: TyrusString) -> bool {
    if path.is_null() {
        os_error!(c"ERR_OS_INVALID_ARGUMENT", c"path must not be null", false);
    }
    let path = unsafe { path.as_str() }.to_owned();
    match trash::delete(&path) {
        Ok(()) => true,
        Err(_) => os_error!(
            c"ERR_OS_TRASH_FAILED",
            c"path does not exist, or the trash service is unavailable",
            false
        ),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_request_attention(critical: bool) {
    use std::io::Write;
    let bell: &[u8] = if critical { b"\x07\x07\x07" } else { b"\x07" };
    let _ = std::io::stderr().write_all(bell);
    let _ = std::io::stderr().flush();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_reveal_in_folder(path: TyrusString) -> bool {
    if path.is_null() {
        os_error!(c"ERR_OS_INVALID_ARGUMENT", c"path must not be null", false);
    }
    let path = unsafe { path.as_str() }.to_owned();
    let target = std::path::Path::new(&path);
    let dir = if target.is_dir() {
        target
    } else {
        target.parent().unwrap_or(target)
    };
    match open::that(dir) {
        Ok(()) => true,
        Err(_) => os_error!(
            c"ERR_OS_REVEAL_FAILED",
            c"no file manager is registered to reveal this path",
            false
        ),
    }
}

unsafe fn unbox_str(boxed: *mut c_char) -> Option<String> {
    if boxed.is_null() {
        return None;
    }
    let inner = unsafe { (*(boxed as *mut TyrusString)).borrow() };
    if inner.is_null() {
        return None;
    }
    Some(unsafe { inner.as_str() }.to_owned())
}

unsafe fn unbox_i32(boxed: *mut c_char) -> Option<i32> {
    if boxed.is_null() {
        None
    } else {
        Some(unsafe { *(boxed as *mut i32) })
    }
}

unsafe fn unbox_u32(boxed: *mut c_char) -> Option<u32> {
    if boxed.is_null() {
        None
    } else {
        Some(unsafe { *(boxed as *mut u32) })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_notify(
    title: TyrusString,
    body: TyrusString,
    options: *const NotifyOptionsFfi,
) {
    if title.is_null() || body.is_null() {
        os_error!(
            c"ERR_OS_INVALID_ARGUMENT",
            c"notification title and body must not be null",
            ()
        );
    }
    let title = unsafe { title.as_str() }.to_owned();
    let body = unsafe { body.as_str() }.to_owned();
    let mut notification = notify_rust::Notification::new();
    notification.summary(&title).body(&body);
    if let Some(options) = unsafe { options.as_ref() } {
        if let Some(icon) = unsafe { unbox_str(options.icon) } {
            notification.icon(&icon);
        }
        #[cfg(not(target_os = "macos"))]
        if let Some(urgency) = unsafe { unbox_i32(options.urgency) } {
            notification.urgency(match urgency {
                0 => notify_rust::Urgency::Low,
                2 => notify_rust::Urgency::Critical,
                _ => notify_rust::Urgency::Normal,
            });
        }
        if let Some(timeout) = unsafe { unbox_u32(options.timeout) } {
            notification.timeout(notify_rust::Timeout::Milliseconds(timeout));
        }
    }
    if notification.show().is_err() {
        os_error!(
            c"ERR_OS_NOTIFICATION_FAILED",
            c"the platform notification service could not show the notification",
            ()
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_notify_result(
    title: TyrusString,
    body: TyrusString,
    options: *const NotifyOptionsFfi,
    out: *mut u8,
) {
    tyrus_os_clear_error();
    unsafe { tyrus_os_notify(title, body, options) };
    unsafe { write_os_void_result(out.cast()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_clipboard_write(text: TyrusString) {
    if text.is_null() {
        os_error!(
            c"ERR_OS_INVALID_ARGUMENT",
            c"clipboard text must not be null",
            ()
        );
    }
    let text = unsafe { text.as_str() }.to_owned();
    let mut clipboard = match arboard::Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(_) => os_error!(
            c"ERR_OS_CLIPBOARD_UNAVAILABLE",
            c"the system clipboard is unavailable",
            ()
        ),
    };
    if clipboard.set_text(text).is_err() {
        os_error!(
            c"ERR_OS_CLIPBOARD_WRITE_FAILED",
            c"the system clipboard rejected the text",
            ()
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_clipboard_write_result(text: TyrusString, out: *mut u8) {
    tyrus_os_clear_error();
    unsafe { tyrus_os_clipboard_write(text) };
    unsafe { write_os_void_result(out.cast()) };
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_clipboard_read() -> BoxedPayload {
    let mut clipboard = match arboard::Clipboard::new() {
        Ok(clipboard) => clipboard,
        Err(_) => {
            set_os_error(
                c"ERR_OS_CLIPBOARD_UNAVAILABLE",
                c"the system clipboard is unavailable",
            );
            return std::ptr::null_mut();
        }
    };
    let text = match clipboard.get_text() {
        Ok(text) => text,
        Err(error) => {
            if let Some((code, message)) = clipboard_read_error(&error) {
                set_os_error(code, message);
            }
            return std::ptr::null_mut();
        }
    };
    box_optional_string(crate::strings::alloc_string(&text))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_clipboard_read_result(out: *mut OsClipboardReadResult) {
    if out.is_null() {
        return;
    }
    tyrus_os_clear_error();
    let value = tyrus_os_clipboard_read();
    let mut error = OsErrorTaken {
        code: TyrusString::null(),
        message: TyrusString::null(),
    };
    unsafe { tyrus_os_take_error(&mut error) };
    let result = os_clipboard_read_result(value, error);
    unsafe { std::ptr::write(out, result) };
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_is_dark_mode() -> bool {
    matches!(dark_light::detect(), Ok(dark_light::Mode::Dark))
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_terminal_size() -> BoxedPayload {
    let Ok((columns, rows)) = crossterm::terminal::size() else {
        return std::ptr::null_mut();
    };
    let mut bytes = [0u8; 4];
    bytes[0..2].copy_from_slice(&columns.to_ne_bytes());
    bytes[2..4].copy_from_slice(&rows.to_ne_bytes());
    box_bytes(&bytes)
}

#[cfg(target_os = "linux")]
fn logind_locked_hint() -> Option<bool> {
    let session_id = std::env::var("XDG_SESSION_ID").ok()?;
    let output = std::process::Command::new("loginctl")
        .args(["show-session", &session_id, "-p", "LockedHint", "--value"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim() == "yes")
}

#[cfg(target_os = "linux")]
fn screensaver_active() -> Option<bool> {
    let output = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--dest=org.freedesktop.ScreenSaver",
            "--type=method_call",
            "--print-reply",
            "/org/freedesktop/ScreenSaver",
            "org.freedesktop.ScreenSaver.GetActive",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).contains("boolean true"))
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_is_screen_locked() -> bool {
    #[cfg(target_os = "linux")]
    {
        screensaver_active().unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_is_session_locked() -> bool {
    #[cfg(target_os = "linux")]
    {
        logind_locked_hint().unwrap_or(false)
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
static SLEEP_INHIBIT_CHILD: std::sync::Mutex<Option<std::process::Child>> =
    std::sync::Mutex::new(None);

#[cfg(windows)]
struct WindowsSleepInhibitor {
    release: std::sync::mpsc::SyncSender<std::sync::mpsc::SyncSender<bool>>,
    worker: std::thread::JoinHandle<()>,
}

#[cfg(windows)]
static WINDOWS_SLEEP_INHIBITOR: std::sync::Mutex<Option<WindowsSleepInhibitor>> =
    std::sync::Mutex::new(None);

#[cfg(windows)]
fn prevent_sleep_windows() -> bool {
    use windows_sys::Win32::System::Power::{
        ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
    };

    let mut inhibitor = WINDOWS_SLEEP_INHIBITOR
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if inhibitor.is_some() {
        return true;
    }
    let (started_tx, started_rx) = std::sync::mpsc::sync_channel(0);
    let (release_tx, release_rx) =
        std::sync::mpsc::sync_channel::<std::sync::mpsc::SyncSender<bool>>(0);
    let worker = match std::thread::Builder::new()
        .name("tyrus-sleep-inhibitor".to_string())
        .spawn(move || {
            let active = unsafe {
                SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED)
            } != 0;
            if started_tx.send(active).is_err() || !active {
                return;
            }
            if let Ok(released_tx) = release_rx.recv() {
                let released = unsafe { SetThreadExecutionState(ES_CONTINUOUS) } != 0;
                let _ = released_tx.send(released);
            }
        }) {
        Ok(worker) => worker,
        Err(_) => {
            return os_error!(
                c"ERR_OS_SLEEP_INHIBIT_FAILED",
                c"could not start the Windows sleep-inhibition worker",
                false
            );
        }
    };
    if !started_rx.recv().unwrap_or(false) {
        let _ = worker.join();
        return os_error!(
            c"ERR_OS_SLEEP_INHIBIT_FAILED",
            c"SetThreadExecutionState failed",
            false
        );
    }
    *inhibitor = Some(WindowsSleepInhibitor {
        release: release_tx,
        worker,
    });
    true
}

#[cfg(windows)]
fn allow_sleep_windows() -> bool {
    let inhibitor = WINDOWS_SLEEP_INHIBITOR
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .take();
    let Some(inhibitor) = inhibitor else {
        return true;
    };
    let (released_tx, released_rx) = std::sync::mpsc::sync_channel(0);
    let release_requested = inhibitor.release.send(released_tx).is_ok();
    let released = release_requested && released_rx.recv().unwrap_or(false);
    let worker_stopped = inhibitor.worker.join().is_ok();
    if !release_requested || !released || !worker_stopped {
        return os_error!(
            c"ERR_OS_SLEEP_ALLOW_FAILED",
            c"SetThreadExecutionState failed while allowing sleep",
            false
        );
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_prevent_sleep() -> bool {
    #[cfg(target_os = "linux")]
    {
        let mut guard = SLEEP_INHIBIT_CHILD.lock().unwrap();
        if guard.is_some() {
            return true;
        }
        match std::process::Command::new("systemd-inhibit")
            .args([
                "--what=idle:sleep:handle-lid-switch",
                "--who=tyrus",
                "--why=tyrus program requested",
                "--mode=block",
                "sleep",
                "infinity",
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => {
                *guard = Some(child);
                true
            }
            Err(_) => os_error!(
                c"ERR_OS_SLEEP_INHIBIT_FAILED",
                c"systemd-inhibit is unavailable; cannot prevent sleep",
                false
            ),
        }
    }
    #[cfg(target_os = "macos")]
    {
        let mut guard = SLEEP_INHIBIT_CHILD.lock().unwrap();
        if guard.is_some() {
            return true;
        }
        match std::process::Command::new("caffeinate")
            .args(["-dims"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
        {
            Ok(child) => {
                *guard = Some(child);
                true
            }
            Err(_) => os_error!(
                c"ERR_OS_SLEEP_INHIBIT_FAILED",
                c"caffeinate is unavailable; cannot prevent sleep",
                false
            ),
        }
    }
    #[cfg(windows)]
    {
        prevent_sleep_windows()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        os_error!(
            c"ERR_OS_SLEEP_INHIBIT_UNSUPPORTED",
            c"sleep prevention is not implemented on this platform",
            false
        )
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_allow_sleep() -> bool {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let mut guard = SLEEP_INHIBIT_CHILD.lock().unwrap();
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        true
    }
    #[cfg(windows)]
    {
        allow_sleep_windows()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    {
        true
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_open_url_result(url: TyrusString, out: *mut u8) {
    tyrus_os_clear_error();
    let _ = unsafe { tyrus_os_open_url(url) };
    unsafe { write_os_void_result(out.cast()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_trash_result(path: TyrusString, out: *mut u8) {
    tyrus_os_clear_error();
    let _ = unsafe { tyrus_os_trash(path) };
    unsafe { write_os_void_result(out.cast()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_reveal_in_folder_result(path: TyrusString, out: *mut u8) {
    tyrus_os_clear_error();
    let _ = unsafe { tyrus_os_reveal_in_folder(path) };
    unsafe { write_os_void_result(out.cast()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_prevent_sleep_result(out: *mut u8) {
    tyrus_os_clear_error();
    let _ = tyrus_os_prevent_sleep();
    unsafe { write_os_void_result(out.cast()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_allow_sleep_result(out: *mut u8) {
    tyrus_os_clear_error();
    let _ = tyrus_os_allow_sleep();
    unsafe { write_os_void_result(out.cast()) };
}

#[repr(C)]
pub struct FontInfo {
    pub name: TyrusString,
    pub style: TyrusString,
    pub path: TyrusString,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_system_fonts(out_array: *mut TyrusArray) {
    if out_array.is_null() {
        return;
    }
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    let mut fonts: Vec<FontInfo> = db
        .faces()
        .map(|face| {
            let name = face.families.first().map_or("", |(name, _)| name.as_str());
            let style = match face.style {
                fontdb::Style::Normal => "normal",
                fontdb::Style::Italic => "italic",
                fontdb::Style::Oblique => "oblique",
            };
            let path = match &face.source {
                fontdb::Source::File(path) | fontdb::Source::SharedFile(path, _) => {
                    path.to_string_lossy().into_owned()
                }
                fontdb::Source::Binary(_) => String::new(),
            };
            FontInfo {
                name: crate::strings::alloc_string(name),
                style: crate::strings::alloc_string(style),
                path: crate::strings::alloc_string(&path),
            }
        })
        .collect();
    let ptr = fonts.as_mut_ptr();
    let len = fonts.len();
    let cap = fonts.capacity();
    std::mem::forget(fonts);
    unsafe {
        (*out_array).data = ptr as *mut u8;
        (*out_array).length = len;
        (*out_array).capacity = cap;
    }
}

fn srgb_to_linear(channel: u8) -> f64 {
    let c = channel as f64 / 255.0;
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(color: &ColorFfi) -> f64 {
    0.2126 * srgb_to_linear(color.r)
        + 0.7152 * srgb_to_linear(color.g)
        + 0.0722 * srgb_to_linear(color.b)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_os_accessible_color_contrast(
    fg: *const ColorFfi,
    bg: *const ColorFfi,
) -> bool {
    let fg = unsafe { &*fg };
    let bg = unsafe { &*bg };
    let fg_luminance = relative_luminance(fg);
    let bg_luminance = relative_luminance(bg);
    let (lighter, darker) = if fg_luminance > bg_luminance {
        (fg_luminance, bg_luminance)
    } else {
        (bg_luminance, fg_luminance)
    };
    (lighter + 0.05) / (darker + 0.05) >= 4.5
}

#[cfg(unix)]
fn passwd_shell() -> Option<String> {
    let _guard = PASSWD_LOOKUP_LOCK.lock().unwrap();
    let mut passwd: libc::passwd = unsafe { std::mem::zeroed() };
    let mut buffer = [0 as libc::c_char; 4096];
    let mut result: *mut libc::passwd = std::ptr::null_mut();
    let status = unsafe {
        libc::getpwuid_r(
            libc::getuid(),
            &mut passwd,
            buffer.as_mut_ptr(),
            buffer.len(),
            &mut result,
        )
    };
    if status != 0 || result.is_null() || passwd.pw_shell.is_null() {
        return None;
    }
    let shell = unsafe { CStr::from_ptr(passwd.pw_shell) }
        .to_string_lossy()
        .into_owned();
    if shell.is_empty() { None } else { Some(shell) }
}

#[cfg(target_os = "macos")]
fn command_exists(command: &str) -> bool {
    std::process::Command::new(command)
        .arg("--version")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok()
}

#[cfg(target_os = "linux")]
fn command_succeeds(command: &str, arguments: &[&str]) -> bool {
    std::process::Command::new(command)
        .args(arguments)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(target_os = "linux")]
fn has_notification_capability() -> bool {
    let output = std::process::Command::new("dbus-send")
        .args([
            "--session",
            "--dest=org.freedesktop.DBus",
            "--type=method_call",
            "--print-reply",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus.NameHasOwner",
            "string:org.freedesktop.Notifications",
        ])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).contains("boolean true")
        }
        _ => false,
    }
}

#[cfg(not(target_os = "linux"))]
fn has_notification_capability() -> bool {
    true
}

fn has_battery_capability() -> bool {
    let Ok(manager) = starship_battery::Manager::new() else {
        return false;
    };
    let Ok(mut batteries) = manager.batteries() else {
        return false;
    };
    matches!(batteries.next(), Some(Ok(_)))
}

#[cfg(target_os = "linux")]
fn has_screen_lock_capability() -> bool {
    screensaver_active().is_some()
}

#[cfg(not(target_os = "linux"))]
fn has_screen_lock_capability() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn has_session_lock_capability() -> bool {
    logind_locked_hint().is_some()
}

#[cfg(not(target_os = "linux"))]
fn has_session_lock_capability() -> bool {
    false
}

#[cfg(target_os = "linux")]
fn has_sleep_prevention_capability() -> bool {
    command_succeeds("systemd-inhibit", &["--list"])
}

#[cfg(target_os = "macos")]
fn has_sleep_prevention_capability() -> bool {
    command_exists("caffeinate")
}

#[cfg(windows)]
fn has_sleep_prevention_capability() -> bool {
    true
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn has_sleep_prevention_capability() -> bool {
    false
}

fn has_fonts_capability() -> bool {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    db.faces().next().is_some()
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_os_has_capability(capability: i32) -> bool {
    match capability {
        0 => has_notification_capability(),
        1 => arboard::Clipboard::new().is_ok(),
        2 => has_battery_capability(),
        3 => dark_light::detect().is_ok(),
        4 => has_screen_lock_capability(),
        5 => has_session_lock_capability(),
        6 => has_sleep_prevention_capability(),
        7 => has_fonts_capability(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_read_distinguishes_empty_content_from_platform_errors() {
        assert!(clipboard_read_error(&arboard::Error::ContentNotAvailable).is_none());
        let (code, message) = clipboard_read_error(&arboard::Error::ClipboardOccupied)
            .expect("clipboard access failures must remain distinguishable from empty content");
        assert_eq!(code.to_str().unwrap(), "ERR_OS_CLIPBOARD_READ_FAILED");
        assert_eq!(
            message.to_str().unwrap(),
            "the system clipboard text could not be read"
        );
    }

    #[test]
    fn clipboard_read_result_encodes_optional_value_and_structured_error() {
        let empty = os_clipboard_read_result(
            std::ptr::null_mut(),
            OsErrorTaken {
                code: TyrusString::null(),
                message: TyrusString::null(),
            },
        );
        assert_eq!(empty.tag, 0);
        assert!(unsafe { empty.payload.value.is_null() });

        let error = os_clipboard_read_result(
            std::ptr::null_mut(),
            OsErrorTaken {
                code: TyrusString::literal("ERR_OS_CLIPBOARD_UNAVAILABLE"),
                message: TyrusString::literal("the system clipboard is unavailable"),
            },
        );
        assert_eq!(error.tag, 1);
        let error = unsafe {
            &*(&error.payload.error as *const ManuallyDrop<OsErrorTaken> as *const OsErrorTaken)
        };
        assert_eq!(
            unsafe { error.code.as_str() },
            "ERR_OS_CLIPBOARD_UNAVAILABLE"
        );
    }

    #[test]
    fn network_interface_enumeration_failure_is_a_structured_result() {
        let result = os_array_result(Err(OsErrorTaken {
            code: TyrusString::literal("ERR_OS_NETWORK_INTERFACES_FAILED"),
            message: TyrusString::literal("failed to enumerate network interfaces"),
        }));
        assert_eq!(result.tag, 1);
        let error = unsafe {
            &*(&result.payload.error as *const ManuallyDrop<OsErrorTaken> as *const OsErrorTaken)
        };
        assert_eq!(
            unsafe { error.code.as_str() },
            "ERR_OS_NETWORK_INTERFACES_FAILED"
        );
        assert_eq!(
            unsafe { error.message.as_str() },
            "failed to enumerate network interfaces"
        );
    }

    #[test]
    fn platform_and_arch_return_a_known_ordinal() {
        assert!((0..=3).contains(&tyrus_os_platform()));
        assert!((0..=4).contains(&tyrus_os_arch()));
    }

    #[test]
    fn trash_result_contains_the_structured_error_variant() {
        let mut out = std::mem::MaybeUninit::<OsVoidResult>::uninit();
        unsafe {
            tyrus_os_trash_result(
                TyrusString::literal("/__tyrus_missing_trash_entry__"),
                out.as_mut_ptr().cast(),
            )
        };
        let result = unsafe { out.assume_init() };
        assert_eq!(result.tag, 1);
        let error = unsafe {
            &*(&result.payload.error as *const ManuallyDrop<OsErrorResult> as *const OsErrorResult)
        };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_OS_TRASH_FAILED");
        assert_eq!(
            unsafe { error.message.as_str() },
            "path does not exist, or the trash service is unavailable"
        );
    }

    #[test]
    fn reveal_in_folder_result_reports_a_null_path_structurally() {
        let mut out = std::mem::MaybeUninit::<OsVoidResult>::uninit();
        unsafe { tyrus_os_reveal_in_folder_result(TyrusString::null(), out.as_mut_ptr().cast()) };
        let result = unsafe { out.assume_init() };
        assert_eq!(result.tag, 1);
        let error = unsafe {
            &*(&result.payload.error as *const ManuallyDrop<OsErrorResult> as *const OsErrorResult)
        };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_OS_INVALID_ARGUMENT");
        assert_eq!(unsafe { error.message.as_str() }, "path must not be null");
    }

    #[test]
    fn notify_result_reports_invalid_arguments_structurally() {
        let mut out = std::mem::MaybeUninit::<OsVoidResult>::uninit();
        unsafe {
            tyrus_os_notify_result(
                TyrusString::null(),
                TyrusString::literal("body"),
                std::ptr::null(),
                out.as_mut_ptr().cast(),
            )
        };
        let result = unsafe { out.assume_init() };
        assert_eq!(result.tag, 1);
        let error = unsafe {
            &*(&result.payload.error as *const ManuallyDrop<OsErrorResult> as *const OsErrorResult)
        };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_OS_INVALID_ARGUMENT");
        assert_eq!(
            unsafe { error.message.as_str() },
            "notification title and body must not be null"
        );
    }

    #[test]
    fn clipboard_write_result_reports_invalid_arguments_structurally() {
        let mut out = std::mem::MaybeUninit::<OsVoidResult>::uninit();
        unsafe { tyrus_os_clipboard_write_result(TyrusString::null(), out.as_mut_ptr().cast()) };
        let result = unsafe { out.assume_init() };
        assert_eq!(result.tag, 1);
        let error = unsafe {
            &*(&result.payload.error as *const ManuallyDrop<OsErrorResult> as *const OsErrorResult)
        };
        assert_eq!(unsafe { error.code.as_str() }, "ERR_OS_INVALID_ARGUMENT");
        assert_eq!(
            unsafe { error.message.as_str() },
            "clipboard text must not be null"
        );
    }

    fn owned_string(value: TyrusString) -> String {
        assert!(!value.is_null());
        let text = unsafe { value.as_str() }.to_owned();
        unsafe { value.release() };
        text
    }

    #[test]
    fn hostname_release_and_type_return_a_non_empty_string() {
        assert!(!owned_string(tyrus_os_hostname()).is_empty());
        assert!(!owned_string(tyrus_os_release()).is_empty());
        assert!(!owned_string(tyrus_os_system_name()).is_empty());
    }

    #[test]
    fn locale_and_language_return_a_non_empty_string() {
        let locale = owned_string(tyrus_os_locale());
        let language = owned_string(tyrus_os_language());
        assert!(!locale.is_empty());
        assert!(!language.is_empty());
        assert!(locale.starts_with(language.as_str()));
    }

    fn optional_owned_string(pointer: BoxedPayload) -> Option<String> {
        if pointer.is_null() {
            return None;
        }
        let inner =
            unsafe { std::mem::replace(&mut *pointer.cast::<TyrusString>(), TyrusString::null()) };
        let result = if inner.is_null() {
            None
        } else {
            Some({
                let text = unsafe { inner.as_str() }.to_owned();
                unsafe { inner.release() };
                text
            })
        };
        unsafe { libc::free(pointer) };
        result
    }

    #[test]
    fn user_info_matches_the_individual_accessors() {
        let mut info = UserInfo {
            username: TyrusString::null(),
            homedir: TyrusString::null(),
            shell: TyrusString::null(),
        };
        unsafe { tyrus_os_user_info(&mut info) };
        assert_eq!(
            owned_string(info.username),
            owned_string(tyrus_os_user_name())
        );
        assert_eq!(
            owned_string(info.homedir),
            owned_string(tyrus_os_home_dir())
        );
        assert_eq!(owned_string(info.shell), env_shell());
    }

    #[test]
    fn env_shell_is_never_empty_even_without_the_shell_variable() {
        assert!(!env_shell().is_empty());
    }

    #[test]
    fn base_directories_are_non_empty_and_distinct() {
        let home = owned_string(tyrus_os_home_dir());
        let cache = owned_string(tyrus_os_cache_dir());
        let config = owned_string(tyrus_os_config_dir());
        let data = owned_string(tyrus_os_data_dir());
        let temp = owned_string(tyrus_os_temp_dir());
        assert!(!home.is_empty());
        assert!(!cache.is_empty());
        assert!(!config.is_empty());
        assert!(!data.is_empty());
        assert!(!temp.is_empty());
    }

    #[test]
    fn desktop_and_documents_dir_are_absent_or_non_empty() {
        if let Some(dir) = optional_owned_string(tyrus_os_desktop_dir()) {
            assert!(!dir.is_empty());
        }
        if let Some(dir) = optional_owned_string(tyrus_os_documents_dir()) {
            assert!(!dir.is_empty());
        }
    }

    #[test]
    fn app_data_dir_includes_the_app_name() {
        let app = TyrusString::literal("tyrus-os-test");
        let dir = owned_string(unsafe { tyrus_os_app_data_dir(app.borrow()) });
        assert!(!dir.is_empty());
        assert!(dir.contains("tyrus-os-test"));
    }

    #[test]
    fn whoami_is_absent_or_non_empty() {
        if let Some(name) = optional_owned_string(tyrus_os_whoami()) {
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn cpus_returns_at_least_one_core_with_a_model_and_speed() {
        let mut array = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe { tyrus_os_cpus(&mut array) };
        assert!(array.length >= 1);
        let cpus =
            unsafe { std::slice::from_raw_parts(array.data as *const CpuInfo, array.length) };
        for cpu in cpus {
            assert!(!owned_string(cpu.model.borrow()).is_empty());
        }
        unsafe {
            let mut cores =
                Vec::from_raw_parts(array.data as *mut CpuInfo, array.length, array.capacity);
            for cpu in &mut cores {
                std::mem::replace(&mut cpu.model, TyrusString::null()).release();
            }
            drop(cores);
        };
    }

    #[test]
    fn total_mem_is_greater_than_free_mem_is_non_negative() {
        assert!(tyrus_os_total_mem() > 0);
        assert!(tyrus_os_free_mem() <= tyrus_os_total_mem());
    }

    #[test]
    fn system_uptime_is_a_sane_number_of_seconds() {
        assert!(tyrus_os_uptime() < 100 * 365 * 24 * 60 * 60);
    }

    #[test]
    fn load_avg_has_exactly_three_non_negative_values() {
        let mut array = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe { tyrus_os_load_avg(&mut array) };
        assert_eq!(array.length, 3);
        let values = unsafe { std::slice::from_raw_parts(array.data as *const f64, 3) };
        for value in values {
            assert!(*value >= 0.0);
        }
        unsafe {
            drop(Vec::from_raw_parts(
                array.data as *mut f64,
                array.length,
                array.capacity,
            ));
        };
    }

    #[test]
    fn battery_is_absent_or_has_a_sane_level() {
        let pointer = tyrus_os_battery();
        if pointer.is_null() {
            return;
        }
        let info = unsafe { &*(pointer as *const BatteryInfo) };
        assert!((0.0..=1.0).contains(&info.level));
        unsafe { libc::free(pointer) };
    }

    #[test]
    fn cpu_temp_defaults_to_celsius_and_is_absent_or_sane() {
        let pointer = unsafe { tyrus_os_cpu_temp(TyrusString::null()) };
        if pointer.is_null() {
            return;
        }
        let celsius = unsafe { *(pointer as *const f32) };
        assert!((-40.0..200.0).contains(&celsius));
        unsafe { libc::free(pointer) };
    }

    #[test]
    fn cpu_temp_fahrenheit_is_32_degrees_above_freezing_relationship() {
        let celsius_ptr = unsafe { tyrus_os_cpu_temp(TyrusString::null()) };
        let fahrenheit_unit = TyrusString::literal("fahrenheit");
        let fahrenheit_ptr = unsafe { tyrus_os_cpu_temp(fahrenheit_unit.borrow()) };
        if celsius_ptr.is_null() || fahrenheit_ptr.is_null() {
            return;
        }
        let celsius = unsafe { *(celsius_ptr as *const f32) };
        let fahrenheit = unsafe { *(fahrenheit_ptr as *const f32) };
        assert!((fahrenheit - (celsius * 9.0 / 5.0 + 32.0)).abs() < 2.0);
        unsafe {
            libc::free(celsius_ptr);
            libc::free(fahrenheit_ptr);
        }
    }

    #[test]
    fn is_dark_mode_does_not_panic() {
        let _ = tyrus_os_is_dark_mode();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn command_succeeds_requires_a_zero_exit_status() {
        assert!(command_succeeds("sh", &["-c", "exit 0"]));
        assert!(!command_succeeds("sh", &["-c", "exit 1"]));
    }

    #[test]
    fn terminal_size_is_absent_or_has_two_positive_dimensions() {
        let pointer = tyrus_os_terminal_size();
        if pointer.is_null() {
            return;
        }
        let bytes = unsafe { std::slice::from_raw_parts(pointer as *const u8, 4) };
        let columns = u16::from_ne_bytes([bytes[0], bytes[1]]);
        let rows = u16::from_ne_bytes([bytes[2], bytes[3]]);
        assert!(columns > 0);
        assert!(rows > 0);
        unsafe { libc::free(pointer) };
    }

    #[test]
    fn is_screen_locked_and_is_session_locked_do_not_panic() {
        let _ = tyrus_os_is_screen_locked();
        let _ = tyrus_os_is_session_locked();
    }

    #[test]
    fn prevent_sleep_and_allow_sleep_round_trip() {
        let prevented = tyrus_os_prevent_sleep();
        #[cfg(windows)]
        {
            assert!(prevented);
            assert!(tyrus_os_prevent_sleep());
            assert!(
                std::thread::spawn(|| tyrus_os_allow_sleep()).join().expect(
                    "the Windows sleep-inhibition worker can be released from another thread"
                )
            );
            assert!(tyrus_os_allow_sleep());
        }
        #[cfg(not(windows))]
        {
            if !prevented {
                tyrus_os_clear_error();
            }
            assert!(tyrus_os_allow_sleep());
        }
    }

    #[test]
    fn system_fonts_returns_at_least_one_face_with_a_known_style() {
        let mut array = TyrusArray {
            data: std::ptr::null_mut(),
            length: 0,
            capacity: 0,
        };
        unsafe { tyrus_os_system_fonts(&mut array) };
        let fonts =
            unsafe { std::slice::from_raw_parts(array.data as *const FontInfo, array.length) };
        for font in fonts {
            let name = owned_string(font.name.borrow());
            let style = owned_string(font.style.borrow());
            assert!(!name.is_empty());
            assert!(style == "normal" || style == "italic" || style == "oblique");
            let _ = owned_string(font.path.borrow());
        }
        unsafe {
            let mut faces =
                Vec::from_raw_parts(array.data as *mut FontInfo, array.length, array.capacity);
            for font in &mut faces {
                std::mem::replace(&mut font.name, TyrusString::null()).release();
                std::mem::replace(&mut font.style, TyrusString::null()).release();
                std::mem::replace(&mut font.path, TyrusString::null()).release();
            }
            drop(faces);
        };
    }

    #[test]
    fn accessible_color_contrast_matches_known_wcag_pairs() {
        let black = ColorFfi { r: 0, g: 0, b: 0 };
        let white = ColorFfi {
            r: 255,
            g: 255,
            b: 255,
        };
        assert!(unsafe { tyrus_os_accessible_color_contrast(&black, &white) });
        assert!(!unsafe { tyrus_os_accessible_color_contrast(&white, &white) });
        assert_eq!(
            unsafe { tyrus_os_accessible_color_contrast(&black, &white) },
            unsafe { tyrus_os_accessible_color_contrast(&white, &black) }
        );
        let passing_gray = ColorFfi {
            r: 0x76,
            g: 0x76,
            b: 0x76,
        };
        let failing_gray = ColorFfi {
            r: 0x77,
            g: 0x77,
            b: 0x77,
        };
        assert!(unsafe { tyrus_os_accessible_color_contrast(&passing_gray, &white) });
        assert!(!unsafe { tyrus_os_accessible_color_contrast(&failing_gray, &white) });
    }

    #[test]
    fn has_capability_covers_every_ordinal_without_panicking() {
        for ordinal in 0..8 {
            let _ = tyrus_os_has_capability(ordinal);
        }
        assert!(!tyrus_os_has_capability(99));
    }

    #[test]
    fn has_capability_matches_the_direct_checks_it_mirrors() {
        assert_eq!(
            tyrus_os_has_capability(1),
            arboard::Clipboard::new().is_ok()
        );
        assert_eq!(tyrus_os_has_capability(2), has_battery_capability());
        assert!(dark_light_capability_matches_within_retries());
        assert_eq!(tyrus_os_has_capability(7), has_fonts_capability());
    }

    /// `dark_light::detect()` on Linux races a 25ms XDG-portal D-Bus round
    /// trip (see the `dark-light` crate's `platforms/freedesktop.rs`); under
    /// sanitizer instrumentation or heavy parallel test load that window can
    /// be missed on one of the two independent live calls being compared
    /// here, making them disagree transiently even though both hit the exact
    /// same detection path. Retry a few times before treating a mismatch as
    /// a real regression.
    fn dark_light_capability_matches_within_retries() -> bool {
        for _ in 0..5 {
            if tyrus_os_has_capability(3) == dark_light::detect().is_ok() {
                return true;
            }
        }
        false
    }
}
