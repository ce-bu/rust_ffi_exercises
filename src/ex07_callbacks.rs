//! # Exercise 07: Function-Pointer Callbacks
//!
//! **Concept:** Since Rust closures cannot cross an FFI boundary,
//! callbacks are passed as bare `extern "C" fn` pointers.  For state,
//! use the classic C pattern: `fn(value, *mut c_void)` where
//! `c_void` carries user data.
//!
//! This exercise covers both directions:
//! - **Rust → C:** Call a C function and pass a Rust function pointer.
//! - **C → Rust:** Implement Rust `extern "C"` functions that accept
//!   and invoke callback pointers.
//!
//! ## Pre-provided (in `csrc/ex07_c_callbacks.c`)
//!
//! ```c
//! void    c_for_each(const int32_t *array, size_t len,
//!                    void (*cb)(int32_t, void*), void *ctx);
//! int32_t c_transform(int32_t value, int32_t (*f)(int32_t));
//! ```
//!
//! ## Your task
//!
//! Implement the four TODO sections.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex07
//! ```

use std::ffi::c_void;

// ── Pre-provided extern declarations ───────────────────────────

extern "C" {
    fn c_for_each(
        array: *const i32,
        len: usize,
        callback: extern "C" fn(i32, *mut c_void),
        user_data: *mut c_void,
    );

    fn c_transform(value: i32, transform: extern "C" fn(i32) -> i32);
}

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Call the C function `c_for_each` with an array and a Rust callback
// that **accumulates** each element into a `Vec<i32>` via the
// `user_data` pointer.
//
// Steps:
//   1. Create a `Vec<i32>`.
//   2. Write an `extern "C" fn accumulate(value: i32, ctx: *mut c_void)`
//      that pushes `value` onto the Vec.
//   3. Call `c_for_each`, passing the Vec as user_data.
//   4. Return the Vec.
//
// Hint: cast `&mut vec as *mut Vec<i32> as *mut c_void`

/// Collect array elements into a Vec by using `c_for_each`.
pub fn collect_via_c(data: &[i32]) -> Vec<i32> {
    todo!("Call c_for_each with a callback that accumulates into a Vec")
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Call the C function `c_transform` with a Rust function pointer.
//
// Write a safe wrapper that squares a number via C dispatch.

/// Returns `value * value` by delegating to `c_transform`.
pub fn square_via_c(value: i32) -> i32 {
    todo!("Define an extern 'C' fn that squares, pass it to c_transform")
}

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Implement an `extern "C"` function that accepts a callback and
// invokes it for each number in `0..n`.
//
// This is the **Rust side** version of c_for_each.

/// Call `cb(i, user_data)` for `i` in `0..n`.
///
/// # Safety
/// `user_data` must be valid for the callback's use, or null if
/// the callback ignores it.
#[no_mangle]
pub unsafe extern "C" fn rust_for_each(
    n: i32,
    cb: extern "C" fn(i32, *mut c_void),
    user_data: *mut c_void,
) {
    todo!("Loop 0..n and call cb(i, user_data) each iteration")
}

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Implement an `extern "C"` comparator-based selection: call `cmp`
// to compare elements pairwise and return the "best" element.
//
// `cmp(a, b)` returns:
//    negative  → a wins
//    zero      → tie (keep a)
//    positive  → b wins

/// # Safety
/// `data` must point to `len` valid `i32` elements. `len` must be ≥ 1.
#[no_mangle]
pub unsafe extern "C" fn rust_select(
    data: *const i32,
    len: usize,
    cmp: extern "C" fn(i32, i32) -> i32,
) -> i32 {
    todo!("Iterate the array, use cmp to pick the 'best' element")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;

    #[test]
    fn test_ex07_collect_via_c() {
        let data = [10, 20, 30];
        let collected = collect_via_c(&data);
        assert_eq!(collected, vec![10, 20, 30]);
    }

    #[test]
    fn test_ex07_square_via_c() {
        assert_eq!(square_via_c(5), 25);
        assert_eq!(square_via_c(-3), 9);
        assert_eq!(square_via_c(0), 0);
    }

    #[test]
    fn test_ex07_rust_for_each() {
        let mut acc: Vec<i32> = Vec::new();

        extern "C" fn push(val: i32, ctx: *mut c_void) {
            let vec = unsafe { &mut *(ctx as *mut Vec<i32>) };
            vec.push(val);
        }

        unsafe {
            rust_for_each(
                4,
                push,
                &mut acc as *mut Vec<i32> as *mut c_void,
            );
        }
        assert_eq!(acc, vec![0, 1, 2, 3]);
    }

    #[test]
    fn test_ex07_rust_select_max() {
        let data = [3, 1, 4, 1, 5, 9, 2, 6];

        extern "C" fn prefer_larger(a: i32, b: i32) -> i32 {
            b - a // positive when b > a → b wins
        }

        let best = unsafe {
            rust_select(data.as_ptr(), data.len(), prefer_larger)
        };
        assert_eq!(best, 9);
    }

    #[test]
    fn test_ex07_rust_select_min() {
        let data = [3, 1, 4, 1, 5];

        extern "C" fn prefer_smaller(a: i32, b: i32) -> i32 {
            a - b // positive when a > b → b wins
        }

        let best = unsafe {
            rust_select(data.as_ptr(), data.len(), prefer_smaller)
        };
        assert_eq!(best, 1);
    }
}
