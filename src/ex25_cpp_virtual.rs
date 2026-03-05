//! # Exercise 25: Calling C++ Virtual Functions — Direct Vtable Access
//!
//! **Concept:** Instead of writing `extern "C"` trampoline functions
//! for every virtual method, Rust reads the **vtable pointer** (vptr)
//! directly from the C++ object and calls the function pointer at the
//! correct slot index.  This exploits the **Itanium C++ ABI** vtable
//! layout, which is used by GCC and Clang on Linux, macOS, and BSDs.
//!
//! ## How it works
//!
//! ```text
//!  C++ object in memory          Vtable (compiler-generated)
//! ┌──────────────────┐          ┌────────────────────────────┐
//! │ vptr ─────────────┼────────▶│ [−2] offset_to_top         │
//! │                  │          │ [−1] typeinfo pointer       │
//! │ ... fields ...   │          │ [ 0] dtor_complete  (D1)   │
//! └──────────────────┘          │ [ 1] dtor_deleting  (D0)   │
//!                               │ [ 2] area()                │
//!  Rust reads vptr,             │ [ 3] perimeter()           │
//!  indexes slot [2],            │ [ 4] bounding_box()        │
//!  calls fn(this)               │ [ 5] scale()               │
//!                               └────────────────────────────┘
//! ```
//!
//! ## Multiple inheritance
//!
//! ```text
//!  LabeledRect object layout      (IShape is primary base)
//! ┌────────────────────────┐
//! │ offset  0: vptr_IShape ─────▶ IShape vtable (6 slots)
//! │ offset  8: vptr_ILabeled ───▶ ILabeled vtable (4 slots)
//! │ offset 16: x_, y_, w_, h_  │
//! │ offset 48: label_[64]      │
//! └────────────────────────┘
//!
//! shape_ptr   == object address         (primary, no adjustment)
//! labeled_ptr == object address + 8     (secondary, adjusted!)
//! ```
//!
//! ## In/out arguments
//!
//! - **Out-params:** `bounding_box(out_x, out_y, out_w, out_h)`,
//!   `get_label(out_buf, buf_len, out_len)`
//! - **In/out-param:** `scale(factor, inout_area)` — caller passes
//!   current area in; C++ writes new area back.
//!
//! ## What the C++ side provides (NO trampolines)
//!
//! - Factory functions to create objects (`cpp_vt_create_rect`, etc.)
//! - Introspection helpers for test verification
//! - **Zero** extern "C" functions that wrap virtual methods
//!
//! ## Portability
//!
//! This technique is specific to the **Itanium C++ ABI**.  It does
//! **not** work with MSVC.  See `docs/ex25_cpp_vtable_abi.md` for
//! full details.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex25
//! ```

use std::ffi::{c_char, c_void, CString};

// ══════════════════════════════════════════════════════════════
// Itanium ABI vtable structures
//
// Each struct mirrors the vtable layout for one C++ interface.
// Fields are in declaration order; virtual destructors occupy
// the first two slots (complete-object D1, deleting D0).
//
// On x86-64 Linux the C++ and C calling conventions are
// identical (SysV ABI), so `extern "C"` function pointers
// are correct for calling C++ virtual methods with `this`
// as the first argument.
// ══════════════════════════════════════════════════════════════

/// Vtable layout for `IShape` (Itanium ABI).
///
/// | Slot | Method                         | Signature (C view)                                         |
/// |------|--------------------------------|------------------------------------------------------------|
/// |  0   | complete destructor (D1)       | `void (*)(IShape *this)`                                   |
/// |  1   | deleting destructor (D0)       | `void (*)(IShape *this)`                                   |
/// |  2   | `area() const`                 | `double (*)(const IShape *this)`                           |
/// |  3   | `perimeter() const`            | `double (*)(const IShape *this)`                           |
/// |  4   | `bounding_box(...) const`      | `void (*)(const IShape*, double*, double*, double*, double*)` |
/// |  5   | `scale(factor, inout_area)`    | `void (*)(IShape*, double, double*)`                       |
#[repr(C)]
pub struct IShapeVtable {
    pub dtor_complete: unsafe extern "C" fn(this: *mut c_void),
    pub dtor_deleting: unsafe extern "C" fn(this: *mut c_void),
    pub area: unsafe extern "C" fn(this: *const c_void) -> f64,
    pub perimeter: unsafe extern "C" fn(this: *const c_void) -> f64,
    pub bounding_box: unsafe extern "C" fn(
        this: *const c_void,
        out_x: *mut f64,
        out_y: *mut f64,
        out_w: *mut f64,
        out_h: *mut f64,
    ),
    pub scale: unsafe extern "C" fn(this: *mut c_void, factor: f64, inout_area: *mut f64),
}

/// Vtable layout for `ILabeled` (Itanium ABI).
///
/// | Slot | Method                    | Signature (C view)                                            |
/// |------|---------------------------|---------------------------------------------------------------|
/// |  0   | complete destructor (D1)  | `void (*)(ILabeled *this)`                                    |
/// |  1   | deleting destructor (D0)  | `void (*)(ILabeled *this)`                                    |
/// |  2   | `get_label(...) const`    | `int (*)(const ILabeled*, char*, size_t, size_t*)`            |
/// |  3   | `set_label(s)`            | `void (*)(ILabeled*, const char*)`                            |
#[repr(C)]
pub struct ILabeledVtable {
    pub dtor_complete: unsafe extern "C" fn(this: *mut c_void),
    pub dtor_deleting: unsafe extern "C" fn(this: *mut c_void),
    pub get_label: unsafe extern "C" fn(
        this: *const c_void,
        out_buf: *mut c_char,
        buf_len: usize,
        out_len: *mut usize,
    ) -> i32,
    pub set_label: unsafe extern "C" fn(this: *mut c_void, new_label: *const c_char),
}

/// Metadata that precedes the function-pointer array in an
/// Itanium ABI vtable.  Located at *negative* offsets from
/// where the vptr points.
///
/// ```text
///   vptr − 16 : offset_to_top   (ptrdiff_t)
///   vptr − 8  : typeinfo_ptr    (*const void)
///   vptr + 0  : first fn ptr    ← vptr points here
/// ```
#[repr(C)]
pub struct VtableMeta {
    pub offset_to_top: isize,
    pub typeinfo: *const c_void,
}

// ══════════════════════════════════════════════════════════════
// Handle returned by the C++ factory
// ══════════════════════════════════════════════════════════════

/// Dual-interface handle.
///
/// * `shape`   — `IShape*` (primary base = allocation address)
/// * `labeled` — `ILabeled*` (secondary base, pointer-adjusted)
#[repr(C)]
pub struct CppDualHandle {
    pub shape: *mut c_void,
    pub labeled: *mut c_void,
}

// ══════════════════════════════════════════════════════════════
// Minimal extern "C" — factories + introspection only
// ══════════════════════════════════════════════════════════════

extern "C" {
    fn cpp_vt_create_rect(label: *const c_char, x: f64, y: f64, w: f64, h: f64) -> CppDualHandle;

    fn cpp_vt_create_circle(label: *const c_char, cx: f64, cy: f64, r: f64) -> CppDualHandle;

    // Introspection (for test verification, not for production use)
    #[allow(dead_code)]
    fn cpp_vt_read_slot(interface_ptr: *const c_void, slot: i32) -> *const c_void;
    #[allow(dead_code)]
    fn cpp_vt_offset_to_top(interface_ptr: *const c_void) -> isize;
    #[allow(dead_code)]
    fn cpp_vt_sizeof_rect() -> usize;
    #[allow(dead_code)]
    fn cpp_vt_sizeof_circle() -> usize;
    #[allow(dead_code)]
    fn cpp_vt_ilabeled_offset_in_rect() -> usize;
    #[allow(dead_code)]
    fn cpp_vt_ilabeled_offset_in_circle() -> usize;
}

// ══════════════════════════════════════════════════════════════
// Low-level helpers for reading vtable pointers
// ══════════════════════════════════════════════════════════════

/// Read the vptr from the first pointer-sized slot of a C++
/// sub-object and return it as a typed vtable reference.
///
/// # Safety
///
/// `subobject` must point to a valid, live C++ object (or
/// sub-object) whose first field is a vptr.
#[inline]
pub unsafe fn read_vptr<V>(subobject: *const c_void) -> *const V {
    *(subobject as *const *const V)
}

/// Read the `VtableMeta` (offset_to_top + typeinfo) that
/// precedes the function-pointer array in the Itanium vtable.
///
/// # Safety
///
/// `subobject` must point to a valid C++ sub-object with a vptr.
#[inline]
pub unsafe fn read_vtable_meta(subobject: *const c_void) -> &'static VtableMeta {
    let vptr = *(subobject as *const *const VtableMeta);
    // VtableMeta is 16 bytes; it sits immediately before vptr.
    &*vptr.offset(-1)
}

// ══════════════════════════════════════════════════════════════
// Safe RAII wrapper — all virtual calls go through the vtable
// ══════════════════════════════════════════════════════════════

/// Owns a C++ object that implements both `IShape` and `ILabeled`.
///
/// Virtual method calls read the vptr and index into the vtable
/// directly — **no extern "C" trampolines**.
///
/// `Drop` calls the deleting destructor through the primary
/// base's vtable (slot 1), which both runs the C++ destructor
/// and deallocates the memory.
pub struct CppShape {
    handle: CppDualHandle,
}

impl CppShape {
    // ── Constructors ─────────────────────────────────────────

    pub fn new_rect(label: &str, x: f64, y: f64, w: f64, h: f64) -> Self {
        let c_label = CString::new(label).expect("label must not contain NUL");
        let handle = unsafe { cpp_vt_create_rect(c_label.as_ptr(), x, y, w, h) };
        Self { handle }
    }

    pub fn new_circle(label: &str, cx: f64, cy: f64, r: f64) -> Self {
        let c_label = CString::new(label).expect("label must not contain NUL");
        let handle = unsafe { cpp_vt_create_circle(c_label.as_ptr(), cx, cy, r) };
        Self { handle }
    }

    // ── IShape virtual calls (via primary vptr) ──────────────

    /// `IShape::area() const` — vtable slot 2.
    pub fn area(&self) -> f64 {
        unsafe {
            let vt = &*read_vptr::<IShapeVtable>(self.handle.shape);
            (vt.area)(self.handle.shape)
        }
    }

    /// `IShape::perimeter() const` — vtable slot 3.
    pub fn perimeter(&self) -> f64 {
        unsafe {
            let vt = &*read_vptr::<IShapeVtable>(self.handle.shape);
            (vt.perimeter)(self.handle.shape)
        }
    }

    /// `IShape::bounding_box(...)` — vtable slot 4.
    ///
    /// Out-parameter example: four `*mut f64` written by C++.
    pub fn bounding_box(&self) -> (f64, f64, f64, f64) {
        let (mut x, mut y, mut w, mut h) = (0.0f64, 0.0, 0.0, 0.0);
        unsafe {
            let vt = &*read_vptr::<IShapeVtable>(self.handle.shape);
            (vt.bounding_box)(self.handle.shape, &mut x, &mut y, &mut w, &mut h);
        }
        (x, y, w, h)
    }

    /// `IShape::scale(factor, inout_area)` — vtable slot 5.
    ///
    /// In/out-parameter example: we pass the current area in
    /// `*inout_area`; C++ writes the new area back.
    pub fn scale(&mut self, factor: f64) -> f64 {
        let mut area = self.area(); // "in" value
        unsafe {
            let vt = &*read_vptr::<IShapeVtable>(self.handle.shape);
            (vt.scale)(self.handle.shape, factor, &mut area);
        }
        area // "out" value written by C++
    }

    // ── ILabeled virtual calls (via secondary vptr) ──────────
    //
    // IMPORTANT: We pass `self.handle.labeled` as `this`, NOT
    // `self.handle.shape`.  The secondary vtable expects the
    // ILabeled sub-object pointer.

    /// `ILabeled::get_label(...)` — vtable slot 2.
    ///
    /// Out-parameter example: C++ writes the label string and
    /// its length into caller-provided buffers.
    pub fn get_label(&self) -> String {
        let mut buf = [0u8; 128];
        let mut len: usize = 0;
        let rc = unsafe {
            let vt = &*read_vptr::<ILabeledVtable>(self.handle.labeled);
            (vt.get_label)(
                self.handle.labeled,
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
                &mut len,
            )
        };
        if rc == 0 {
            String::from_utf8_lossy(&buf[..len]).into_owned()
        } else {
            String::from("<error>")
        }
    }

    /// `ILabeled::set_label(s)` — vtable slot 3.
    pub fn set_label(&mut self, new_label: &str) {
        let c = CString::new(new_label).expect("label must not contain NUL");
        unsafe {
            let vt = &*read_vptr::<ILabeledVtable>(self.handle.labeled);
            (vt.set_label)(self.handle.labeled, c.as_ptr());
        }
    }

    // ── Diagnostics ──────────────────────────────────────────

    /// Returns `true` when the two interface pointers have
    /// different addresses — demonstrating MI pointer adjustment.
    pub fn interface_pointers_differ(&self) -> bool {
        self.handle.shape != self.handle.labeled
    }

    /// Read the `offset_to_top` metadata from the ILabeled
    /// (secondary) vtable.  Should be negative, indicating
    /// how far back to the complete object.
    pub fn ilabeled_offset_to_top(&self) -> isize {
        unsafe { read_vtable_meta(self.handle.labeled).offset_to_top }
    }

    /// Returns the raw handle for advanced inspection.
    pub fn raw_handle(&self) -> &CppDualHandle {
        &self.handle
    }
}

impl Drop for CppShape {
    fn drop(&mut self) {
        // Call the **deleting destructor** (slot 1) through the
        // primary base (IShape) vtable.  This both runs the full
        // C++ destructor chain and calls `operator delete`.
        //
        // We use `shape` (primary base pointer) which equals the
        // original allocation address — so `delete` is correct.
        unsafe {
            let vt = &*read_vptr::<IShapeVtable>(self.handle.shape);
            (vt.dtor_deleting)(self.handle.shape);
        }
    }
}

// ══════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    // ── Vtable layout verification ───────────────────────────
    //
    // These tests compare the function pointer Rust reads from
    // our `#[repr(C)]` vtable struct against the pointer the C++
    // introspection helper reads from the same slot — proving
    // our slot numbering is correct.

    #[test]
    fn test_ishape_vtable_slot_area() {
        let obj = CppShape::new_rect("slot_test", 0.0, 0.0, 1.0, 1.0);
        let vt = unsafe { &*read_vptr::<IShapeVtable>(obj.handle.shape) };
        let rust_fn = vt.area as *const ();
        let cpp_fn = unsafe { cpp_vt_read_slot(obj.handle.shape, 2) };
        assert_eq!(
            rust_fn, cpp_fn as *const (),
            "IShapeVtable.area should be at slot 2"
        );
    }

    #[test]
    fn test_ishape_vtable_slot_perimeter() {
        let obj = CppShape::new_rect("slot_test", 0.0, 0.0, 1.0, 1.0);
        let vt = unsafe { &*read_vptr::<IShapeVtable>(obj.handle.shape) };
        let rust_fn = vt.perimeter as *const ();
        let cpp_fn = unsafe { cpp_vt_read_slot(obj.handle.shape, 3) };
        assert_eq!(
            rust_fn, cpp_fn as *const (),
            "IShapeVtable.perimeter should be at slot 3"
        );
    }

    #[test]
    fn test_ishape_vtable_slot_bounding_box() {
        let obj = CppShape::new_rect("slot_test", 0.0, 0.0, 1.0, 1.0);
        let vt = unsafe { &*read_vptr::<IShapeVtable>(obj.handle.shape) };
        let rust_fn = vt.bounding_box as *const ();
        let cpp_fn = unsafe { cpp_vt_read_slot(obj.handle.shape, 4) };
        assert_eq!(
            rust_fn, cpp_fn as *const (),
            "IShapeVtable.bounding_box should be at slot 4"
        );
    }

    #[test]
    fn test_ishape_vtable_slot_scale() {
        let obj = CppShape::new_rect("slot_test", 0.0, 0.0, 1.0, 1.0);
        let vt = unsafe { &*read_vptr::<IShapeVtable>(obj.handle.shape) };
        let rust_fn = vt.scale as *const ();
        let cpp_fn = unsafe { cpp_vt_read_slot(obj.handle.shape, 5) };
        assert_eq!(
            rust_fn, cpp_fn as *const (),
            "IShapeVtable.scale should be at slot 5"
        );
    }

    #[test]
    fn test_ilabeled_vtable_slot_get_label() {
        let obj = CppShape::new_rect("slot_test", 0.0, 0.0, 1.0, 1.0);
        let vt = unsafe { &*read_vptr::<ILabeledVtable>(obj.handle.labeled) };
        let rust_fn = vt.get_label as *const ();
        let cpp_fn = unsafe { cpp_vt_read_slot(obj.handle.labeled, 2) };
        assert_eq!(
            rust_fn, cpp_fn as *const (),
            "ILabeledVtable.get_label should be at slot 2"
        );
    }

    #[test]
    fn test_ilabeled_vtable_slot_set_label() {
        let obj = CppShape::new_rect("slot_test", 0.0, 0.0, 1.0, 1.0);
        let vt = unsafe { &*read_vptr::<ILabeledVtable>(obj.handle.labeled) };
        let rust_fn = vt.set_label as *const ();
        let cpp_fn = unsafe { cpp_vt_read_slot(obj.handle.labeled, 3) };
        assert_eq!(
            rust_fn, cpp_fn as *const (),
            "ILabeledVtable.set_label should be at slot 3"
        );
    }

    // ── Basic virtual dispatch ───────────────────────────────

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

    // ── Out-parameters: bounding_box ─────────────────────────

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

    // ── In/out parameter: scale ──────────────────────────────

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
        let old = c.area();
        assert!((old - PI * 4.0).abs() < 1e-9);
        let new_area = c.scale(3.0);
        // r=6 → area = π·36
        assert!((new_area - PI * 36.0).abs() < 1e-9);
    }

    // ── ILabeled: get / set ──────────────────────────────────

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

    // ── Multiple inheritance ─────────────────────────────────

    #[test]
    fn test_mi_pointer_adjustment() {
        let r = CppShape::new_rect("MI", 0.0, 0.0, 1.0, 1.0);
        // ILabeled sub-object is at +8 from the primary base.
        assert!(
            r.interface_pointers_differ(),
            "shape and labeled should have different addresses (MI)"
        );

        let shape_addr = r.handle.shape as usize;
        let labeled_addr = r.handle.labeled as usize;
        let expected_offset = unsafe { cpp_vt_ilabeled_offset_in_rect() };
        assert_eq!(
            labeled_addr - shape_addr,
            expected_offset,
            "ILabeled should be at the expected offset from IShape"
        );
    }

    #[test]
    fn test_offset_to_top_primary() {
        let r = CppShape::new_rect("OTT", 0.0, 0.0, 1.0, 1.0);
        // Primary base (IShape): offset_to_top == 0
        let ott = unsafe { read_vtable_meta(r.handle.shape).offset_to_top };
        assert_eq!(ott, 0, "primary base offset_to_top should be 0");
    }

    #[test]
    fn test_offset_to_top_secondary() {
        let r = CppShape::new_rect("OTT2", 0.0, 0.0, 1.0, 1.0);
        // Secondary base (ILabeled): offset_to_top == -(offset of ILabeled)
        let ott = r.ilabeled_offset_to_top();
        let expected = -(unsafe { cpp_vt_ilabeled_offset_in_rect() } as isize);
        assert_eq!(
            ott, expected,
            "secondary base offset_to_top should be negative"
        );

        // Also verify with the C++ introspection helper:
        let cpp_ott = unsafe { cpp_vt_offset_to_top(r.handle.labeled) };
        assert_eq!(
            ott, cpp_ott,
            "Rust-read and C++-read offset_to_top should match"
        );
    }

    // ── Polymorphic dispatch ─────────────────────────────────

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

    // ── Combined workflow ────────────────────────────────────

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

        // 3. Read back — verify changes took effect
        assert_eq!(obj.get_label(), "scaled_widget");
        assert!((new_area - 60.0).abs() < 1e-9);
        assert!((obj.area() - 60.0).abs() < 1e-9);
        let (_, _, bw2, bh2) = obj.bounding_box();
        assert!((bw2 - 10.0).abs() < 1e-9);
        assert!((bh2 - 6.0).abs() < 1e-9);
    }

    // ── Destruction via vtable ───────────────────────────────

    #[test]
    fn test_drop_via_vtable_deleting_destructor() {
        // Verifies that creating and dropping many objects doesn't
        // leak or crash — Drop calls the deleting destructor
        // through the vtable, not an extern "C" trampoline.
        for _ in 0..1000 {
            let _ = CppShape::new_rect("d", 0.0, 0.0, 1.0, 1.0);
            let _ = CppShape::new_circle("d", 0.0, 0.0, 1.0);
        }
    }

    // ── Size verification ────────────────────────────────────

    #[test]
    fn test_object_sizes() {
        let rect_size = unsafe { cpp_vt_sizeof_rect() };
        let circle_size = unsafe { cpp_vt_sizeof_circle() };
        // Both have: 2 vptrs (16) + data + label[64]
        // Rect:   16 + 4×8(doubles) + 64 = 112
        // Circle: 16 + 3×8(doubles) + 64 = 104
        assert_eq!(rect_size, 112, "LabeledRect should be 112 bytes");
        assert_eq!(circle_size, 104, "LabeledCircle should be 104 bytes");
    }
}
