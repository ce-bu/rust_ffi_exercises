//! # Exercise 04: C-String Passing and Receiving
//!
//! **Concept:** Strings in C are null-terminated `char*` pointers.
//! Rust has two dedicated types for FFI string work:
//!
//! | Type      | Owned? | Analogous to    | Use when …                          |
//! |-----------|--------|-----------------|-------------------------------------|
//! | `CStr`    | No     | `&str`          | **borrowing** a C string            |
//! | `CString` | Yes    | `String`        | **creating** a string to hand to C  |
//!
//! Key functions:
//! - `CStr::from_ptr(ptr)` — borrow a `*const c_char` (unsafe).
//! - `CStr::to_str()` → `Result<&str, Utf8Error>`.
//! - `CString::new(s)` → `Result<CString, NulError>`.
//! - `CString::into_raw()` → `*mut c_char` (caller must free).
//! - `CString::from_raw(ptr)` — reclaim ownership for deallocation.
//!
//! ## Your task
//!
//! Implement the five functions below.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex04
//! ```

use std::ffi::{c_char, CStr, CString};

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Compute the length (in bytes, excluding the null terminator) of
// a C string.  Return 0 if `s` is null.
//
// Hint: CStr::from_ptr(s).to_bytes().len()

/// # Safety
/// If non-null, `s` must point to a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ffi_string_length(s: *const c_char) -> usize {
    todo!("Return the byte-length of the C string (0 if null)")
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Build the greeting `"Hello, {name}!"` and return it as a
// heap-allocated C string.  The caller is responsible for freeing
// it with `ffi_free_string`.
//
// Return null if `name` is null.
//
// Hint: CString::new(format!(...)).unwrap().into_raw()

/// # Safety
/// If non-null, `name` must point to a valid null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn ffi_hello(name: *const c_char) -> *mut c_char {
    todo!("Allocate and return a greeting string, or null")
}

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Free a string previously returned by `ffi_hello` (or any
// Rust-allocated `CString::into_raw()` string).
//
// Must be a no-op if `s` is null.
//
// Hint: CString::from_raw(s) — dropping it deallocates.

/// # Safety
/// `s` must have been allocated by `CString::into_raw`, or be null.
#[no_mangle]
pub unsafe extern "C" fn ffi_free_string(s: *mut c_char) {
    todo!("Reclaim and drop the CString (no-op if null)")
}

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Convert `input` to uppercase and write the result (including a
// null terminator) into the caller-provided buffer `buf` of size
// `buf_len`.
//
// Return value:
//   ≥ 0  →  number of bytes written (excluding null terminator)
//   -1   →  buffer too small
//   -1   →  input is null
//
// Hint: after uppercasing, check that `result.len() + 1 <= buf_len`,
//       then use `std::ptr::copy_nonoverlapping`.

/// # Safety
/// - `input` must be a valid C string (or null).
/// - `buf` must point to at least `buf_len` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn ffi_to_uppercase(
    input: *const c_char,
    buf: *mut c_char,
    buf_len: usize,
) -> isize {
    todo!("Uppercase the input and write into buf; return length or -1")
}

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Concatenate two C strings with a separator between them.
// Return a new heap-allocated C string.  Caller frees with
// `ffi_free_string`.
//
// Return null if either pointer is null.

/// # Safety
/// Both `a` and `b` must be valid C strings (or null).
#[no_mangle]
pub unsafe extern "C" fn ffi_concat(
    a: *const c_char,
    b: *const c_char,
    sep: *const c_char,
) -> *mut c_char {
    todo!("Concatenate a + sep + b into a new CString and return its raw pointer")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::ptr;

    #[test]
    fn test_ex04_string_length() {
        let s = CString::new("hello").unwrap();
        let len = unsafe { ffi_string_length(s.as_ptr()) };
        assert_eq!(len, 5);
    }

    #[test]
    fn test_ex04_string_length_null() {
        let len = unsafe { ffi_string_length(ptr::null()) };
        assert_eq!(len, 0);
    }

    #[test]
    fn test_ex04_hello() {
        let name = CString::new("Rustacean").unwrap();
        let greeting = unsafe { ffi_hello(name.as_ptr()) };
        assert!(!greeting.is_null());
        let result = unsafe { CStr::from_ptr(greeting) }.to_str().unwrap();
        assert_eq!(result, "Hello, Rustacean!");
        unsafe { ffi_free_string(greeting) };
    }

    #[test]
    fn test_ex04_hello_null() {
        let greeting = unsafe { ffi_hello(ptr::null()) };
        assert!(greeting.is_null());
    }

    #[test]
    fn test_ex04_to_uppercase() {
        let input = CString::new("hello").unwrap();
        let mut buf = vec![0i8; 64];
        let n = unsafe {
            ffi_to_uppercase(input.as_ptr(), buf.as_mut_ptr(), buf.len())
        };
        assert_eq!(n, 5);
        let result = unsafe { CStr::from_ptr(buf.as_ptr()) }
            .to_str()
            .unwrap();
        assert_eq!(result, "HELLO");
    }

    #[test]
    fn test_ex04_to_uppercase_too_small() {
        let input = CString::new("hello").unwrap();
        let mut buf = vec![0i8; 3]; // too small for "HELLO\0"
        let n = unsafe {
            ffi_to_uppercase(input.as_ptr(), buf.as_mut_ptr(), buf.len())
        };
        assert_eq!(n, -1);
    }

    #[test]
    fn test_ex04_concat() {
        let a = CString::new("hello").unwrap();
        let b = CString::new("world").unwrap();
        let sep = CString::new(", ").unwrap();
        let result = unsafe { ffi_concat(a.as_ptr(), b.as_ptr(), sep.as_ptr()) };
        assert!(!result.is_null());
        let s = unsafe { CStr::from_ptr(result) }.to_str().unwrap();
        assert_eq!(s, "hello, world");
        unsafe { ffi_free_string(result) };
    }

    #[test]
    fn test_ex04_concat_null() {
        let a = CString::new("hello").unwrap();
        let result = unsafe { ffi_concat(a.as_ptr(), ptr::null(), ptr::null()) };
        assert!(result.is_null());
    }
}
