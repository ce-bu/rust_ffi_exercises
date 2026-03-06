//! # Exercise 29: `UnsafeCell` and Aliasing in FFI
//!
//! **Concept:** Rust tells LLVM that `&T` references are `noalias
//! readonly` — meaning the memory behind them will not change.  LLVM
//! uses this to cache loads, reorder reads, and eliminate "redundant"
//! accesses.  When C code mutates data that Rust holds as `&T`,
//! these optimizations produce **wrong results**.
//!
//! `UnsafeCell<T>` is the escape hatch: it tells the compiler that
//! the memory *may* change through shared references, disabling
//! the `noalias readonly` annotation.
//!
//! ## Your task
//!
//!  A. Fix a struct where C mutates a field behind `&T` (aliasing UB).
//!  B. Build a safe wrapper for a C "sensor" that updates in-place.
//!  C. Use atomics (which contain `UnsafeCell`) for cross-language
//!     concurrent counters.
//!  D. Demonstrate the problem: show that without `UnsafeCell`,
//!     reads can be cached (simulated via a helper).
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex29
//! ```
//!
//! ## See also
//!
//! `docs/ex29_unsafecell_aliasing.md` — full explanation of LLVM's
//! `noalias`, Stacked Borrows, and when `UnsafeCell` is needed.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicI32, Ordering};

// ══════════════════════════════════════════════════════════════
// Simulated C functions (pure Rust — keeps the exercise
// self-contained and Miri-compatible)
// ══════════════════════════════════════════════════════════════

/// Simulates a C function that writes a new value through a raw
/// pointer.  In real FFI this would be `extern "C"`.
///
/// # Safety
/// `ptr` must be valid and writable.
pub unsafe fn c_update_value(ptr: *mut i32, new_val: i32) {
    ptr.write(new_val);
}

/// Simulates a C sensor read cycle: writes `reading` to the
/// sensor's `value` field.
///
/// # Safety
/// `sensor_ptr` must point to a valid `Sensor` (Part B).
pub unsafe fn c_sensor_tick(sensor_ptr: *mut i32, reading: i32) {
    sensor_ptr.write(reading);
}

/// Simulates a C function that atomically increments a counter.
///
/// # Safety
/// `counter` must point to a valid `AtomicI32`.
pub unsafe fn c_atomic_increment(counter: *const AtomicI32, amount: i32) {
    (*counter).fetch_add(amount, Ordering::SeqCst);
}


// ══════════════════════════════════════════════════════════════
// Part A — The aliasing problem (fix the struct)
// ══════════════════════════════════════════════════════════════

// ── TODO 1 ─────────────────────────────────────────────────────
//
// `BrokenDevice` has a field `status` that C updates through a raw
// pointer while Rust holds `&BrokenDevice`.  This is UB because
// LLVM assumes `&BrokenDevice` means `status` won't change.
//
// Your task: define `FixedDevice` where the `status` field is
// wrapped in `UnsafeCell<u32>`, making it safe for C to mutate.
//
// Also implement:
//   - `FixedDevice::new(id, status)` → constructor
//   - `FixedDevice::status(&self) -> u32` → reads the status safely
//   - `FixedDevice::status_ptr(&self) -> *mut u32` → for C to write to
//
// Hint:
//   #[repr(C)]
//   pub struct FixedDevice {
//       pub id: u32,
//       pub status: UnsafeCell<u32>,
//   }
//
//   impl FixedDevice {
//       pub fn status(&self) -> u32 {
//           unsafe { *self.status.get() }
//       }
//       pub fn status_ptr(&self) -> *mut u32 {
//           self.status.get()
//       }
//   }

/// Broken version — DO NOT USE in real code.  Shown for contrast.
#[repr(C)]
pub struct BrokenDevice {
    pub id: u32,
    pub status: u32,   // ← NOT UnsafeCell — UB if C writes while &self exists
}

// TODO: Define FixedDevice with UnsafeCell<u32> for status,
//       and implement new(), status(), status_ptr().


// ══════════════════════════════════════════════════════════════
// Part B — Safe sensor wrapper
// ══════════════════════════════════════════════════════════════

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Build a `Sensor` that wraps a C-updated value.
//
// The sensor has:
//   - `name: String`         (immutable — no UnsafeCell needed)
//   - `value: UnsafeCell<i32>` (C writes new readings here)
//   - `read_count: UnsafeCell<u64>` (tracks how many times Rust read)
//
// Implement:
//   - `Sensor::new(name: &str, initial: i32) -> Self`
//   - `Sensor::read(&self) -> i32`
//       Reads the value AND increments read_count.
//       Must use unsafe { *self.value.get() } since it's UnsafeCell.
//   - `Sensor::read_count(&self) -> u64`
//   - `Sensor::value_ptr(&self) -> *mut i32`
//       Returns a raw pointer C can write to.
//
// The key insight: `read(&self)` takes a SHARED reference, but
// can still update `read_count` because it's in UnsafeCell.
// This is the interior mutability pattern.

#[repr(C)]
pub struct Sensor {
    // TODO: define fields
}

impl Sensor {
    pub fn new(name: &str, initial: i32) -> Self {
        todo!("TODO 2: construct Sensor")
    }

    /// Read the current sensor value.
    /// Also increments the internal read counter.
    pub fn read(&self) -> i32 {
        todo!("TODO 2: read *self.value.get(), increment *self.read_count.get()")
    }

    /// How many times has `read()` been called?
    pub fn read_count(&self) -> u64 {
        todo!("TODO 2: return *self.read_count.get()")
    }

    /// Pointer to the value field, for C to write into.
    pub fn value_ptr(&self) -> *mut i32 {
        todo!("TODO 2: return self.value.get()")
    }
}


// ══════════════════════════════════════════════════════════════
// Part C — Atomic counters shared with C
// ══════════════════════════════════════════════════════════════

// ── TODO 3 ─────────────────────────────────────────────────────
//
// `AtomicI32` is `#[repr(transparent)]` over `UnsafeCell<i32>`,
// which means it is layout-compatible with C's `_Atomic int32_t`.
//
// Build a `SharedCounters` struct with two atomic counters:
//   - `requests: AtomicI32`
//   - `errors: AtomicI32`
//
// Mark it `#[repr(C)]` so C can access the fields by offset.
//
// Implement:
//   - `SharedCounters::new() -> Self`  (both start at 0)
//   - `SharedCounters::requests(&self) -> i32`
//   - `SharedCounters::errors(&self) -> i32`
//   - `SharedCounters::inc_requests(&self)`  — atomic increment
//   - `SharedCounters::inc_errors(&self)`    — atomic increment
//   - `SharedCounters::requests_ptr(&self) -> *const AtomicI32`
//       For C to call atomic operations on.
//
// Note: all methods take `&self` (shared reference), but
// mutation is safe because AtomicI32 uses UnsafeCell internally.

#[repr(C)]
pub struct SharedCounters {
    // TODO: define fields
}

impl SharedCounters {
    pub fn new() -> Self {
        todo!("TODO 3: construct SharedCounters")
    }

    pub fn requests(&self) -> i32 {
        todo!("TODO 3: load self.requests atomically")
    }

    pub fn errors(&self) -> i32 {
        todo!("TODO 3: load self.errors atomically")
    }

    pub fn inc_requests(&self) {
        todo!("TODO 3: fetch_add(1) on requests")
    }

    pub fn inc_errors(&self) {
        todo!("TODO 3: fetch_add(1) on errors")
    }

    /// Raw pointer to the requests counter, for C atomic ops.
    pub fn requests_ptr(&self) -> *const AtomicI32 {
        todo!("TODO 3: return pointer to self.requests")
    }
}


// ══════════════════════════════════════════════════════════════
// Part D — Demonstrating the caching problem
// ══════════════════════════════════════════════════════════════

// ── TODO 4 ─────────────────────────────────────────────────────
//
// This part demonstrates WHY UnsafeCell matters by showing the
// difference in behavior.
//
// `observe_without_unsafecell` reads a value, calls a mutating
// function through a raw pointer, then reads again.  Without
// UnsafeCell, the compiler MAY return the same value both times
// (though in debug mode it usually doesn't — the bug is subtle).
//
// `observe_with_unsafecell` does the same through UnsafeCell.
// The compiler MUST reload, so the second read reflects the change.
//
// Implement both functions.  We test correctness rather than
// optimization behavior (since tests run in debug mode), but
// the doc comments explain what would happen in release.

/// Read a value, mutate it through a raw pointer derived from
/// `UnsafeCell::get()`, read again.  Returns (before, after).
///
/// With UnsafeCell this is well-defined: the compiler does not
/// assume the value is unchanged after the write.
///
/// # Safety
/// This function uses internal unsafe operations on UnsafeCell.
pub fn observe_with_unsafecell() -> (i32, i32) {
    let cell = UnsafeCell::new(10);
    let before = unsafe { *cell.get() };

    // Mutate through UnsafeCell's raw pointer — this is legal.
    unsafe { *cell.get() = 42; }

    let after = unsafe { *cell.get() };
    (before, after)  // Always (10, 42) — well-defined
}

/// Same idea, but using a plain value + raw pointer.
/// This version documents the WRONG approach.
///
/// In a real codebase, mutating `*ptr` while `&val` exists would
/// be UB.  Here we do it safely by not holding a reference across
/// the mutation — but the function illustrates the pattern you'd
/// see in buggy FFI code.
///
/// Returns (before, after).
pub fn observe_without_unsafecell() -> (i32, i32) {
    let mut val: i32 = 10;
    let before = val;

    // In real buggy FFI code, this mutation would happen through C
    // while a &T reference exists — which is UB without UnsafeCell.
    // Here we mutate directly to be safe in this demo.
    let ptr: *mut i32 = &mut val;
    unsafe { *ptr = 42; }

    let after = val;
    (before, after)  // (10, 42) in this safe version
}


// ══════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Part A tests ───────────────────────────────────────────

    #[test]
    fn test_ex29_fixed_device_new() {
        let dev = FixedDevice::new(1, 0xFF);
        assert_eq!(dev.id, 1);
        assert_eq!(dev.status(), 0xFF);
    }

    #[test]
    fn test_ex29_fixed_device_c_updates() {
        let dev = FixedDevice::new(1, 0);
        assert_eq!(dev.status(), 0);

        // Simulate C writing to the status field
        unsafe { c_update_value(dev.status_ptr(), 42); }
        assert_eq!(dev.status(), 42);

        // C writes again
        unsafe { c_update_value(dev.status_ptr(), 99); }
        assert_eq!(dev.status(), 99);
    }

    #[test]
    fn test_ex29_fixed_device_id_unchanged() {
        let dev = FixedDevice::new(7, 0);
        unsafe { c_update_value(dev.status_ptr(), 123); }
        // id should not be affected
        assert_eq!(dev.id, 7);
    }

    // ── Part B tests ───────────────────────────────────────────

    #[test]
    fn test_ex29_sensor_new() {
        let s = Sensor::new("temp", 25);
        assert_eq!(s.read(), 25);
        assert_eq!(s.read_count(), 1); // read() was called once
    }

    #[test]
    fn test_ex29_sensor_c_updates_value() {
        let s = Sensor::new("pressure", 100);
        assert_eq!(s.read(), 100);

        // C writes a new reading
        unsafe { c_sensor_tick(s.value_ptr(), 200); }
        assert_eq!(s.read(), 200);

        // Check read count
        assert_eq!(s.read_count(), 2);
    }

    #[test]
    fn test_ex29_sensor_multiple_reads() {
        let s = Sensor::new("humidity", 50);
        for _ in 0..5 {
            let _ = s.read();
        }
        assert_eq!(s.read_count(), 5);
    }

    #[test]
    fn test_ex29_sensor_shared_ref() {
        // The whole point: read(&self) takes a shared reference,
        // but can still track read_count via interior mutability.
        let s = Sensor::new("wind", 0);
        let r1 = &s;
        let r2 = &s;
        r1.read();
        r2.read();
        assert_eq!(s.read_count(), 2);
    }

    // ── Part C tests ───────────────────────────────────────────

    #[test]
    fn test_ex29_counters_new() {
        let c = SharedCounters::new();
        assert_eq!(c.requests(), 0);
        assert_eq!(c.errors(), 0);
    }

    #[test]
    fn test_ex29_counters_increment() {
        let c = SharedCounters::new();
        c.inc_requests();
        c.inc_requests();
        c.inc_errors();
        assert_eq!(c.requests(), 2);
        assert_eq!(c.errors(), 1);
    }

    #[test]
    fn test_ex29_counters_c_increment() {
        let c = SharedCounters::new();
        c.inc_requests(); // Rust increments

        // C increments through the raw pointer
        unsafe { c_atomic_increment(c.requests_ptr(), 5); }

        assert_eq!(c.requests(), 6); // 1 + 5
    }

    #[test]
    fn test_ex29_counters_shared_ref() {
        // Multiple shared refs can all increment — interior mutability
        let c = SharedCounters::new();
        let r1 = &c;
        let r2 = &c;
        r1.inc_requests();
        r2.inc_requests();
        assert_eq!(c.requests(), 2);
    }

    // ── Part D tests ───────────────────────────────────────────

    #[test]
    fn test_ex29_observe_with_unsafecell() {
        let (before, after) = observe_with_unsafecell();
        assert_eq!(before, 10);
        assert_eq!(after, 42);
    }

    #[test]
    fn test_ex29_observe_without_unsafecell() {
        let (before, after) = observe_without_unsafecell();
        assert_eq!(before, 10);
        assert_eq!(after, 42);
    }

    // ── Layout tests ──────────────────────────────────────────

    #[test]
    fn test_ex29_unsafecell_is_transparent() {
        // UnsafeCell<T> has the same layout as T.
        assert_eq!(
            std::mem::size_of::<UnsafeCell<u32>>(),
            std::mem::size_of::<u32>()
        );
        assert_eq!(
            std::mem::align_of::<UnsafeCell<u32>>(),
            std::mem::align_of::<u32>()
        );
    }

    #[test]
    fn test_ex29_atomic_contains_unsafecell() {
        // AtomicI32 is repr(transparent) over UnsafeCell<i32>,
        // so it is layout-compatible with C's _Atomic int32_t.
        assert_eq!(
            std::mem::size_of::<AtomicI32>(),
            std::mem::size_of::<i32>()
        );
    }
}
