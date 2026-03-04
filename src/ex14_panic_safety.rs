//! # Exercise 14: Panic Safety at FFI Boundaries
//!
//! **Concept:** Unwinding a Rust panic across an `extern "C"`
//! boundary is **undefined behavior**.  Every `extern "C"` function
//! that could panic must catch the panic before it escapes.
//!
//! The standard tool is `std::panic::catch_unwind`, which captures
//! a panic and returns `Result<T, Box<dyn Any>>`.
//!
//! ## Pattern
//!
//! ```rust,ignore
//! #[no_mangle]
//! pub extern "C" fn my_func(x: i32) -> i32 {
//!     match std::panic::catch_unwind(|| {
//!         // code that might panic
//!     }) {
//!         Ok(val)  => val,
//!         Err(_)   => -1,   // return a sentinel error value
//!     }
//! }
//! ```
//!
//! ## Your task
//!
//! 1. Implement `safe_divide` with panic catching.
//! 2. Implement `safe_parse_json_len` with panic catching.
//! 3. **Bonus:** Write a helper macro that wraps any expression
//!    in `catch_unwind` and returns an error code on panic.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex14
//! ```

use std::panic;
use std::ffi::{c_char, CStr};

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Divide `a / b`, catching any panic (e.g. from overflow or
// logic errors you might accidentally introduce).
//
// Returns:
//   0  + writes result to *out  → success
//  -1  (out is null)            → error
//  -2  (b is zero)              → error
//  -3  (panic caught)           → error
//
// Steps:
//   1. Check `out` is non-null.
//   2. Wrap the division in `panic::catch_unwind(|| a / b)`.
//   3. On Ok(val) → write to *out, return 0.
//   4. On Err(_)  → return -3.

/// # Safety
/// `out` must be a valid pointer or null.
#[no_mangle]
pub unsafe extern "C" fn safe_divide(a: i32, b: i32, out: *mut i32) -> i32 {
    todo!("Check null, catch_unwind the division, write result or return error code")
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Count the number of top-level keys in a JSON-like string.
// (Simple approach: just count `:` characters at depth 1.)
//
// This function might panic if the input contains unexpected data.
// Wrap everything in catch_unwind.
//
// Returns:
//  ≥ 0  number of keys found
//  -1   null input
//  -2   invalid UTF-8
//  -3   panic caught

/// # Safety
/// `json` must be a valid C string or null.
#[no_mangle]
pub unsafe extern "C" fn safe_count_colons(json: *const c_char) -> i32 {
    todo!(
        "catch_unwind: CStr::from_ptr, to_str, count ':' chars. \
         Return count on success, negative on error."
    )
}

// ── TODO 3 (Bonus) ─────────────────────────────────────────────
//
// Write a macro `ffi_catch!` that wraps an expression in
// catch_unwind and returns an error code on panic.
//
// Example usage:
//
//   ffi_catch! {
//       let result = some_risky_computation();
//       *out = result;
//       0  // success return value
//   } or_panic { -99 }
//
// If you prefer, implement it as a function:
//   fn ffi_catch<F: FnOnce() -> i32 + panic::UnwindSafe>(f: F, panic_code: i32) -> i32
//
// Either approach teaches the same concept.

/// Helper that runs `f`, returning `panic_code` if it panics.
pub fn ffi_catch<F>(f: F, panic_code: i32) -> i32
where
    F: FnOnce() -> i32 + panic::UnwindSafe,
{
    todo!("panic::catch_unwind(f).unwrap_or(panic_code)")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_ex14_safe_divide_ok() {
        let mut result: i32 = 0;
        let code = unsafe { safe_divide(10, 3, &mut result) };
        assert_eq!(code, 0);
        assert_eq!(result, 3);
    }

    #[test]
    fn test_ex14_safe_divide_zero() {
        let mut result: i32 = 0;
        let code = unsafe { safe_divide(10, 0, &mut result) };
        assert_eq!(code, -2);
    }

    #[test]
    fn test_ex14_safe_divide_null() {
        let code = unsafe { safe_divide(10, 3, std::ptr::null_mut()) };
        assert_eq!(code, -1);
    }

    #[test]
    fn test_ex14_safe_divide_overflow() {
        // i32::MIN / -1 panics in debug mode (overflow).
        let mut result: i32 = 0;
        let code = unsafe { safe_divide(i32::MIN, -1, &mut result) };
        // Should NOT crash — should return -3 (panic caught) or a valid result.
        assert!(code == 0 || code == -3);
    }

    #[test]
    fn test_ex14_count_colons_ok() {
        let json = CString::new(r#"{"a": 1, "b": 2, "c": 3}"#).unwrap();
        let count = unsafe { safe_count_colons(json.as_ptr()) };
        assert_eq!(count, 3);
    }

    #[test]
    fn test_ex14_count_colons_null() {
        let count = unsafe { safe_count_colons(std::ptr::null()) };
        assert_eq!(count, -1);
    }

    #[test]
    fn test_ex14_count_colons_empty() {
        let json = CString::new("").unwrap();
        let count = unsafe { safe_count_colons(json.as_ptr()) };
        assert_eq!(count, 0);
    }

    #[test]
    fn test_ex14_ffi_catch_ok() {
        let code = ffi_catch(|| 42, -99);
        assert_eq!(code, 42);
    }

    #[test]
    fn test_ex14_ffi_catch_panic() {
        let code = ffi_catch(|| panic!("boom"), -99);
        assert_eq!(code, -99);
    }
}
