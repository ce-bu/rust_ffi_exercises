//! # Exercise 09: Error Handling Across FFI
//!
//! **Concept:** Rust's `Result<T, E>` cannot cross an FFI boundary.
//! The standard C pattern is:
//!
//! 1. Return an **integer error code** (0 = success).
//! 2. Store a human-readable message in **thread-local** storage.
//! 3. Provide `ffi_last_error()` so the caller can retrieve it.
//!
//! This is the same pattern used by `errno`, `GetLastError()`, and
//! `dlerror()`.
//!
//! ## Your task
//!
//! 1. Define error-code constants.
//! 2. Set up `thread_local!` storage for the last error message.
//! 3. Implement the public API functions.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex09
//! ```

use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Define these public constants:
//   FFI_OK           =  0
//   FFI_ERR_NULL     = -1
//   FFI_ERR_UTF8     = -2
//   FFI_ERR_OVERFLOW = -3
//   FFI_ERR_UNKNOWN  = -99

// pub const FFI_OK: i32 = ...;
// pub const FFI_ERR_NULL: i32 = ...;
// ...

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Create thread-local storage for the last error message.
//
// Hint:
//   thread_local! {
//       static LAST_ERROR: RefCell<Option<CString>> = RefCell::new(None);
//   }
//
// Then write two internal helpers:
//   fn set_last_error(msg: &str)   — stores a CString in the cell
//   fn clear_last_error()          — sets the cell to None

// thread_local! { ... }
// fn set_last_error(msg: &str) { ... }
// fn clear_last_error() { ... }

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Return a pointer to the last error message (valid until the next
// FFI call on this thread), or null if there is no error.
//
// Hint: LAST_ERROR.with(|cell| { ... cell.borrow().as_ref()?.as_ptr() ... })
//       Be careful: the borrow must not be dropped while you still
//       use the pointer.  Use `with_borrow` or return a raw pointer
//       inside the `with` closure.

/// Returns a pointer to the last error message, or null.
/// The returned pointer is valid until the next FFI call.
#[no_mangle]
pub extern "C" fn ffi_last_error() -> *const c_char {
    todo!("Read LAST_ERROR thread-local and return the CString pointer")
}

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Clear the last error.

#[no_mangle]
pub extern "C" fn ffi_clear_error() {
    todo!("Call clear_last_error()")
}

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Safe integer division.
//
// - If `b == 0`, set last error to "division by zero" and return
//   `FFI_ERR_OVERFLOW`.
// - If `out` is null, set last error and return `FFI_ERR_NULL`.
// - Otherwise write `a / b` to `*out` and return `FFI_OK`.

/// # Safety
/// `out` must be a valid pointer (or null, which is reported as an error).
#[no_mangle]
pub unsafe extern "C" fn ffi_divide(a: i32, b: i32, out: *mut i32) -> i32 {
    todo!("Validate inputs, write result, return error code")
}

// ── TODO 6 ─────────────────────────────────────────────────────
//
// Parse a C string into an `i64`.
//
// - Null pointer → FFI_ERR_NULL + set error.
// - Invalid UTF-8 → FFI_ERR_UTF8 + set error.
// - Parse failure → FFI_ERR_UNKNOWN + set error with the parse message.
// - Success → write parsed value to `*out`, return FFI_OK.

/// # Safety
/// - `input` must be a valid C string or null.
/// - `out` must be a valid writable pointer or null.
#[no_mangle]
pub unsafe extern "C" fn ffi_parse_int(
    input: *const c_char,
    out: *mut i64,
) -> i32 {
    todo!("Parse the string, handle errors with set_last_error")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CStr;

    #[test]
    fn test_ex09_divide_ok() {
        let mut result: i32 = 0;
        let code = unsafe { ffi_divide(10, 3, &mut result) };
        assert_eq!(code, 0);   // FFI_OK
        assert_eq!(result, 3); // integer division
    }

    #[test]
    fn test_ex09_divide_by_zero() {
        let mut result: i32 = 0;
        let code = unsafe { ffi_divide(10, 0, &mut result) };
        assert!(code < 0, "expected error code, got {code}");

        let msg = ffi_last_error();
        assert!(!msg.is_null());
        let s = unsafe { CStr::from_ptr(msg) }.to_str().unwrap();
        assert!(
            s.contains("zero") || s.contains("Zero"),
            "error message should mention zero, got: {s}"
        );
    }

    #[test]
    fn test_ex09_divide_null_out() {
        let code = unsafe { ffi_divide(1, 1, std::ptr::null_mut()) };
        assert!(code < 0);
    }

    #[test]
    fn test_ex09_parse_int_ok() {
        let input = CString::new("42").unwrap();
        let mut val: i64 = 0;
        let code = unsafe { ffi_parse_int(input.as_ptr(), &mut val) };
        assert_eq!(code, 0);
        assert_eq!(val, 42);
    }

    #[test]
    fn test_ex09_parse_int_negative() {
        let input = CString::new("-7").unwrap();
        let mut val: i64 = 0;
        let code = unsafe { ffi_parse_int(input.as_ptr(), &mut val) };
        assert_eq!(code, 0);
        assert_eq!(val, -7);
    }

    #[test]
    fn test_ex09_parse_int_invalid() {
        let input = CString::new("not_a_number").unwrap();
        let mut val: i64 = 0;
        let code = unsafe { ffi_parse_int(input.as_ptr(), &mut val) };
        assert!(code < 0);

        let msg = ffi_last_error();
        assert!(!msg.is_null());
    }

    #[test]
    fn test_ex09_parse_int_null() {
        let mut val: i64 = 0;
        let code = unsafe { ffi_parse_int(std::ptr::null(), &mut val) };
        assert!(code < 0);
    }

    #[test]
    fn test_ex09_clear_error() {
        // Force an error
        let code = unsafe { ffi_divide(1, 0, &mut 0) };
        assert!(code < 0);
        assert!(!ffi_last_error().is_null());

        // Clear it
        ffi_clear_error();
        assert!(ffi_last_error().is_null());
    }
}
