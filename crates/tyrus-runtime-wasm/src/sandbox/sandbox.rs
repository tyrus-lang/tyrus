use std::cell::Cell;
use tyrus_string::TyrusString;

#[derive(Clone, Copy)]
pub(crate) struct SandboxError {
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
}

pub(crate) struct ErrorSlot(Cell<Option<SandboxError>>, Cell<Option<*const u8>>);

impl ErrorSlot {
    pub(crate) const fn new() -> Self {
        Self(Cell::new(None), Cell::new(None))
    }

    pub(crate) fn set(&self, error: SandboxError) {
        if self.1.get() != Some(error.message.as_ptr()) {
            self.1.set(Some(error.message.as_ptr()));
            announce(error);
        }
        self.0.set(Some(error));
    }

    /// Move the pending error into `code_out`/`message_out`, leaving them
    /// untouched when nothing is pending, the caller checks its own pre-zeroed
    /// slot rather than a return value, exactly as the native runtime does.
    ///
    /// # Safety
    ///
    /// Both pointers must be null or point at writable `TyrusString` slots.
    pub(crate) unsafe fn take_into(
        &self,
        code_out: *mut TyrusString,
        message_out: *mut TyrusString,
    ) {
        unsafe {
            let Some(error) = self.0.take() else {
                return;
            };
            if !code_out.is_null() {
                *code_out = TyrusString::literal(error.code);
            }
            if !message_out.is_null() {
                *message_out = TyrusString::literal(error.message);
            }
        }
    }
}

unsafe impl Sync for ErrorSlot {}

fn announce(error: SandboxError) {
    unsafe {
        crate::output::write_stderr(error.code.as_bytes());
        crate::output::write_stderr(b": ");
        crate::output::write_stderr(error.message.as_bytes());
        crate::output::write_stderr(b"\n");
    }
}

pub(crate) static OS_ERROR: ErrorSlot = ErrorSlot::new();
pub(crate) static FS_ERROR: ErrorSlot = ErrorSlot::new();

macro_rules! sandbox_error {
    ($code:literal, $api:literal) => {
        $crate::sandbox::SandboxError {
            code: $code,
            message: concat!(
                "`",
                $api,
                "` is not available in the playground sandbox, which runs your program in the \
                 browser with no operating system, filesystem or network access"
            ),
        }
    };
}

macro_rules! unavailable {
    ($slot:expr_2021, $code:literal, $api:literal) => {{
        $slot.set($crate::sandbox::sandbox_error!($code, $api));
    }};
}

pub(crate) use {sandbox_error, unavailable};
