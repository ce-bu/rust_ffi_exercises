//! # Exercise 03: `#[repr(C)]` Struct Passing
//!
//! **Concept:** Share structured data between Rust and C by using
//! `#[repr(C)]` to guarantee a C-compatible memory layout.
//!
//! ## Background
//!
//! Rust's default struct layout is **unspecified** — the compiler may
//! reorder fields.  Adding `#[repr(C)]` pins the layout to match C's
//! rules (fields in declaration order, platform-specific alignment).
//!
//! Structs can be passed across FFI:
//! - **By value** — copied onto the stack (small structs).
//! - **By pointer** — `*const T` for read-only, `*mut T` for mutation.
//!
//! ## Your task
//!
//! 1. Add `#[repr(C)]` and useful derives to each struct below.
//! 2. Implement the six `extern "C"` functions.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex03
//! ```

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Add the correct attributes to make these structs FFI-safe:
//   #[repr(C)]
//   #[derive(Debug, Clone, Copy, PartialEq)]
//
// Without #[repr(C)] the layout is undefined and C code cannot
// safely read the fields.

pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub struct Size {
    pub width: f64,
    pub height: f64,
}

pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Implement these extern "C" functions.

/// Create a `Point` by value.
#[no_mangle]
pub extern "C" fn point_new(x: f64, y: f64) -> Point {
    todo!("Return a Point {{ x, y }}")
}

/// Add two points component-wise, return the result by value.
#[no_mangle]
pub extern "C" fn point_add(a: Point, b: Point) -> Point {
    todo!("Return Point {{ x: a.x + b.x, y: a.y + b.y }}")
}

/// Scale a point **in place** through a mutable pointer.
///
/// # Safety
/// `p` must be a valid, non-null, aligned pointer.
#[no_mangle]
pub unsafe extern "C" fn point_scale(p: *mut Point, factor: f64) {
    todo!("Multiply both x and y of *p by factor")
}

/// Construct a `Rect` from position and dimensions, returned by value.
#[no_mangle]
pub extern "C" fn rect_new(x: f64, y: f64, w: f64, h: f64) -> Rect {
    todo!("Return a Rect with origin (x,y) and size (w,h)")
}

/// Compute the area of a rectangle through a read-only pointer.
///
/// # Safety
/// `r` must be a valid, non-null pointer.
#[no_mangle]
pub unsafe extern "C" fn rect_area(r: *const Rect) -> f64 {
    todo!("Return width * height from the pointed-to Rect")
}

/// Return `true` if the rectangle contains the given point.
/// A point on the boundary counts as inside.
///
/// # Safety
/// Both pointers must be valid and non-null.
#[no_mangle]
pub unsafe extern "C" fn rect_contains(r: *const Rect, p: *const Point) -> bool {
    todo!("Check if p is within the bounds of r")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ex03_point_new() {
        let p = point_new(1.0, 2.0);
        assert_eq!(p.x, 1.0);
        assert_eq!(p.y, 2.0);
    }

    #[test]
    fn test_ex03_point_add() {
        let a = point_new(1.0, 2.0);
        let b = point_new(3.0, 4.0);
        let c = point_add(a, b);
        assert_eq!(c.x, 4.0);
        assert_eq!(c.y, 6.0);
    }

    #[test]
    fn test_ex03_point_scale() {
        let mut p = point_new(3.0, 4.0);
        unsafe { point_scale(&mut p, 2.0) };
        assert_eq!(p.x, 6.0);
        assert_eq!(p.y, 8.0);
    }

    #[test]
    fn test_ex03_rect_area() {
        let r = rect_new(0.0, 0.0, 5.0, 3.0);
        let a = unsafe { rect_area(&r) };
        assert!((a - 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_ex03_rect_contains_inside() {
        let r = rect_new(0.0, 0.0, 10.0, 10.0);
        let p = point_new(5.0, 5.0);
        assert!(unsafe { rect_contains(&r, &p) });
    }

    #[test]
    fn test_ex03_rect_contains_boundary() {
        let r = rect_new(0.0, 0.0, 10.0, 10.0);
        let p = point_new(10.0, 10.0);
        assert!(unsafe { rect_contains(&r, &p) });
    }

    #[test]
    fn test_ex03_rect_contains_outside() {
        let r = rect_new(0.0, 0.0, 10.0, 10.0);
        let p = point_new(11.0, 5.0);
        assert!(!unsafe { rect_contains(&r, &p) });
    }
}
