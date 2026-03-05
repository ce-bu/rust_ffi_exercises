// src/ex26_cpp_exceptions.rs
//
// Safe Rust wrappers for C++ functions that may throw exceptions.
//
// The C++ side (csrc/ex26_cpp_exceptions.cpp) wraps every extern "C"
// function in try/catch and stores exception info in a thread-local.
// On the Rust side we:
//   1. Call the extern "C" function and check the return code.
//   2. If non-zero, retrieve the exception info → CppException.
//   3. Return Result<T, CppException>.
//
// This is the canonical pattern for turning C++ exceptions into Rust
// Result values.

use std::ffi::{c_char, c_void, CStr, CString};
use std::fmt;

/* ══════════════════════════════════════════════════════════════
 * Error codes (must match the C++ defines)
 * ══════════════════════════════════════════════════════════════ */

pub const CPP_EX_OK: i32 = 0;
pub const CPP_EX_ERR_DOMAIN: i32 = -1;
pub const CPP_EX_ERR_INVALID: i32 = -2;
pub const CPP_EX_ERR_CUSTOM: i32 = -3;
pub const CPP_EX_ERR_UNKNOWN: i32 = -99;

/* ══════════════════════════════════════════════════════════════
 * extern "C" declarations
 * ══════════════════════════════════════════════════════════════ */

type CppExMapFn = extern "C" fn(f64, *mut f64, *mut c_void) -> i32;

extern "C" {
    fn cpp_ex_get_error(
        out_msg: *mut c_char,
        msg_len: usize,
        out_type: *mut c_char,
        type_len: usize,
        out_code: *mut i32,
    ) -> i32;
    fn cpp_ex_clear_error();

    fn cpp_ex_divide(a: f64, b: f64, out: *mut f64) -> i32;
    fn cpp_ex_parse_int(s: *const c_char, len: usize, out: *mut i64) -> i32;
    fn cpp_ex_sqrt(x: f64, out: *mut f64) -> i32;
    fn cpp_ex_process_data(data: *const u8, len: usize, out_checksum: *mut i32) -> i32;
    fn cpp_ex_trigger_unknown() -> i32;
    fn cpp_ex_map_array(
        input: *const f64,
        output: *mut f64,
        len: usize,
        map_fn: CppExMapFn,
        ctx: *mut c_void,
    ) -> i32;
}

/* ══════════════════════════════════════════════════════════════
 * CppException — Rust error type carrying C++ exception info
 * ══════════════════════════════════════════════════════════════ */

#[derive(Debug, Clone)]
pub struct CppException {
    pub message: String,
    pub type_name: String,
    pub code: i32,
}

impl fmt::Display for CppException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "C++ exception [{}] (code {}): {}",
            self.type_name, self.code, self.message
        )
    }
}

impl std::error::Error for CppException {}

/* ══════════════════════════════════════════════════════════════
 * Helpers
 * ══════════════════════════════════════════════════════════════ */

/// Retrieve the last C++ exception from thread-local storage.
pub fn get_last_cpp_error(fallback_code: i32) -> CppException {
    let mut msg_buf = [0u8; 512];
    let mut type_buf = [0u8; 128];
    let mut code: i32 = fallback_code;
    unsafe {
        cpp_ex_get_error(
            msg_buf.as_mut_ptr() as *mut c_char,
            msg_buf.len(),
            type_buf.as_mut_ptr() as *mut c_char,
            type_buf.len(),
            &mut code,
        );
    }
    let message = unsafe { CStr::from_ptr(msg_buf.as_ptr() as *const c_char) }
        .to_string_lossy()
        .into_owned();
    let type_name = unsafe { CStr::from_ptr(type_buf.as_ptr() as *const c_char) }
        .to_string_lossy()
        .into_owned();
    CppException {
        message,
        type_name,
        code,
    }
}

/// Check a C++ return code — Ok(()) or Err(CppException).
fn check(rc: i32) -> Result<(), CppException> {
    if rc == CPP_EX_OK {
        Ok(())
    } else {
        Err(get_last_cpp_error(rc))
    }
}

/// Clear the thread-local exception state.
pub fn clear_error() {
    unsafe { cpp_ex_clear_error() }
}

/* ══════════════════════════════════════════════════════════════
 * Safe public wrappers
 * ══════════════════════════════════════════════════════════════ */

/// Divide `a / b`.  Returns `Err` if `b == 0.0` (domain error).
pub fn divide(a: f64, b: f64) -> Result<f64, CppException> {
    let mut result = 0.0f64;
    let rc = unsafe { cpp_ex_divide(a, b, &mut result) };
    check(rc)?;
    Ok(result)
}

/// Parse a decimal integer from a string.
pub fn parse_int(s: &str) -> Result<i64, CppException> {
    let mut result = 0i64;
    let rc = unsafe { cpp_ex_parse_int(s.as_ptr() as *const c_char, s.len(), &mut result) };
    check(rc)?;
    Ok(result)
}

/// Square root.  Returns `Err` for negative input (domain error).
pub fn sqrt_checked(x: f64) -> Result<f64, CppException> {
    let mut result = 0.0f64;
    let rc = unsafe { cpp_ex_sqrt(x, &mut result) };
    check(rc)?;
    Ok(result)
}

/// Process a data buffer.  Returns a checksum on success.
///
/// Errors:
/// - null pointer  → `ERR_INVALID`
/// - empty data    → `ERR_CUSTOM` (ProcessingError)
/// - data > 4096   → `ERR_CUSTOM`
/// - byte 0xFF     → `ERR_CUSTOM`
pub fn process_data(data: &[u8]) -> Result<i32, CppException> {
    let mut checksum = 0i32;
    let rc = unsafe { cpp_ex_process_data(data.as_ptr(), data.len(), &mut checksum) };
    check(rc)?;
    Ok(checksum)
}

/// Trigger a non-std::exception throw (integer).
pub fn trigger_unknown() -> Result<(), CppException> {
    let rc = unsafe { cpp_ex_trigger_unknown() };
    check(rc)
}

/* ══════════════════════════════════════════════════════════════
 * Callback-based API: map an array through a Rust closure
 * ══════════════════════════════════════════════════════════════ */

/// Apply a transformation to each element of `input`, writing
/// results to a new `Vec<f64>`.
///
/// The closure returns `Ok(value)` for the transformed value or
/// `Err(code)` to signal an error code (will appear as a
/// `CppException` with type "callback_error").
pub fn map_array<F>(input: &[f64], mut f: F) -> Result<Vec<f64>, CppException>
where
    F: FnMut(f64) -> Result<f64, i32>,
{
    let mut output = vec![0.0f64; input.len()];

    // Trampoline: calls the closure through a type-erased pointer.
    extern "C" fn trampoline<F2>(
        val: f64,
        out: *mut f64,
        ctx: *mut c_void,
    ) -> i32
    where
        F2: FnMut(f64) -> Result<f64, i32>,
    {
        let closure = unsafe { &mut *(ctx as *mut F2) };
        match closure(val) {
            Ok(v) => {
                unsafe { *out = v };
                0
            }
            Err(code) => code,
        }
    }

    let rc = unsafe {
        cpp_ex_map_array(
            input.as_ptr(),
            output.as_mut_ptr(),
            input.len(),
            trampoline::<F>,
            &mut f as *mut F as *mut c_void,
        )
    };
    check(rc)?;
    Ok(output)
}

/* ══════════════════════════════════════════════════════════════
 * CString helper (for convenience in tests)
 * ══════════════════════════════════════════════════════════════ */

/// Create a CString, mapping NUL errors to CppException.
#[allow(dead_code)]
pub fn make_cstring(s: &str) -> Result<CString, CppException> {
    CString::new(s).map_err(|_| CppException {
        message: "interior NUL byte".into(),
        type_name: "Rust".into(),
        code: CPP_EX_ERR_INVALID,
    })
}
