use std::io::IsTerminal;
use std::sync::OnceLock;
use tyrus_string::TyrusString;

fn resolve_color_enabled(is_terminal: bool) -> bool {
    if std::env::var("NO_COLOR").is_ok_and(|value| !value.is_empty()) {
        return false;
    }
    if std::env::var("FORCE_COLOR").is_ok_and(|value| !value.is_empty() && value != "0") {
        return true;
    }
    is_terminal
}

fn color_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| resolve_color_enabled(std::io::stdout().is_terminal()))
}

fn stderr_color_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| resolve_color_enabled(std::io::stderr().is_terminal()))
}

unsafe fn text_of(value: &TyrusString) -> String {
    if value.is_null() {
        String::new()
    } else {
        unsafe { value.as_str() }.to_owned()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_style(
    open: TyrusString,
    text: TyrusString,
    close: TyrusString,
) -> TyrusString {
    unsafe {
        let text = text_of(&text);
        let result = if color_enabled() {
            format!("{}{text}{}", text_of(&open), text_of(&close))
        } else {
            text
        };
        crate::strings::alloc_string(&result)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_console_stderr_color_enabled() -> bool {
    stderr_color_enabled()
}
