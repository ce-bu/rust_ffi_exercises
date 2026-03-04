//! # Exercise 11: C++ Vtable Pattern
//!
//! **Concept:** C++ uses virtual function tables (vtables) for
//! polymorphism.  In FFI, this is represented as a **struct of
//! function pointers** — one pointer per "virtual method".
//!
//! A polymorphic object is then a pair:
//!
//! ```text
//! (data: *mut c_void,  vtable: *const VTable)
//! ```
//!
//! C/C++ code can call any "shape" uniformly through the vtable
//! without knowing the concrete type.
//!
//! ## Pre-provided (in `csrc/ex11_shapes.cpp`)
//!
//! ```cpp
//! double cpp_total_area(const CShape *shapes, size_t count);
//! void   cpp_destroy_shapes(CShape *shapes, size_t count);
//! ```
//!
//! These functions receive shapes from Rust and dispatch through
//! the vtable pointers.
//!
//! ## Your task
//!
//! 1. Implement `Circle` and `Rectangle` vtable functions.
//! 2. Write constructors that assemble a `CShape`.
//! 3. Call the C++ `cpp_total_area` function from Rust.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex11
//! ```

use std::f64::consts::PI;
use std::ffi::c_void;

// ── Pre-provided types (must match the C++ layout) ─────────────

/// A single "virtual method" table for shapes.
#[repr(C)]
pub struct ShapeVTable {
    pub area: extern "C" fn(data: *const c_void) -> f64,
    pub perimeter: extern "C" fn(data: *const c_void) -> f64,
    pub destroy: extern "C" fn(data: *mut c_void),
}

/// A shape = data pointer + vtable pointer.
#[repr(C)]
pub struct CShape {
    pub data: *mut c_void,
    pub vtable: *const ShapeVTable,
}

// Pre-provided extern declarations for the C++ helpers.
extern "C" {
    fn cpp_total_area(shapes: *const CShape, count: usize) -> f64;
    fn cpp_destroy_shapes(shapes: *mut CShape, count: usize);
}

// ══════════════════════════════════════════════════════════════
// Circle
// ══════════════════════════════════════════════════════════════

pub struct Circle {
    pub radius: f64,
}

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Implement the three vtable functions for Circle:
//
//   extern "C" fn circle_area(data: *const c_void) -> f64
//     → cast data back to *const Circle, compute π·r²
//
//   extern "C" fn circle_perimeter(data: *const c_void) -> f64
//     → 2·π·r
//
//   extern "C" fn circle_destroy(data: *mut c_void)
//     → cast to *mut Circle, reconstruct Box, drop
//
// Then create a static CIRCLE_VTABLE: ShapeVTable.

// extern "C" fn circle_area(data: *const c_void) -> f64 { todo!() }
// extern "C" fn circle_perimeter(data: *const c_void) -> f64 { todo!() }
// extern "C" fn circle_destroy(data: *mut c_void) { todo!() }

// static CIRCLE_VTABLE: ShapeVTable = ShapeVTable {
//     area: circle_area,
//     perimeter: circle_perimeter,
//     destroy: circle_destroy,
// };

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Create a CShape representing a Circle.
//
// Steps:
//   let circle = Box::new(Circle { radius });
//   let data = Box::into_raw(circle) as *mut c_void;
//   CShape { data, vtable: &CIRCLE_VTABLE }

/// Construct a Circle behind an opaque CShape.
pub fn shape_new_circle(radius: f64) -> CShape {
    todo!("Allocate Circle, return CShape with circle vtable")
}

// ══════════════════════════════════════════════════════════════
// Rectangle
// ══════════════════════════════════════════════════════════════

pub struct Rectangle {
    pub width: f64,
    pub height: f64,
}

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Same as TODO 1 but for Rectangle:
//   area = width × height
//   perimeter = 2 × (width + height)

// extern "C" fn rect_area(...) -> f64 { ... }
// extern "C" fn rect_perimeter(...) -> f64 { ... }
// extern "C" fn rect_destroy(...) { ... }
// static RECT_VTABLE: ShapeVTable = ...;

/// Construct a Rectangle behind an opaque CShape.
pub fn shape_new_rect(width: f64, height: f64) -> CShape {
    todo!("Allocate Rectangle, return CShape with rect vtable")
}

// ══════════════════════════════════════════════════════════════
// Dispatch helpers
// ══════════════════════════════════════════════════════════════

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Call through the vtable to get the area of a single shape.

/// Dispatch `area` through the vtable.
///
/// # Safety
/// `shape` must be a valid CShape with a valid vtable.
pub unsafe fn shape_area(shape: &CShape) -> f64 {
    todo!("Call (shape.vtable.area)(shape.data)")
}

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Destroy a shape by calling through its vtable.

/// Dispatch `destroy` through the vtable.
///
/// # Safety
/// `shape` must be a valid CShape.  After this call the shape is invalid.
pub unsafe fn shape_destroy(shape: &mut CShape) {
    todo!("Call (shape.vtable.destroy)(shape.data)")
}

// ── TODO 6 ─────────────────────────────────────────────────────
//
// Call the C++ function `cpp_total_area` with a slice of shapes.

/// Sum the areas of all shapes using the C++ dispatcher.
///
/// # Safety
/// All shapes must be valid.
pub unsafe fn total_area_via_cpp(shapes: &[CShape]) -> f64 {
    todo!("Call cpp_total_area(shapes.as_ptr(), shapes.len())")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ex11_circle_area() {
        let shape = shape_new_circle(3.0);
        let a = unsafe { shape_area(&shape) };
        assert!(
            (a - PI * 9.0).abs() < 1e-10,
            "expected ~{}, got {a}",
            PI * 9.0
        );
        let mut shape = shape;
        unsafe { shape_destroy(&mut shape) };
    }

    #[test]
    fn test_ex11_rect_area() {
        let shape = shape_new_rect(4.0, 5.0);
        let a = unsafe { shape_area(&shape) };
        assert!((a - 20.0).abs() < 1e-10);
        let mut shape = shape;
        unsafe { shape_destroy(&mut shape) };
    }

    #[test]
    fn test_ex11_cpp_total_area() {
        let mut shapes = vec![
            shape_new_circle(1.0),  // area = π
            shape_new_rect(2.0, 3.0), // area = 6
        ];
        let total = unsafe { total_area_via_cpp(&shapes) };
        assert!(
            (total - (PI + 6.0)).abs() < 1e-10,
            "expected ~{}, got {total}",
            PI + 6.0
        );
        // Clean up
        unsafe { cpp_destroy_shapes(shapes.as_mut_ptr(), shapes.len()) };
    }
}
