//! # Exercise 06: Array and Slice Passing
//!
//! **Concept:** C has no slice type — arrays are `(pointer, length)`
//! pairs.  Rust reconstructs slices with `slice::from_raw_parts`.
//!
//! Key patterns:
//! - **Read a C array:**  `slice::from_raw_parts(ptr, len)`.
//! - **Write into a C buffer:**  `slice::from_raw_parts_mut(ptr, len)`.
//! - **Return a Rust-allocated array:**  `Vec` → `(ptr, len)` via
//!   `mem::forget` + return pointer.  Caller frees with a matching
//!   `ffi_free_*` function.
//!
//! ## Your task
//!
//! Implement the six functions below.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex06
//! ```

use std::slice;

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Sum all elements of a C `int32_t` array.
// Return 0 if `data` is null or `len` is 0.
//
// Hint: slice::from_raw_parts(data, len).iter().map(|&x| x as i64).sum()

/// # Safety
/// `data` must point to at least `len` valid `i32` elements (or be null).
#[no_mangle]
pub unsafe extern "C" fn ffi_sum(data: *const i32, len: usize) -> i64 {
    todo!("Reconstruct a slice and compute the sum")
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Fill a caller-provided buffer with the first `len` Fibonacci
// numbers: F(0)=0, F(1)=1, F(2)=1, F(3)=2, …
//
// Hint: slice::from_raw_parts_mut(buf, len), then fill iteratively.

/// # Safety
/// `buf` must point to at least `len` writable `u64` elements.
#[no_mangle]
pub unsafe extern "C" fn ffi_fill_fibonacci(buf: *mut u64, len: usize) {
    todo!("Write the Fibonacci sequence into the buffer")
}

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Allocate and return a new array containing the integers
// `[start, start+1, …, end-1]`.  Write the array length into
// `*out_len`.
//
// The caller must free the array with `ffi_free_int_array`.
//
// Hint:
//   let v: Vec<i32> = (start..end).collect();
//   let ptr = v.as_ptr();           // ← don't use as_mut_ptr on immutable
//   let len = v.len();
//   std::mem::forget(v);            // prevent deallocation
//   *out_len = len;
//   ptr as *mut i32

/// # Safety
/// `out_len` must be a valid writable pointer.
#[no_mangle]
pub unsafe extern "C" fn ffi_create_range(
    start: i32,
    end: i32,
    out_len: *mut usize,
) -> *mut i32 {
    todo!("Build a Vec, forget it, and return the raw pointer")
}

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Free an array previously returned by `ffi_create_range`.
//
// Hint: Vec::from_raw_parts(ptr, len, len) — dropping it frees.

/// # Safety
/// `ptr` and `len` must have been returned together by `ffi_create_range`.
#[no_mangle]
pub unsafe extern "C" fn ffi_free_int_array(ptr: *mut i32, len: usize) {
    todo!("Reconstruct the Vec and let it drop")
}

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Sort the array **in-place** in descending order.
//
// Hint: slice::from_raw_parts_mut → sort_by, or sort then reverse.

/// # Safety
/// `data` must point to at least `len` valid, writable `i32` elements.
#[no_mangle]
pub unsafe extern "C" fn ffi_sort_desc(data: *mut i32, len: usize) {
    todo!("Sort the slice in descending order in-place")
}

// ── TODO 6 ─────────────────────────────────────────────────────
//
// Apply a mapping function to every element and return a **new**
// array with the results.  Write the length to `*out_len`.
// Caller frees with `ffi_free_int_array`.

/// # Safety
/// - `src` must point to `len` valid `i32` elements.
/// - `out_len` must be a valid writable pointer.
/// - `f` must be a valid function pointer.
#[no_mangle]
pub unsafe extern "C" fn ffi_map(
    src: *const i32,
    len: usize,
    f: extern "C" fn(i32) -> i32,
    out_len: *mut usize,
) -> *mut i32 {
    todo!("Map f over the input slice and return a new array")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ex06_sum() {
        let data = [1, 2, 3, 4, 5];
        let total = unsafe { ffi_sum(data.as_ptr(), data.len()) };
        assert_eq!(total, 15);
    }

    #[test]
    fn test_ex06_sum_empty() {
        let total = unsafe { ffi_sum(std::ptr::null(), 0) };
        assert_eq!(total, 0);
    }

    #[test]
    fn test_ex06_fill_fibonacci() {
        let mut buf = [0u64; 8];
        unsafe { ffi_fill_fibonacci(buf.as_mut_ptr(), buf.len()) };
        assert_eq!(buf, [0, 1, 1, 2, 3, 5, 8, 13]);
    }

    #[test]
    fn test_ex06_create_range_and_free() {
        let mut len: usize = 0;
        let ptr = unsafe { ffi_create_range(3, 7, &mut len) };
        assert_eq!(len, 4);
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        assert_eq!(slice, &[3, 4, 5, 6]);
        unsafe { ffi_free_int_array(ptr, len) };
    }

    #[test]
    fn test_ex06_sort_desc() {
        let mut data = [3, 1, 4, 1, 5, 9, 2, 6];
        unsafe { ffi_sort_desc(data.as_mut_ptr(), data.len()) };
        assert_eq!(data, [9, 6, 5, 4, 3, 2, 1, 1]);
    }

    #[test]
    fn test_ex06_map() {
        let input = [1, 2, 3, 4];
        extern "C" fn double_it(x: i32) -> i32 { x * 2 }

        let mut out_len: usize = 0;
        let ptr = unsafe {
            ffi_map(input.as_ptr(), input.len(), double_it, &mut out_len)
        };
        assert_eq!(out_len, 4);
        let slice = unsafe { std::slice::from_raw_parts(ptr, out_len) };
        assert_eq!(slice, &[2, 4, 6, 8]);
        unsafe { ffi_free_int_array(ptr, out_len) };
    }
}
