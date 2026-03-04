//! # Exercise 01: Calling C from Rust
//!
//! **Concept:** The most fundamental FFI operation — importing C
//! functions so Rust code can call them.
//!
//! ## Background
//!
//! C functions compiled by `build.rs` (via the `cc` crate) are linked
//! into the final binary.  Rust can call them by declaring matching
//! signatures in an `extern "C" { }` block.  Every such call is
//! `unsafe` because the compiler cannot verify the C side.
//!
//! ## Pre-provided
//!
//! `csrc/ex01_math.c` defines four functions:
//!
//! | C signature                                                 | Purpose          |
//! |-------------------------------------------------------------|------------------|
//! | `int    c_add(int a, int b)`                                | Addition         |
//! | `int    c_multiply(int a, int b)`                           | Multiplication   |
//! | `double c_distance(double x1, double y1, double x2, double y2)` | 2-D distance |
//! | `int    c_abs(int x)`                                       | Absolute value   |
//!
//! ## Your task
//!
//! 1. Fill in the `extern "C"` block with the correct Rust signatures.
//!    *Hint:* C `int` → `i32`, C `double` → `f64`.
//! 2. Implement each safe wrapper so it calls the C function inside
//!    `unsafe { }`.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex01
//! ```

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Declare the four C functions.  Replace the comments with real
// function signatures.  Example:
//
//     fn c_add(a: i32, b: i32) -> i32;
//
extern "C" {
    // fn c_add(???) -> ???;
    // fn c_multiply(???) -> ???;
    // fn c_distance(???) -> ???;
    // fn c_abs(???) -> ???;
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Each wrapper calls the matching C function inside `unsafe { }`.

/// Returns `a + b` (delegates to C).
pub fn add(a: i32, b: i32) -> i32 {
    todo!("Call c_add in an unsafe block")
}

/// Returns `a * b` (delegates to C).
pub fn multiply(a: i32, b: i32) -> i32 {
    todo!("Call c_multiply in an unsafe block")
}

/// Euclidean distance between `(x1,y1)` and `(x2,y2)`.
pub fn distance(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    todo!("Call c_distance in an unsafe block")
}

/// Absolute value of `x` (delegates to C).
pub fn abs(x: i32) -> i32 {
    todo!("Call c_abs in an unsafe block")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ex01_add() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(add(-1, 1), 0);
        assert_eq!(add(0, 0), 0);
    }

    #[test]
    fn test_ex01_multiply() {
        assert_eq!(multiply(3, 4), 12);
        assert_eq!(multiply(-2, 5), -10);
        assert_eq!(multiply(0, 100), 0);
    }

    #[test]
    fn test_ex01_distance() {
        let d = distance(0.0, 0.0, 3.0, 4.0);
        assert!((d - 5.0).abs() < 1e-10, "expected 5.0, got {d}");
    }

    #[test]
    fn test_ex01_distance_same_point() {
        let d = distance(1.0, 2.0, 1.0, 2.0);
        assert!((d - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_ex01_abs() {
        assert_eq!(abs(-42), 42);
        assert_eq!(abs(42), 42);
        assert_eq!(abs(0), 0);
    }
}
