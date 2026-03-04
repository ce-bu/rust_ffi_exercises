//! # Exercise 08: Cross-Boundary Memory Management
//!
//! **Concept:** The golden rule of FFI memory:
//!
//! > **Whoever allocates must also free.**
//!
//! If Rust allocates a buffer, C must call a Rust-provided free
//! function — never `free()`.  And vice versa.
//!
//! ## Patterns you'll implement
//!
//! 1. **Allocate / free a raw buffer:** `Vec<u8>` → `(ptr, len)` via
//!    `mem::forget`, reconstruct with `Vec::from_raw_parts` to free.
//!
//! 2. **Opaque builder:** Incremental construction via a handle, then
//!    "finish" to extract the raw data.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex08
//! ```

use std::mem;
use std::ptr;

// ══════════════════════════════════════════════════════════════
// Part A — Raw buffer round-trip
// ══════════════════════════════════════════════════════════════

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Allocate a zero-filled buffer of `size` bytes.
// Write the length to `*out_len` and return the pointer.
//
// Hint:
//   let mut v = vec![0u8; size];
//   let ptr = v.as_mut_ptr();
//   let len = v.len();
//   std::mem::forget(v);  // prevent drop!
//   *out_len = len;
//   ptr

/// # Safety
/// `out_len` must be a valid writable pointer.
#[no_mangle]
pub unsafe extern "C" fn ffi_alloc_buffer(
    size: usize,
    out_len: *mut usize,
) -> *mut u8 {
    todo!("Allocate a Vec<u8>, forget it, return the raw pointer")
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Free a buffer previously returned by `ffi_alloc_buffer`.
//
// Hint: Vec::from_raw_parts(ptr, len, len) — drop frees it.
//       (We use `len` for both len and capacity because
//        ffi_alloc_buffer allocates exactly `size` bytes.)

/// # Safety
/// `ptr` and `len` must match a previous `ffi_alloc_buffer` call.
#[no_mangle]
pub unsafe extern "C" fn ffi_free_buffer(ptr: *mut u8, len: usize) {
    todo!("Reconstruct the Vec and let it drop")
}

// ══════════════════════════════════════════════════════════════
// Part B — Opaque ByteBuffer builder
// ══════════════════════════════════════════════════════════════

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Define a `ByteBuffer` struct wrapping a `Vec<u8>`.
// This is an opaque handle — C never sees inside.

pub struct ByteBuffer {
    // TODO: add inner Vec<u8>
}

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Implement the builder API.

/// Create a new empty builder.
#[no_mangle]
pub extern "C" fn byte_buffer_new() -> *mut ByteBuffer {
    todo!("Box::new(ByteBuffer {{ ... }}) → Box::into_raw")
}

/// Push a single byte.
///
/// # Safety
/// `bb` must be a valid pointer from `byte_buffer_new`.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_push(bb: *mut ByteBuffer, byte: u8) {
    todo!("Push byte onto the inner Vec")
}

/// Append `len` bytes from `data`.
///
/// # Safety
/// - `bb` must be valid.
/// - `data` must point to at least `len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_append(
    bb: *mut ByteBuffer,
    data: *const u8,
    len: usize,
) {
    todo!("Extend the inner Vec from the slice")
}

/// Read-only access to the builder's current data.
///
/// The returned pointer is valid only until the next mutation.
///
/// # Safety
/// `bb` must be valid.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_data(bb: *const ByteBuffer) -> *const u8 {
    todo!("Return inner Vec's as_ptr()")
}

/// Current length of the buffered data.
///
/// # Safety
/// `bb` must be valid.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_len(bb: *const ByteBuffer) -> usize {
    todo!("Return inner Vec's len()")
}

/// **Consume** the builder: return the raw data pointer and write
/// the length to `*out_len`.  The builder handle becomes invalid.
/// Caller must free the returned pointer with `ffi_free_buffer`.
///
/// # Safety
/// - `bb` must be valid.
/// - `out_len` must be writable.
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_finish(
    bb: *mut ByteBuffer,
    out_len: *mut usize,
) -> *mut u8 {
    todo!(
        "Take back the Box, extract the inner Vec, forget it, \
         return (ptr, len)"
    )
}

/// Destroy the builder **without** extracting data.
///
/// # Safety
/// `bb` must be valid, or null (no-op).
#[no_mangle]
pub unsafe extern "C" fn byte_buffer_free(bb: *mut ByteBuffer) {
    todo!("Box::from_raw(bb) — dropping frees everything")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ex08_alloc_and_free() {
        let mut len: usize = 0;
        let ptr = unsafe { ffi_alloc_buffer(64, &mut len) };
        assert!(!ptr.is_null());
        assert_eq!(len, 64);

        // Buffer should be zero-filled
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        assert!(slice.iter().all(|&b| b == 0));

        unsafe { ffi_free_buffer(ptr, len) };
    }

    #[test]
    fn test_ex08_builder_push_and_len() {
        let bb = byte_buffer_new();
        unsafe {
            byte_buffer_push(bb, b'H');
            byte_buffer_push(bb, b'i');
            assert_eq!(byte_buffer_len(bb), 2);
            byte_buffer_free(bb);
        }
    }

    #[test]
    fn test_ex08_builder_append() {
        let bb = byte_buffer_new();
        let data = b"Hello";
        unsafe {
            byte_buffer_append(bb, data.as_ptr(), data.len());
            assert_eq!(byte_buffer_len(bb), 5);

            let ptr = byte_buffer_data(bb);
            let slice = std::slice::from_raw_parts(ptr, 5);
            assert_eq!(slice, b"Hello");

            byte_buffer_free(bb);
        }
    }

    #[test]
    fn test_ex08_builder_finish() {
        let bb = byte_buffer_new();
        unsafe {
            byte_buffer_push(bb, 0xDE);
            byte_buffer_push(bb, 0xAD);
            let extra: [u8; 2] = [0xBE, 0xEF];
            byte_buffer_append(bb, extra.as_ptr(), 2);

            let mut len: usize = 0;
            let ptr = byte_buffer_finish(bb, &mut len);
            // bb is now consumed — do NOT use it again.

            assert_eq!(len, 4);
            let slice = std::slice::from_raw_parts(ptr, len);
            assert_eq!(slice, &[0xDE, 0xAD, 0xBE, 0xEF]);

            ffi_free_buffer(ptr, len);
        }
    }
}
