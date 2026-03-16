//! # Exercise 17: Wrapping a C Library's Opaque Handles
//!
//! **Concept:** Most real-world C libraries expose opaque handles —
//! a pointer to an incomplete type that you pass to every API
//! function.  Think `sqlite3*`, `SSL_CTX*`, `CURL*`, etc.
//!
//! The idiomatic Rust pattern is:
//!
//! 1. Declare the opaque type as a zero-sized `#[repr(C)]` struct
//!    (or use `std::ffi::c_void`).
//! 2. Write `extern "C"` bindings that mirror the C header.
//! 3. Build a **safe wrapper** struct that:
//!    - Owns the raw handle (stored as a `NonNull<OpaqueType>`).
//!    - Implements `Drop` to call the C destructor.
//!    - Provides safe methods returning `Result<T, E>`.
//!
//! ## The C library
//!
//! `csrc/ex17_cdb.c` implements a tiny key-value "database":
//!
//! ```text
//! cdb_open(path)               → CdbHandle*
//! cdb_put(db, key, value)      → int status
//! cdb_get(db, key, buf, len)   → int status
//! cdb_delete(db, key)          → int status
//! cdb_count(db)                → size_t
//! cdb_close(db)                → int status
//! ```
//!
//! Status codes: `CDB_OK (0)`, `CDB_ERR_INVALID (-1)`,
//!               `CDB_ERR_NOT_FOUND (-2)`, `CDB_ERR_OVERFLOW (-3)`.
//!
//! ## Your task
//!
//! 1. Write the raw FFI bindings (`extern "C"` block).
//! 2. Define a `CdbError` enum mapping the C error codes.
//! 3. Implement `Database` — a safe, RAII wrapper that:
//!    - Opens/closes the handle automatically.
//!    - Converts C errors to `Result`.
//!    - Manages the caller-provided buffer for `cdb_get`.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex17
//! ```

use std::ffi::{c_char, c_int, CStr, CString};
use std::ptr::NonNull;

// ══════════════════════════════════════════════════════════════
// Part A — Raw FFI bindings
// ══════════════════════════════════════════════════════════════

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Declare the opaque handle type.  In C it is
//    typedef struct CdbHandle CdbHandle;
// In Rust we create an uninhabited, zero-sized repr(C) type.
// This ensures no one can construct a CdbHandle on the Rust
// side — it only ever lives behind a raw pointer.
//
// Example:
//   #[repr(C)]
//   pub struct CdbHandle { _private: [u8; 0] }

#[repr(C)]
pub struct CdbHandle {
    // TODO: add a zero-sized private field so the type is FFI-safe
    // but unconstructable from Rust code.
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Write the `extern "C"` block that declares all six C functions:
//   cdb_open, cdb_put, cdb_get, cdb_delete, cdb_count, cdb_close
//
// Match the signatures from include/exercises.h exactly.  Use:
//   *const c_char     for `const char *`
//   *mut c_char       for `char *`
//   *mut CdbHandle    for `CdbHandle *`
//   c_int             for `int`
//   usize             for `size_t`

extern "C" {
    // TODO: declare cdb_open
    //   fn cdb_open(path: ...) -> *mut CdbHandle;
    //
    // TODO: declare cdb_put
    //   fn cdb_put(db: ..., key: ..., value: ...) -> c_int;
    //
    // TODO: declare cdb_get
    //   fn cdb_get(db: ..., key: ..., out_value: ..., out_len: ...) -> c_int;
    //
    // TODO: declare cdb_delete
    //   fn cdb_delete(db: ..., key: ...) -> c_int;
    //
    // TODO: declare cdb_count
    //   fn cdb_count(db: ...) -> usize;
    //
    // TODO: declare cdb_close
    //   fn cdb_close(db: ...) -> c_int;
}

// ══════════════════════════════════════════════════════════════
// Part B — Error type
// ══════════════════════════════════════════════════════════════

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Define an error enum with variants for each CDB_ERR_* code.
//
// Map from the C status codes:
//   CDB_ERR_INVALID  (-1) → CdbError::Invalid
//   CDB_ERR_NOT_FOUND(-2) → CdbError::NotFound
//   CDB_ERR_OVERFLOW (-3) → CdbError::Overflow
//   anything else         → CdbError::Unknown(c_int)
//
// Implement `From<c_int>` for CdbError, and implement
// `std::fmt::Display` and `std::error::Error`.

#[derive(Debug, PartialEq, Eq)]
pub enum CdbError {
    Invalid,
    NotFound,
    Overflow,
    Unknown,
}

impl From<c_int> for CdbError {
    fn from(code: c_int) -> Self {
        todo!("Map -1, -2, -3 to the correct variant, else Unknown(code)")
    }
}

impl std::fmt::Display for CdbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!("Display a human-readable message for each variant")
    }
}

impl std::error::Error for CdbError {}

/// Convert a C return code to `Result<(), CdbError>`.
/// `CDB_OK (0)` → `Ok(())`, everything else → `Err(...)`.
fn check(code: c_int) -> Result<(), CdbError> {
    todo!("if code == 0 {{ Ok(()) }} else {{ Err(CdbError::from(code)) }}")
}

// ══════════════════════════════════════════════════════════════
// Part C — Safe RAII wrapper
// ══════════════════════════════════════════════════════════════

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Define `Database` — a safe wrapper around `*mut CdbHandle`.
//
// Store the handle as `NonNull<CdbHandle>` for null-safety.
//
// The struct must NOT implement Copy/Clone — the handle must
// have a single owner, and `Drop` must call `cdb_close`.

pub struct Database {
    // TODO: store the handle as NonNull<CdbHandle>
}

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Implement the safe public API on `Database`.

impl Database {
    /// Open a new database handle.
    ///
    /// Calls `cdb_open`.  Returns `Err(CdbError::Invalid)` if the
    /// C library returns NULL.
    pub fn open(path: &str) -> Result<Self, CdbError> {
        todo!(
            "Convert `path` to CString, call cdb_open, \
             wrap NonNull::new(...).ok_or(CdbError::Invalid)"
        )
    }

    /// Insert or update a key-value pair.
    pub fn put(&mut self, key: &str, value: &str) -> Result<(), CdbError> {
        todo!(
            "Convert key/value to CString, call cdb_put, \
             use `check()` to convert the return code"
        )
    }

    /// Look up the value for `key`.
    ///
    /// Returns `Ok(String)` on success, or `Err(CdbError::NotFound)`
    /// if the key does not exist.
    ///
    /// Hint: allocate a `Vec<u8>` buffer of 256 bytes, pass it to
    /// `cdb_get`, then convert the filled buffer to a `String` via
    /// `CStr::from_ptr`.
    pub fn get(&self, key: &str) -> Result<String, CdbError> {
        todo!(
            "Allocate a buffer, call cdb_get, convert the \
             C string in the buffer to a Rust String"
        )
    }

    /// Delete a key.  Returns `Err(CdbError::NotFound)` if absent.
    pub fn delete(&mut self, key: &str) -> Result<(), CdbError> {
        todo!("Convert key to CString, call cdb_delete, use check()")
    }

    /// Number of stored entries.
    pub fn count(&self) -> usize {
        todo!("Call cdb_count")
    }
}

// ── TODO 6 ─────────────────────────────────────────────────────
//
// Implement `Drop` so the handle is always closed.
//
// Call `cdb_close` in `drop`.  Ignore the return code (we can't
// propagate errors from `Drop`).

impl Drop for Database {
    fn drop(&mut self) {
        todo!("Call cdb_close(self.handle.as_ptr())")
    }
}

// ══════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ex17_open_close() {
        let db = Database::open("test.db").expect("open failed");
        assert_eq!(db.count(), 0);
        drop(db); // should call cdb_close via Drop
    }

    #[test]
    fn test_ex17_open_empty_path_fails() {
        let result = Database::open("");
        assert!(result.is_err());
    }

    #[test]
    fn test_ex17_put_and_get() {
        let mut db = Database::open("test.db").unwrap();
        db.put("language", "Rust").unwrap();
        assert_eq!(db.get("language").unwrap(), "Rust");
    }

    #[test]
    fn test_ex17_get_missing_key() {
        let db = Database::open("test.db").unwrap();
        let err = db.get("nonexistent").unwrap_err();
        assert_eq!(err, CdbError::NotFound);
    }

    #[test]
    fn test_ex17_put_update() {
        let mut db = Database::open("test.db").unwrap();
        db.put("k", "v1").unwrap();
        db.put("k", "v2").unwrap();
        assert_eq!(db.count(), 1);
        assert_eq!(db.get("k").unwrap(), "v2");
    }

    #[test]
    fn test_ex17_delete() {
        let mut db = Database::open("test.db").unwrap();
        db.put("a", "1").unwrap();
        db.put("b", "2").unwrap();
        assert_eq!(db.count(), 2);

        db.delete("a").unwrap();
        assert_eq!(db.count(), 1);
        assert_eq!(db.get("a").unwrap_err(), CdbError::NotFound);
    }

    #[test]
    fn test_ex17_delete_missing() {
        let mut db = Database::open("test.db").unwrap();
        let err = db.delete("nope").unwrap_err();
        assert_eq!(err, CdbError::NotFound);
    }

    #[test]
    fn test_ex17_count() {
        let mut db = Database::open("test.db").unwrap();
        assert_eq!(db.count(), 0);
        db.put("x", "1").unwrap();
        assert_eq!(db.count(), 1);
        db.put("y", "2").unwrap();
        assert_eq!(db.count(), 2);
        db.delete("x").unwrap();
        assert_eq!(db.count(), 1);
    }

    #[test]
    fn test_ex17_multiple_databases() {
        let mut db1 = Database::open("one.db").unwrap();
        let mut db2 = Database::open("two.db").unwrap();

        db1.put("k", "from_db1").unwrap();
        db2.put("k", "from_db2").unwrap();

        assert_eq!(db1.get("k").unwrap(), "from_db1");
        assert_eq!(db2.get("k").unwrap(), "from_db2");
    }

    #[test]
    fn test_ex17_error_display() {
        // Verify the error type implements Display (required by
        // std::error::Error).
        let err = CdbError::NotFound;
        let msg = format!("{err}");
        assert!(!msg.is_empty());
    }
}
