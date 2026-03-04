//! # Exercise 05: Opaque Handle Pattern
//!
//! **Concept:** C code holds a pointer to a Rust object whose
//! internal layout is hidden ("opaque").  The lifecycle is always:
//!
//! ```text
//! create  →  use (one or more calls)  →  destroy
//! ```
//!
//! Rust side:
//! - `Box::new(value)` → `Box::into_raw(box)` to hand ownership to C.
//! - `Box::from_raw(ptr)` to reclaim ownership and drop.
//!
//! C side sees only `typedef struct Config Config;` — it cannot
//! access fields.
//!
//! ## Your task
//!
//! Implement a key-value `Config` store exposed through an opaque
//! handle API.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex05
//! ```

use std::collections::HashMap;
use std::ffi::{c_char, CStr, CString};

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Define the `Config` struct.  It does NOT need `#[repr(C)]`
// because C never sees its layout — only a pointer.
//
// Internally it should wrap a `HashMap<String, String>`.

pub struct Config {
    // TODO: add a field for the inner HashMap
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Implement the extern "C" API below.
//
// Memory rules:
//   - config_new:          allocates Config via Box::into_raw
//   - config_set:          borrows key/value from C (CStr), stores copies
//   - config_get:          returns a COPY as CString::into_raw (caller frees)
//   - config_remove:       removes a key, returns whether it existed
//   - config_count:        number of entries
//   - config_free_string:  frees a string returned by config_get
//   - config_free:         drops the Config via Box::from_raw

/// Create a new, empty `Config`.
#[no_mangle]
pub extern "C" fn config_new() -> *mut Config {
    todo!("Box::new(Config {{ ... }}) then Box::into_raw")
}

/// Insert or update a key-value pair.
///
/// # Safety
/// - `cfg` must be a valid pointer from `config_new`.
/// - `key` and `value` must be valid C strings.
#[no_mangle]
pub unsafe extern "C" fn config_set(
    cfg: *mut Config,
    key: *const c_char,
    value: *const c_char,
) {
    todo!("Convert key/value from CStr, insert into the HashMap")
}

/// Retrieve the value for `key`.  Returns a **new** heap-allocated
/// C string (caller must free with `config_free_string`), or null
/// if the key does not exist.
///
/// # Safety
/// - `cfg` must be a valid pointer from `config_new`.
/// - `key` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn config_get(
    cfg: *const Config,
    key: *const c_char,
) -> *mut c_char {
    todo!("Look up key in HashMap, return CString::into_raw or null")
}

/// Remove a key.  Returns `true` if the key existed.
///
/// # Safety
/// - `cfg` must be a valid pointer from `config_new`.
/// - `key` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn config_remove(
    cfg: *mut Config,
    key: *const c_char,
) -> bool {
    todo!("Remove the key from the HashMap")
}

/// Return the number of stored key-value pairs.
///
/// # Safety
/// `cfg` must be a valid pointer from `config_new`.
#[no_mangle]
pub unsafe extern "C" fn config_count(cfg: *const Config) -> usize {
    todo!("Return HashMap::len()")
}

/// Free a string returned by `config_get`.
///
/// # Safety
/// `s` was allocated by `CString::into_raw`, or is null.
#[no_mangle]
pub unsafe extern "C" fn config_free_string(s: *mut c_char) {
    todo!("Reclaim CString via from_raw (no-op if null)")
}

/// Destroy the Config, releasing all memory.
///
/// # Safety
/// `cfg` must have been returned by `config_new` and must not be
/// used after this call.
#[no_mangle]
pub unsafe extern "C" fn config_free(cfg: *mut Config) {
    todo!("Box::from_raw(cfg) — dropping the Box frees everything")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::{CStr, CString};
    use std::ptr;

    /// Helper: create a CString and return its pointer.
    fn cstr(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    #[test]
    fn test_ex05_create_and_free() {
        let cfg = config_new();
        assert!(!cfg.is_null());
        unsafe { config_free(cfg) };
    }

    #[test]
    fn test_ex05_set_and_get() {
        let cfg = config_new();
        let k = cstr("name");
        let v = cstr("Ferris");

        unsafe {
            config_set(cfg, k.as_ptr(), v.as_ptr());
            let result = config_get(cfg, k.as_ptr());
            assert!(!result.is_null());
            assert_eq!(CStr::from_ptr(result).to_str().unwrap(), "Ferris");
            config_free_string(result);
            config_free(cfg);
        }
    }

    #[test]
    fn test_ex05_get_missing_key() {
        let cfg = config_new();
        let k = cstr("nonexistent");
        unsafe {
            let result = config_get(cfg, k.as_ptr());
            assert!(result.is_null());
            config_free(cfg);
        }
    }

    #[test]
    fn test_ex05_remove() {
        let cfg = config_new();
        let k = cstr("key");
        let v = cstr("value");
        unsafe {
            config_set(cfg, k.as_ptr(), v.as_ptr());
            assert_eq!(config_count(cfg), 1);
            assert!(config_remove(cfg, k.as_ptr()));
            assert_eq!(config_count(cfg), 0);
            assert!(!config_remove(cfg, k.as_ptr())); // already gone
            config_free(cfg);
        }
    }

    #[test]
    fn test_ex05_count() {
        let cfg = config_new();
        let k1 = cstr("a");
        let v1 = cstr("1");
        let k2 = cstr("b");
        let v2 = cstr("2");
        unsafe {
            assert_eq!(config_count(cfg), 0);
            config_set(cfg, k1.as_ptr(), v1.as_ptr());
            assert_eq!(config_count(cfg), 1);
            config_set(cfg, k2.as_ptr(), v2.as_ptr());
            assert_eq!(config_count(cfg), 2);
            config_free(cfg);
        }
    }

    #[test]
    fn test_ex05_overwrite() {
        let cfg = config_new();
        let k = cstr("key");
        let v1 = cstr("first");
        let v2 = cstr("second");
        unsafe {
            config_set(cfg, k.as_ptr(), v1.as_ptr());
            config_set(cfg, k.as_ptr(), v2.as_ptr());
            assert_eq!(config_count(cfg), 1);
            let result = config_get(cfg, k.as_ptr());
            assert_eq!(CStr::from_ptr(result).to_str().unwrap(), "second");
            config_free_string(result);
            config_free(cfg);
        }
    }
}
