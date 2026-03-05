// Standalone integration test for ex25 — bypasses test-mode
// compilation errors in other exercise modules.
//
// Run with:  cargo test --test ex25_test

use rust_ffi_exercises::ex25_cpp_virtual::*;
use std::f64::consts::PI;
use std::ffi::c_void;

// Re-declare introspection helpers (they're extern "C" in the
// static library but not re-exported by the Rust module).
extern "C" {
    fn cpp_vt_read_slot(interface_ptr: *const c_void, slot: i32) -> *const c_void;
    fn cpp_vt_offset_to_top(interface_ptr: *const c_void) -> isize;
    fn cpp_vt_sizeof_rect() -> usize;
    fn cpp_vt_sizeof_circle() -> usize;
    fn cpp_vt_ilabeled_offset_in_rect() -> usize;
    #[allow(dead_code)]
    fn cpp_vt_ilabeled_offset_in_circle() -> usize;
}

// ══════════════════════════════════════════════════════════════
// Basic virtual dispatch
// ══════════════════════════════════════════════════════════════

#[test]
fn test_rect_area_and_perimeter() {
    let r = CppShape::new_rect("R1", 0.0, 0.0, 3.0, 4.0);
    assert!((r.area() - 12.0).abs() < 1e-9);
    assert!((r.perimeter() - 14.0).abs() < 1e-9);
}

#[test]
fn test_circle_area_and_perimeter() {
    let c = CppShape::new_circle("C1", 0.0, 0.0, 5.0);
    assert!((c.area() - PI * 25.0).abs() < 1e-9);
    assert!((c.perimeter() - 2.0 * PI * 5.0).abs() < 1e-9);
}

// ══════════════════════════════════════════════════════════════
// Out-parameters: bounding_box
// ══════════════════════════════════════════════════════════════

#[test]
fn test_rect_bounding_box() {
    let r = CppShape::new_rect("BB", 1.0, 2.0, 10.0, 20.0);
    let (x, y, w, h) = r.bounding_box();
    assert!((x - 1.0).abs() < 1e-9);
    assert!((y - 2.0).abs() < 1e-9);
    assert!((w - 10.0).abs() < 1e-9);
    assert!((h - 20.0).abs() < 1e-9);
}

#[test]
fn test_circle_bounding_box() {
    let c = CppShape::new_circle("CB", 5.0, 5.0, 3.0);
    let (x, y, w, h) = c.bounding_box();
    assert!((x - 2.0).abs() < 1e-9);
    assert!((y - 2.0).abs() < 1e-9);
    assert!((w - 6.0).abs() < 1e-9);
    assert!((h - 6.0).abs() < 1e-9);
}

// ══════════════════════════════════════════════════════════════
// In/out-parameter: scale
// ══════════════════════════════════════════════════════════════

#[test]
fn test_scale_inout_rect() {
    let mut r = CppShape::new_rect("SR", 0.0, 0.0, 3.0, 4.0);
    assert!((r.area() - 12.0).abs() < 1e-9);
    let new_area = r.scale(2.0);
    // w=6, h=8 → area=48
    assert!((new_area - 48.0).abs() < 1e-9);
    assert!((r.area() - 48.0).abs() < 1e-9);
}

#[test]
fn test_scale_inout_circle() {
    let mut c = CppShape::new_circle("SC", 0.0, 0.0, 2.0);
    assert!((c.area() - PI * 4.0).abs() < 1e-9);
    let new_area = c.scale(3.0);
    // r=6 → area=π·36
    assert!((new_area - PI * 36.0).abs() < 1e-9);
}

// ══════════════════════════════════════════════════════════════
// ILabeled: get / set
// ══════════════════════════════════════════════════════════════

#[test]
fn test_label_get_set() {
    let mut obj = CppShape::new_rect("hello", 0.0, 0.0, 1.0, 1.0);
    assert_eq!(obj.get_label(), "hello");
    obj.set_label("world");
    assert_eq!(obj.get_label(), "world");
}

#[test]
fn test_circle_label() {
    let mut c = CppShape::new_circle("my_circle", 0.0, 0.0, 1.0);
    assert_eq!(c.get_label(), "my_circle");
    c.set_label("renamed");
    assert_eq!(c.get_label(), "renamed");
}

// ══════════════════════════════════════════════════════════════
// Multiple inheritance
// ══════════════════════════════════════════════════════════════

#[test]
fn test_mi_pointer_adjustment() {
    let r = CppShape::new_rect("MI", 0.0, 0.0, 1.0, 1.0);
    let h = r.raw_handle();
    let shape_addr = h.shape as usize;
    let labeled_addr = h.labeled as usize;
    assert_ne!(
        shape_addr, labeled_addr,
        "MI: shape and labeled should have different addresses"
    );
    let expected_offset = unsafe { cpp_vt_ilabeled_offset_in_rect() };
    assert_eq!(
        labeled_addr - shape_addr,
        expected_offset,
        "ILabeled should be at the expected offset from IShape"
    );
}

#[test]
fn test_offset_to_top() {
    let r = CppShape::new_rect("OTT", 0.0, 0.0, 1.0, 1.0);
    let h = r.raw_handle();
    // Primary base (IShape): offset_to_top == 0
    let ott_primary = unsafe { read_vtable_meta(h.shape).offset_to_top };
    assert_eq!(ott_primary, 0, "primary base offset_to_top should be 0");
    // Secondary base (ILabeled): offset_to_top == -(offset)
    let ott_secondary = r.ilabeled_offset_to_top();
    let expected = -(unsafe { cpp_vt_ilabeled_offset_in_rect() } as isize);
    assert_eq!(
        ott_secondary, expected,
        "secondary base offset_to_top should be negative"
    );
    // Cross-check with C++ introspection helper
    let cpp_ott = unsafe { cpp_vt_offset_to_top(h.labeled) };
    assert_eq!(
        ott_secondary, cpp_ott,
        "Rust-read and C++-read offset_to_top should match"
    );
}

// ══════════════════════════════════════════════════════════════
// Vtable slot verification
//
// Prove our #[repr(C)] vtable struct has correct slot numbering
// by comparing function pointers against what C++ reports via
// cpp_vt_read_slot.
// ══════════════════════════════════════════════════════════════

#[test]
fn test_vtable_slot_verification() {
    let obj = CppShape::new_rect("slots", 0.0, 0.0, 1.0, 1.0);
    let h = obj.raw_handle();
    unsafe {
        let vt = &*read_vptr::<IShapeVtable>(h.shape);
        assert_eq!(
            vt.area as *const (),
            cpp_vt_read_slot(h.shape, 2) as *const (),
            "IShape::area should be at slot 2"
        );
        assert_eq!(
            vt.perimeter as *const (),
            cpp_vt_read_slot(h.shape, 3) as *const (),
            "IShape::perimeter should be at slot 3"
        );
        assert_eq!(
            vt.bounding_box as *const (),
            cpp_vt_read_slot(h.shape, 4) as *const (),
            "IShape::bounding_box should be at slot 4"
        );
        assert_eq!(
            vt.scale as *const (),
            cpp_vt_read_slot(h.shape, 5) as *const (),
            "IShape::scale should be at slot 5"
        );

        let vl = &*read_vptr::<ILabeledVtable>(h.labeled);
        assert_eq!(
            vl.get_label as *const (),
            cpp_vt_read_slot(h.labeled, 2) as *const (),
            "ILabeled::get_label should be at slot 2"
        );
        assert_eq!(
            vl.set_label as *const (),
            cpp_vt_read_slot(h.labeled, 3) as *const (),
            "ILabeled::set_label should be at slot 3"
        );
    }
}

// ══════════════════════════════════════════════════════════════
// Polymorphic dispatch
// ══════════════════════════════════════════════════════════════

#[test]
fn test_polymorphic_dispatch() {
    let shapes: Vec<CppShape> = vec![
        CppShape::new_rect("R", 0.0, 0.0, 3.0, 4.0),
        CppShape::new_circle("C", 0.0, 0.0, 5.0),
    ];
    let total: f64 = shapes.iter().map(|s| s.area()).sum();
    let expected = 12.0 + PI * 25.0;
    assert!((total - expected).abs() < 1e-9);
}

// ══════════════════════════════════════════════════════════════
// Combined workflow
// ══════════════════════════════════════════════════════════════

#[test]
fn test_combined_workflow() {
    let mut obj = CppShape::new_rect("widget", 10.0, 20.0, 5.0, 3.0);

    // 1. Read initial state through virtual calls
    assert_eq!(obj.get_label(), "widget");
    assert!((obj.area() - 15.0).abs() < 1e-9);
    let (bx, by, bw, bh) = obj.bounding_box();
    assert!((bx - 10.0).abs() < 1e-9);
    assert!((by - 20.0).abs() < 1e-9);
    assert!((bw - 5.0).abs() < 1e-9);
    assert!((bh - 3.0).abs() < 1e-9);

    // 2. Mutate through virtual calls
    obj.set_label("scaled_widget");
    let new_area = obj.scale(2.0); // w=10, h=6

    // 3. Verify mutations took effect
    assert_eq!(obj.get_label(), "scaled_widget");
    assert!((new_area - 60.0).abs() < 1e-9);
    assert!((obj.area() - 60.0).abs() < 1e-9);
}

// ══════════════════════════════════════════════════════════════
// Object sizes
// ══════════════════════════════════════════════════════════════

#[test]
fn test_object_sizes() {
    let rect_size = unsafe { cpp_vt_sizeof_rect() };
    let circle_size = unsafe { cpp_vt_sizeof_circle() };
    // Rect:   2 vptrs(16) + 4 doubles(32) + label[64] = 112
    // Circle: 2 vptrs(16) + 3 doubles(24) + label[64] = 104
    assert_eq!(rect_size, 112, "LabeledRect should be 112 bytes");
    assert_eq!(circle_size, 104, "LabeledCircle should be 104 bytes");
}

// ══════════════════════════════════════════════════════════════
// Destruction via vtable deleting destructor
// ══════════════════════════════════════════════════════════════

#[test]
fn test_drop_via_vtable_destructor() {
    // Create and drop many objects — Drop calls the deleting
    // destructor through vtable slot 1, not an extern "C" wrapper.
    for _ in 0..1000 {
        let _ = CppShape::new_rect("d", 0.0, 0.0, 1.0, 1.0);
        let _ = CppShape::new_circle("d", 0.0, 0.0, 1.0);
    }
}
