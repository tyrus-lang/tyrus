//! Interactive terminal prompts and the `isTTY` terminal query exposed to
//! generated tyrus programs through a C ABI, backed by the `inquire` crate for
//! prompts. Imported in tyrus as `tyrus:cli`.
//!
//! The prompt UI renders on stderr (inquire's default), keeping stdout reserved
//! for the program's own `console.log` output. When there is no interactive
//! terminal, or the user cancels, the process aborts via `tyrus_panic` (stderr,
//! non-zero exit), per the runtime error convention.
//!
//! `isTTY()` never aborts: it reports whether stdout is attached to a terminal,
//! which is exactly the fact a program needs before deciding whether to prompt.
//!
//! Each `*Opts` struct mirrors, field by field, the LLVM layout the code
//! generator materializes for the matching `tyrus:cli` option interface (see
//! `tyrus-core/src/cli.rs`). Optional fields are pointers: null means absent.

use super::*;
use inquire::validator::Validation;
use inquire::{CustomType, InquireError, MultiSelect, Password, PasswordDisplayMode, Select, Text};
use std::io::IsTerminal;
use tyrus_string::TyrusString;

type OptStr = *const TyrusString;

#[repr(C)]
pub struct InputOpts {
    message: TyrusString,
    default: OptStr,
    placeholder: OptStr,
    help: OptStr,
    min_length: *const i32,
    max_length: *const i32,
    pattern: OptStr,
    pattern_message: OptStr,
}

#[repr(C)]
pub struct PasswordOpts {
    message: TyrusString,
    help: OptStr,
    confirm: *const bool,
    confirm_message: OptStr,
    pattern: OptStr,
    pattern_message: OptStr,
}

#[repr(C)]
pub struct ConfirmOpts {
    message: TyrusString,
    default: *const bool,
    help: OptStr,
    yes: OptStr,
    no: OptStr,
}

#[repr(C)]
pub struct SelectOpts {
    message: TyrusString,
    choices: TyrusStringArray,
    default: OptStr,
    help: OptStr,
    page_size: *const i32,
}

#[repr(C)]
pub struct MultiSelectOpts {
    message: TyrusString,
    choices: TyrusStringArray,
    help: OptStr,
    page_size: *const i32,
    required: *const bool,
}

#[repr(C)]
pub struct NumberOpts {
    message: TyrusString,
    default: *const f64,
    min: *const f64,
    max: *const f64,
    help: OptStr,
}

#[repr(C)]
pub struct IntegerOpts {
    message: TyrusString,
    default: *const i64,
    min: *const i64,
    max: *const i64,
    help: OptStr,
}

fn cli_panic(message: String) -> ! {
    let message = CString::new(message)
        .unwrap_or_else(|_| CString::new("tyrus runtime error").expect("no NUL"));
    unsafe { tyrus_panic(message.as_ptr()) }
}

fn require_tty(prompt: &str) {
    if !std::io::stdin().is_terminal() {
        cli_panic(format!(
            "error: `{prompt}` requires an interactive terminal (stdin is not a TTY)"
        ));
    }
}

fn finish<T>(result: Result<T, InquireError>, prompt: &str) -> T {
    match result {
        Ok(value) => value,
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
            cli_panic(format!("error: `{prompt}` was cancelled"))
        }
        Err(InquireError::NotTTY) => cli_panic(format!(
            "error: `{prompt}` requires an interactive terminal (stdin is not a TTY)"
        )),
        Err(error) => cli_panic(format!("error: `{prompt}` failed: {error}")),
    }
}

fn required_text(value: TyrusString, prompt: &str) -> String {
    text(&value).unwrap_or_else(|()| cli_panic(format!("error: `{prompt}` requires a `message`")))
}

unsafe fn optional_text(value: OptStr) -> Option<String> {
    unsafe {
        if value.is_null() {
            return None;
        }
        text(&*value).ok()
    }
}

unsafe fn read_opt<T: Copy>(pointer: *const T) -> Option<T> {
    unsafe {
        if pointer.is_null() {
            None
        } else {
            Some(*pointer)
        }
    }
}

fn choices(array: &TyrusStringArray) -> Vec<String> {
    let length = array.length;
    if array.data.is_null() || length == 0 {
        return Vec::new();
    }
    let slice = unsafe { std::slice::from_raw_parts(array.data, length) };
    slice
        .iter()
        .map(|stored| text(stored).unwrap_or_default())
        .collect()
}

fn alloc(value: &str) -> TyrusString {
    crate::strings::alloc_string(value)
}

unsafe fn pattern_validator(
    pattern: OptStr,
    pattern_message: OptStr,
    prompt: &str,
) -> Option<impl Fn(&str) -> Result<Validation, inquire::CustomUserError> + Clone + use<>> {
    unsafe {
        let pattern = optional_text(pattern)?;
        let regex = regex::Regex::new(&pattern).unwrap_or_else(|_| {
            cli_panic(format!(
                "error: `{prompt}` pattern is not a valid regular expression"
            ))
        });
        let message =
            optional_text(pattern_message).unwrap_or_else(|| "invalid format".to_string());
        Some(move |input: &str| {
            if regex.is_match(input) {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid(message.clone().into()))
            }
        })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __tyrus_cli_input(options: *const InputOpts) -> TyrusString {
    unsafe {
        require_tty("input");
        let options = &*options;
        let message = required_text(options.message.borrow(), "input");
        let default = optional_text(options.default);
        let placeholder = optional_text(options.placeholder);
        let help = optional_text(options.help);
        let min_length = read_opt(options.min_length).map(|value| value.max(0) as usize);
        let max_length = read_opt(options.max_length).map(|value| value.max(0) as usize);

        let mut prompt = Text::new(&message);
        if let Some(default) = default.as_deref() {
            prompt = prompt.with_default(default);
        }
        if let Some(placeholder) = placeholder.as_deref() {
            prompt = prompt.with_placeholder(placeholder);
        }
        if let Some(help) = help.as_deref() {
            prompt = prompt.with_help_message(help);
        }
        if let Some(min) = min_length {
            prompt = prompt.with_validator(move |input: &str| {
                if input.chars().count() < min {
                    Ok(Validation::Invalid(
                        format!("must be at least {min} characters").into(),
                    ))
                } else {
                    Ok(Validation::Valid)
                }
            });
        }
        if let Some(max) = max_length {
            prompt = prompt.with_validator(move |input: &str| {
                if input.chars().count() > max {
                    Ok(Validation::Invalid(
                        format!("must be at most {max} characters").into(),
                    ))
                } else {
                    Ok(Validation::Valid)
                }
            });
        }
        if let Some(validator) =
            pattern_validator(options.pattern, options.pattern_message, "input")
        {
            prompt = prompt.with_validator(validator);
        }
        alloc(&finish(prompt.prompt(), "input"))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __tyrus_cli_password(options: *const PasswordOpts) -> TyrusString {
    unsafe {
        require_tty("password");
        let options = &*options;
        let message = required_text(options.message.borrow(), "password");
        let help = optional_text(options.help);
        let confirm = read_opt(options.confirm).unwrap_or(false);
        let confirm_message = optional_text(options.confirm_message);

        let mut prompt = Password::new(&message).with_display_mode(PasswordDisplayMode::Masked);
        if confirm {
            if let Some(confirm_message) = confirm_message.as_deref() {
                prompt = prompt.with_custom_confirmation_message(confirm_message);
            }
        } else {
            prompt = prompt.without_confirmation();
        }
        if let Some(help) = help.as_deref() {
            prompt = prompt.with_help_message(help);
        }
        if let Some(validator) =
            pattern_validator(options.pattern, options.pattern_message, "password")
        {
            prompt = prompt.with_validator(validator);
        }
        alloc(&finish(prompt.prompt(), "password"))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __tyrus_cli_confirm(options: *const ConfirmOpts) -> bool {
    unsafe {
        require_tty("confirm");
        let options = &*options;
        let message = required_text(options.message.borrow(), "confirm");
        let default = read_opt(options.default).unwrap_or(true);
        let help = optional_text(options.help);
        let yes = optional_text(options.yes).unwrap_or_else(|| "Yes".to_string());
        let no = optional_text(options.no).unwrap_or_else(|| "No".to_string());

        let choices = vec![yes.clone(), no];
        let mut prompt =
            Select::new(&message, choices).with_starting_cursor(if default { 0 } else { 1 });
        if let Some(help) = help.as_deref() {
            prompt = prompt.with_help_message(help);
        }
        finish(prompt.prompt(), "confirm") == yes
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __tyrus_cli_select(options: *const SelectOpts) -> TyrusString {
    unsafe {
        require_tty("select");
        let options = &*options;
        let message = required_text(options.message.borrow(), "select");
        let items = choices(&options.choices);
        let default = optional_text(options.default);
        let help = optional_text(options.help);
        let page_size = read_opt(options.page_size).map(|value| value.max(1) as usize);

        let mut prompt = Select::new(&message, items.clone());
        if let Some(default) = default.as_deref()
            && let Some(index) = items.iter().position(|item| item == default)
        {
            prompt = prompt.with_starting_cursor(index);
        }
        if let Some(page_size) = page_size {
            prompt = prompt.with_page_size(page_size);
        }
        if let Some(help) = help.as_deref() {
            prompt = prompt.with_help_message(help);
        }
        alloc(&finish(prompt.prompt(), "select"))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __tyrus_cli_multiselect(
    options: *const MultiSelectOpts,
    out: *mut TyrusArray,
) {
    unsafe {
        require_tty("multiselect");
        let options = &*options;
        let message = required_text(options.message.borrow(), "multiselect");
        let items = choices(&options.choices);
        let help = optional_text(options.help);
        let page_size = read_opt(options.page_size).map(|value| value.max(1) as usize);
        let required = read_opt(options.required).unwrap_or(false);

        let mut prompt = MultiSelect::new(&message, items);
        if let Some(page_size) = page_size {
            prompt = prompt.with_page_size(page_size);
        }
        if let Some(help) = help.as_deref() {
            prompt = prompt.with_help_message(help);
        }
        if required {
            prompt = prompt.with_validator(
                |selected: &[inquire::list_option::ListOption<&String>]| {
                    if selected.is_empty() {
                        Ok(Validation::Invalid("select at least one option".into()))
                    } else {
                        Ok(Validation::Valid)
                    }
                },
            );
        }
        let selected = finish(prompt.prompt(), "multiselect");

        let mut data: Vec<TyrusString> = selected.iter().map(|value| alloc(value)).collect();
        let pointer = data.as_mut_ptr();
        let length = data.len();
        let capacity = data.capacity();
        std::mem::forget(data);
        if !out.is_null() {
            (*out).data = pointer as *mut u8;
            (*out).length = length;
            (*out).capacity = capacity;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __tyrus_cli_number(options: *const NumberOpts) -> f64 {
    unsafe {
        require_tty("number");
        let options = &*options;
        let message = required_text(options.message.borrow(), "number");
        let default = read_opt(options.default);
        let min = read_opt(options.min);
        let max = read_opt(options.max);
        let help = optional_text(options.help);

        let mut prompt =
            CustomType::<f64>::new(&message).with_error_message("please enter a valid number");
        if let Some(default) = default {
            prompt = prompt.with_default(default);
        }
        if let Some(help) = help.as_deref() {
            prompt = prompt.with_help_message(help);
        }
        if min.is_some() || max.is_some() {
            prompt = prompt.with_validator(move |value: &f64| {
                if let Some(min) = min
                    && *value < min
                {
                    return Ok(Validation::Invalid(
                        format!("must be at least {min}").into(),
                    ));
                }
                if let Some(max) = max
                    && *value > max
                {
                    return Ok(Validation::Invalid(format!("must be at most {max}").into()));
                }
                Ok(Validation::Valid)
            });
        }
        finish(prompt.prompt(), "number")
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn __tyrus_cli_integer(options: *const IntegerOpts) -> i64 {
    unsafe {
        require_tty("integer");
        let options = &*options;
        let message = required_text(options.message.borrow(), "integer");
        let default = read_opt(options.default);
        let min = read_opt(options.min);
        let max = read_opt(options.max);
        let help = optional_text(options.help);

        let mut prompt =
            CustomType::<i64>::new(&message).with_error_message("please enter a whole number");
        if let Some(default) = default {
            prompt = prompt.with_default(default);
        }
        if let Some(help) = help.as_deref() {
            prompt = prompt.with_help_message(help);
        }
        if min.is_some() || max.is_some() {
            prompt = prompt.with_validator(move |value: &i64| {
                if let Some(min) = min
                    && *value < min
                {
                    return Ok(Validation::Invalid(
                        format!("must be at least {min}").into(),
                    ));
                }
                if let Some(max) = max
                    && *value > max
                {
                    return Ok(Validation::Invalid(format!("must be at most {max}").into()));
                }
                Ok(Validation::Valid)
            });
        }
        finish(prompt.prompt(), "integer")
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn __tyrus_cli_is_tty() -> bool {
    std::io::stdout().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn optional_text_dereferences_boxed_strings() {
        let owned = TyrusString::literal("Confirm password");
        let inner: TyrusString = owned.borrow();
        let boxed: OptStr = &inner as *const TyrusString;
        assert_eq!(
            unsafe { optional_text(boxed) },
            Some("Confirm password".to_string())
        );
        assert_eq!(unsafe { optional_text(std::ptr::null()) }, None);
    }
}
