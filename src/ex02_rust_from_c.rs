//! # Exercise 02: Exposing Rust Functions to C
//!
//! **Concept:** Make Rust functions callable from C code by using
//! `#[no_mangle]` and `extern "C"`.
//!
//! ## Background
//!
//! By default Rust *mangles* symbol names, making them invisible to C.
//! To export a function with C linkage you need:
//!
//! ```rust,ignore
//! #[no_mangle]
//! pub extern "C" fn my_function(x: i32) -> i32 { ... }
//! ```
//!
//! - `#[no_mangle]` — keeps the symbol name as-is in the binary.
//! - `extern "C"`   — uses the C calling convention.
//!
//! ## Your task
//!
//! An example (`rust_add`) is provided.  Implement the remaining three
//! functions following the same pattern.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex02
//! ```

// ── Example (pre-provided) ─────────────────────────────────────

/// Returns `a + b`.  Already implemented as a reference.
#[no_mangle]
pub extern "C" fn rust_add(a: i32, b: i32) -> i32 {
    a + b
}

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Return the **larger** of the two values.

#[no_mangle]
pub extern "C" fn rust_max(a: i32, b: i32) -> i32 {
    a.max(b)
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Return the `n`-th Fibonacci number (0-indexed).
// F(0) = 0, F(1) = 1, F(2) = 1, F(3) = 2, …

#[no_mangle]
pub extern "C" fn rust_fibonacci(n: u32) -> u64 {
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }

    let mut x_1 = 1;
    let mut x = 1;
    for _i in 2..n {
        let tmp = x_1 + x;
        x_1 = x;
        x = tmp;
    }
    x
}

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Return `true` if `n` is a prime number, `false` otherwise.
// 0 and 1 are **not** prime.

#[no_mangle]
pub extern "C" fn rust_is_prime(n: u32) -> bool {
    if n <= 1 {
        return false;
    }
    // 2 and 3 are prime numbers
    if n <= 3 {
        return true;
    }
    // Eliminate multiples of 2 and 3
    if n.is_multiple_of(2) || n.is_multiple_of(3) {
        return false;
    }

    // Check factors up to the square root of n
    // Using a step of 6 (checking i and i + 2) optimizes the search
    let mut i = 5;
    while i * i <= n {
        if n.is_multiple_of(i) || n.is_multiple_of(i + 2) {
            return false;
        }
        i += 6;
    }

    true
}

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Return the greatest common divisor of `a` and `b` using the
// Euclidean algorithm.  `gcd(0, 0)` should return `0`.

#[no_mangle]
pub extern "C" fn rust_gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let temp = b;
        b = a % b;
        a = temp;
    }
    a
}

// ── Tests ──────────────────────────────────────────────────────
//
// These tests call the functions through their extern "C" signatures,
// simulating how C code would call them.

#[cfg(test)]
mod tests {
    // Import the extern "C" functions by their symbol name.
    extern "C" {
        fn rust_add(a: i32, b: i32) -> i32;
        fn rust_max(a: i32, b: i32) -> i32;
        fn rust_fibonacci(n: u32) -> u64;
        fn rust_is_prime(n: u32) -> bool;
        fn rust_gcd(a: u32, b: u32) -> u32;
    }

    #[test]
    fn test_ex02_add() {
        unsafe {
            assert_eq!(rust_add(10, 20), 30);
        }
    }

    #[test]
    fn test_ex02_max() {
        unsafe {
            assert_eq!(rust_max(3, 7), 7);
            assert_eq!(rust_max(7, 3), 7);
            assert_eq!(rust_max(-1, -5), -1);
            assert_eq!(rust_max(0, 0), 0);
        }
    }

    #[test]
    fn test_ex02_fibonacci() {
        unsafe {
            assert_eq!(rust_fibonacci(0), 0);
            assert_eq!(rust_fibonacci(1), 1);
            assert_eq!(rust_fibonacci(2), 1);
            assert_eq!(rust_fibonacci(10), 55);
            assert_eq!(rust_fibonacci(20), 6765);
        }
    }

    #[test]
    fn test_ex02_is_prime() {
        unsafe {
            assert!(!rust_is_prime(0));
            assert!(!rust_is_prime(1));
            assert!(rust_is_prime(2));
            assert!(rust_is_prime(3));
            assert!(!rust_is_prime(4));
            assert!(rust_is_prime(97));
        }
    }

    #[test]
    fn test_ex02_gcd() {
        unsafe {
            assert_eq!(rust_gcd(12, 8), 4);
            assert_eq!(rust_gcd(7, 13), 1);
            assert_eq!(rust_gcd(0, 5), 5);
            assert_eq!(rust_gcd(0, 0), 0);
        }
    }
}
