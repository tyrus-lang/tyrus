use super::panic::abort_with;
use std::cell::RefCell;
use std::ptr;
use tyrus_string::TyrusString;

thread_local! {
    static VALIDATION_ISSUES: RefCell<Vec<TyrusValidationIssue>> = const { RefCell::new(Vec::new()) };
}

#[repr(C)]
#[derive(Debug)]
pub struct TyrusValidationIssue {
    pub target: TyrusString,
    pub validator: TyrusString,
    pub message: TyrusString,
}

#[repr(C)]
#[derive(Debug)]
pub struct TyrusValidationError {
    pub message: TyrusString,
    pub issues: *const TyrusValidationIssue,
    pub issue_count: usize,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_validation_error_has_issues(
    error: *const TyrusValidationError,
) -> bool {
    unsafe { !error.is_null() && (*error).issue_count > 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_validation_error_describe(
    message: TyrusString,
    issues: *const TyrusValidationIssue,
    issue_count: usize,
) -> TyrusString {
    unsafe {
        let fallback = || crate::strings::alloc_string("ValidationError");
        let read = |text: &TyrusString| -> Option<String> {
            (!text.is_null()).then(|| { text.as_str() }.to_owned())
        };
        let issues = if issues.is_null() {
            &[][..]
        } else {
            std::slice::from_raw_parts(issues, issue_count)
        };
        let Some(first) = issues.first() else {
            let Some(message) = read(&message) else {
                return fallback();
            };
            let allocated = crate::strings::alloc_string(&format!("ValidationError: {message}"));
            return if allocated.is_null() {
                fallback()
            } else {
                allocated
            };
        };
        let validator = read(&first.validator).unwrap_or_else(|| "validator".to_string());
        let target = read(&first.target).unwrap_or_else(|| "value".to_string());
        let message = read(&first.message).unwrap_or_else(|| "value is invalid".to_string());
        let mut text = format!("ValidationError: `{validator}` failed for `{target}`: {message}");
        if issues.len() > 1 {
            let rest = issues.len() - 1;
            let plural = if rest == 1 { "issue" } else { "issues" };
            text.push_str(&format!(" (and {rest} more {plural})"));
        }
        let allocated = crate::strings::alloc_string(&text);
        if allocated.is_null() {
            fallback()
        } else {
            allocated
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_validation_format_issue(
    target: TyrusString,
    validator: TyrusString,
    message: TyrusString,
) -> TyrusString {
    let text = |value: &TyrusString, fallback: &str| {
        if value.is_null() {
            fallback.to_string()
        } else {
            unsafe { value.as_str().to_string() }
        }
    };
    let validator = text(&validator, "validator");
    let target = text(&target, "value");
    let message = text(&message, "value is invalid");
    let formatted =
        format!("tyrus validation error: `{validator}` failed for `{target}`: {message}");
    crate::strings::alloc_string(&formatted)
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_validation_issues_begin() {
    VALIDATION_ISSUES.with(|issues| issues.borrow_mut().clear());
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_validation_issue_add(
    target: TyrusString,
    validator: TyrusString,
    message: TyrusString,
) {
    VALIDATION_ISSUES.with(|issues| {
        issues.borrow_mut().push(TyrusValidationIssue {
            target,
            validator,
            message,
        });
    });
}

#[allow(dead_code)]
pub(crate) fn take_validation_issues() -> Vec<(String, String, String)> {
    VALIDATION_ISSUES.with(|issues| {
        std::mem::take(&mut *issues.borrow_mut())
            .into_iter()
            .filter_map(|issue| {
                let target = crate::types::text(&issue.target).ok()?;
                let validator = crate::types::text(&issue.validator).ok()?;
                let message = crate::types::text(&issue.message).ok()?;
                Some((target, validator, message))
            })
            .collect()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_validation_issues_count() -> usize {
    VALIDATION_ISSUES.with(|issues| issues.borrow().len())
}

#[unsafe(no_mangle)]
pub extern "C" fn tyrus_validation_issues_take() -> *mut TyrusValidationIssue {
    VALIDATION_ISSUES.with(|issues| {
        let values = std::mem::take(&mut *issues.borrow_mut());
        if values.is_empty() {
            return ptr::null_mut();
        }
        Box::into_raw(values.into_boxed_slice()).cast::<TyrusValidationIssue>()
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_validation_issues_free(
    issues: *mut TyrusValidationIssue,
    count: usize,
) {
    unsafe {
        if issues.is_null() || count == 0 {
            return;
        }
        let mut values = Box::from_raw(ptr::slice_from_raw_parts_mut(issues, count));
        for issue in values.iter_mut() {
            {
                std::mem::replace(&mut issue.target, TyrusString::null()).release();
                std::mem::replace(&mut issue.validator, TyrusString::null()).release();
                std::mem::replace(&mut issue.message, TyrusString::null()).release();
            }
        }
        drop(values);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tyrus_panic_validation(message: TyrusString) -> ! {
    unsafe { abort_with(message, "tyrus validation error") }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::ptr;

    #[test]
    fn detects_structured_validation_issues() {
        let error = TyrusValidationError {
            message: TyrusString::null(),
            issues: ptr::null(),
            issue_count: 1,
        };

        assert!(unsafe { tyrus_validation_error_has_issues(&error) });
        assert!(!unsafe { tyrus_validation_error_has_issues(ptr::null()) });
    }

    #[test]
    fn an_uncaught_validation_error_names_the_field_and_the_rule() {
        let target = TyrusString::literal("age");
        let validator = TyrusString::literal("@min(18)");
        let message = TyrusString::literal("value must be at least 18");
        let issue = TyrusValidationIssue {
            target: target.borrow(),
            validator: validator.borrow(),
            message: message.borrow(),
        };

        let described = unsafe { tyrus_validation_error_describe(TyrusString::null(), &issue, 1) };
        let text = unsafe { described.as_str() };
        assert_eq!(
            text,
            "ValidationError: `@min(18)` failed for `age`: value must be at least 18"
        );
        unsafe { described.release() };
    }

    #[test]
    fn an_uncaught_validation_error_counts_the_issues_it_does_not_print() {
        let target = TyrusString::literal("username");
        let validator = TyrusString::literal("@minLength(3)");
        let message = TyrusString::literal("value must be at least 3 characters");
        let issue = || TyrusValidationIssue {
            target: target.borrow(),
            validator: validator.borrow(),
            message: message.borrow(),
        };
        let issues = [issue(), issue(), issue()];

        let described =
            unsafe { tyrus_validation_error_describe(TyrusString::null(), issues.as_ptr(), 3) };
        let text = unsafe { described.as_str() };
        assert_eq!(
            text,
            "ValidationError: `@minLength(3)` failed for `username`: value must be at least 3 characters (and 2 more issues)"
        );
        unsafe { described.release() };
    }

    #[test]
    fn collects_transfers_and_releases_multiple_issues() {
        tyrus_validation_issues_begin();
        tyrus_validation_issue_add(
            TyrusString::null(),
            TyrusString::null(),
            TyrusString::null(),
        );
        tyrus_validation_issue_add(
            TyrusString::null(),
            TyrusString::null(),
            TyrusString::null(),
        );
        assert_eq!(tyrus_validation_issues_count(), 2);
        let issues = tyrus_validation_issues_take();
        assert!(!issues.is_null());
        assert_eq!(tyrus_validation_issues_count(), 0);
        unsafe { tyrus_validation_issues_free(issues, 2) };
    }
}
