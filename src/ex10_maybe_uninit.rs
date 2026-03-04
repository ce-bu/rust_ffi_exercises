//! # Exercise 10: `MaybeUninit` and `NonNull`
//!
//! **Concept:** Raw pointers are error-prone.  Rust provides two
//! tools to make FFI safer without runtime cost:
//!
//! - **`MaybeUninit<T>`** — a wrapper that tells the compiler "this
//!   memory may not be initialized yet."  Use it for **out-parameters**
//!   to avoid undefined behavior from reading uninitialized memory.
//!
//! - **`NonNull<T>`** — a pointer that is guaranteed non-null at the
//!   type level.  Use it inside safe wrapper types to encode the
//!   invariant "this handle always points to something."
//!
//! ## Your task
//!
//! Implement the three parts below.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex10
//! ```

use std::mem::MaybeUninit;
use std::ptr::NonNull;
use std::ffi::c_void;

// ══════════════════════════════════════════════════════════════
// Part A — MaybeUninit for out-parameters
// ══════════════════════════════════════════════════════════════

/// A version struct returned via an out-parameter.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Fill in a `Version` via an out-parameter.  C code would call
// this as:
//
//   Version v;
//   ffi_get_version(&v);
//
// Using `MaybeUninit` on the Rust side is safer than using `*mut Version`
// directly because it prevents reading uninitialized memory.
//
// Steps:
//   1. Check that `out` is non-null (return -1 if null).
//   2. Use `out.write(Version { major: 1, minor: 4, patch: 0 })`.
//   3. Return 0.
//
// NOTE: The function signature uses `*mut MaybeUninit<Version>`.
//       If the C caller passes `&v`, Rust writes into it safely
//       via `MaybeUninit::write()` — no read of uninitialized data.

/// Write the current library version into `*out`.
/// Returns 0 on success, -1 if `out` is null.
///
/// # Safety
/// `out` must point to valid (possibly uninitialized) memory for
/// one `Version`, or be null.
#[no_mangle]
pub unsafe extern "C" fn ffi_get_version(
    out: *mut MaybeUninit<Version>,
) -> i32 {
    todo!("Write Version {{ 1, 4, 0 }} into *out using MaybeUninit::write")
}

// ══════════════════════════════════════════════════════════════
// Part B — NonNull wrapper for opaque handles
// ══════════════════════════════════════════════════════════════

/// Internal resource (not exposed to C).
pub struct Resource {
    pub name: String,
    pub value: i64,
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Create a safe Rust wrapper around an opaque `*mut Resource`
// that uses `NonNull` to guarantee the pointer is never null.
//
// Implement:
//   - `SafeHandle::new(name, value)` → `Option<Self>`
//       Allocates via Box::into_raw, wraps in NonNull.
//   - `SafeHandle::name(&self)` → `&str`
//   - `SafeHandle::value(&self)` → i64
//   - `SafeHandle::set_value(&mut self, val: i64)`
//   - Drop: reclaims via Box::from_raw
//
// Key: `NonNull::new(ptr)` returns `None` if ptr is null.

pub struct SafeHandle {
    // TODO: store a NonNull<Resource>
    _placeholder: (),
}

impl SafeHandle {
    /// Create a new handle.  Returns `None` if allocation fails
    /// (extremely unlikely but the API is honest about it).
    pub fn new(name: &str, value: i64) -> Option<Self> {
        todo!("Box::new → into_raw → NonNull::new → wrap in SafeHandle")
    }

    pub fn name(&self) -> &str {
        todo!("Dereference NonNull, return &name")
    }

    pub fn value(&self) -> i64 {
        todo!("Dereference NonNull, return value")
    }

    pub fn set_value(&mut self, val: i64) {
        todo!("Dereference NonNull mutably, update value")
    }
}

// TODO: implement Drop for SafeHandle to free the Resource
// impl Drop for SafeHandle { ... }

// ══════════════════════════════════════════════════════════════
// Part C — Initializing an array with MaybeUninit
// ══════════════════════════════════════════════════════════════

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Initialize a caller-allocated array of `MaybeUninit<i32>` with
// the values `[0, 1, 4, 9, 16, …]` (squares).
//
// After this call, the caller can safely call `assume_init` on
// each element.
//
// Steps:
//   1. Check null / zero length.
//   2. Loop `0..len`, write `(i * i) as i32` via `(*buf.add(i)).write(val)`.
//   3. Return 0.

/// # Safety
/// `buf` must point to at least `len` `MaybeUninit<i32>` slots.
#[no_mangle]
pub unsafe extern "C" fn ffi_init_squares(
    buf: *mut MaybeUninit<i32>,
    len: usize,
) -> i32 {
    todo!("Initialize each MaybeUninit slot with i*i")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::MaybeUninit;

    #[test]
    fn test_ex10_get_version() {
        let mut v = MaybeUninit::<Version>::uninit();
        let rc = unsafe { ffi_get_version(&mut v) };
        assert_eq!(rc, 0);
        let v = unsafe { v.assume_init() };
        assert_eq!(v, Version { major: 1, minor: 4, patch: 0 });
    }

    #[test]
    fn test_ex10_get_version_null() {
        let rc = unsafe { ffi_get_version(std::ptr::null_mut()) };
        assert_eq!(rc, -1);
    }

    #[test]
    fn test_ex10_safe_handle_create() {
        let h = SafeHandle::new("test", 42).expect("allocation failed");
        assert_eq!(h.name(), "test");
        assert_eq!(h.value(), 42);
    }

    #[test]
    fn test_ex10_safe_handle_mutate() {
        let mut h = SafeHandle::new("counter", 0).unwrap();
        h.set_value(100);
        assert_eq!(h.value(), 100);
        // h is dropped here — should not leak
    }

    #[test]
    fn test_ex10_init_squares() {
        let mut buf: [MaybeUninit<i32>; 5] = unsafe {
            MaybeUninit::uninit().assume_init()
        };
        let rc = unsafe { ffi_init_squares(buf.as_mut_ptr(), buf.len()) };
        assert_eq!(rc, 0);

        let values: Vec<i32> = buf
            .iter()
            .map(|x| unsafe { x.assume_init() })
            .collect();
        assert_eq!(values, vec![0, 1, 4, 9, 16]);
    }
}
