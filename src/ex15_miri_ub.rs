//! # Exercise 15: Fixing Undefined Behavior with Miri
//!
//! **Concept:** FFI code is full of `unsafe` — and it's easy to
//! introduce *undefined behavior* (UB) that compiles and even appears
//! to work, but is fundamentally broken.
//!
//! **Miri** is a Rust interpreter that can detect many kinds of UB:
//! - Stacked Borrows violations (aliasing rules broken)
//! - Use-after-free
//! - Out-of-bounds memory access
//! - Unaligned pointer access
//! - Reading uninitialized memory
//!
//! ## How to use Miri
//!
//! ```sh
//! # Install Miri (one-time)
//! rustup component add miri
//!
//! # Run all tests under Miri
//! cargo miri test ex15
//!
//! # Run a specific test
//! cargo miri test ex15_bug1
//! ```
//!
//! Miri will print a detailed error showing exactly which operation
//! violated the rules, with a backtrace pointing to the offending line.
//!
//! ## Your task
//!
//! Each `bugN_*` function below contains **intentional UB**.  The code
//! compiles and *may even pass* `cargo test` — but `cargo miri test`
//! will catch the bug.
//!
//! For each bug:
//! 1. Run `cargo miri test ex15_bugN` and read the error.
//! 2. Understand WHY it's UB.
//! 3. Fix the function so Miri is happy.
//!
//! The `fixedN_*` functions are stubs for your corrected versions.
//!
//! ## Verify
//!
//! ```sh
//! cargo miri test ex15        # ALL bugs fixed when this passes
//! ```

use std::ptr;

// ══════════════════════════════════════════════════════════════
// Bug 1 — Pointer invalidated by reallocation
// ══════════════════════════════════════════════════════════════
//
// A pointer into a Vec becomes dangling when the Vec reallocates.
// This is a USE-AFTER-FREE / dangling pointer read.
//
// Miri error: "pointer to alloc was dereferenced after this
// allocation got freed"

/// **BUGGY** — do NOT change this function.
pub fn bug1_dangling_vec_pointer() -> i32 {
    let mut v = vec![1, 2, 3];
    let ptr = v.as_ptr(); // pointer to current allocation

    // This push may reallocate, invalidating `ptr`.
    v.push(4);
    v.push(5);
    v.push(6);

    // Reading through the (now possibly dangling) pointer = UB!
    unsafe { *ptr.add(0) + *ptr.add(1) }
}

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Write a fixed version that computes the same result (sum of the
// first two elements after pushing 4,5,6) without UB.
//
// Hint: take the pointer AFTER all mutations are done.

pub fn fixed1_dangling_vec_pointer() -> i32 {
    todo!("Fix bug1: take the pointer after all pushes")
}

// ══════════════════════════════════════════════════════════════
// Bug 2 — Mutable aliasing through raw pointers
// ══════════════════════════════════════════════════════════════
//
// Creating two `&mut` references to the same data at the same time
// violates Rust's aliasing rules — even when done through raw
// pointers.
//
// Miri error: "trying to retag ... but found SharedReadOnly"
// or "borrow stack violation"

/// **BUGGY** — do NOT change this function.
pub fn bug2_aliased_mutable_refs() -> (i32, i32) {
    let mut value: i32 = 10;
    let ptr = &mut value as *mut i32;

    // Create two &mut references from the same pointer — UB!
    let ref_a = unsafe { &mut *ptr };
    let ref_b = unsafe { &mut *ptr }; // invalidates ref_a's "borrow"

    *ref_a += 1; // writes through invalidated borrow
    *ref_b += 2;

    (*ref_a, *ref_b)
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Fix this so it produces the same result (11, 13) without aliasing.
//
// Hint: do NOT hold two &mut at the same time.  Either:
//   (a) Use one &mut at a time (reborrow after each use), or
//   (b) Use raw pointer writes: ptr.write(...) / ptr.read()

pub fn fixed2_aliased_mutable_refs() -> (i32, i32) {
    todo!("Fix bug2: avoid two simultaneous &mut to the same data")
}

// ══════════════════════════════════════════════════════════════
// Bug 3 — Reading uninitialized memory
// ══════════════════════════════════════════════════════════════
//
// Allocating memory and reading it before writing is UB — the
// bytes are uninitialized.
//
// Miri error: "using uninitialized data" / "Uninit"

/// **BUGGY** — do NOT change this function.
pub fn bug3_read_uninit() -> i32 {
    let layout = std::alloc::Layout::new::<i32>();
    let ptr = unsafe { std::alloc::alloc(layout) as *mut i32 };

    // Reading before writing — the memory is uninitialized!
    let val = unsafe { *ptr };

    unsafe { std::alloc::dealloc(ptr as *mut u8, layout) };
    val
}

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Fix this: allocate, WRITE a value, then read it back.
//
// Hint: use ptr::write(ptr, some_value) before reading.

pub fn fixed3_read_uninit() -> i32 {
    todo!("Fix bug3: write before reading")
}

// ══════════════════════════════════════════════════════════════
// Bug 4 — Out-of-bounds pointer arithmetic
// ══════════════════════════════════════════════════════════════
//
// Pointer arithmetic past the allocation is UB even if you don't
// dereference the out-of-bounds pointer (the arithmetic itself
// is the violation with offset, though add is typically caught
// on deref).
//
// Miri error: "out-of-bounds pointer arithmetic" or
// "dereferencing pointer ... which is out of bounds"

/// **BUGGY** — do NOT change this function.
pub fn bug4_oob_access() -> i32 {
    let data = [10, 20, 30];
    let ptr = data.as_ptr();

    // Index 3 is past the end of a 3-element array!
    let sum = unsafe { *ptr.add(0) + *ptr.add(1) + *ptr.add(3) };
    sum
}

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Fix: sum all three elements (indices 0, 1, 2).

pub fn fixed4_oob_access() -> i32 {
    todo!("Fix bug4: use correct indices")
}

// ══════════════════════════════════════════════════════════════
// Bug 5 — Use after free
// ══════════════════════════════════════════════════════════════
//
// Using a Box's pointer after the Box has been dropped.
//
// Miri error: "pointer to alloc was used after this allocation
// got freed"

/// **BUGGY** — do NOT change this function.
pub fn bug5_use_after_free() -> i32 {
    let b = Box::new(42i32);
    let ptr = &*b as *const i32;

    drop(b); // frees the heap allocation

    // Reading through a dangling pointer = UB!
    unsafe { *ptr }
}

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Fix: read the value BEFORE dropping the Box.

pub fn fixed5_use_after_free() -> i32 {
    todo!("Fix bug5: read before drop, or don't drop manually")
}

// ══════════════════════════════════════════════════════════════
// Bug 6 — Stacked Borrows: &T to &mut T transmute
// ══════════════════════════════════════════════════════════════
//
// Casting away immutability via raw pointers is UB when the
// original reference is &T (shared).  Stacked Borrows enforces
// this — a SharedReadOnly tag cannot become Unique.
//
// Miri error: "attempting a write through ... but it is
// SharedReadOnly"

/// **BUGGY** — do NOT change this function.
pub fn bug6_shared_to_mut(value: &i32) -> i32 {
    // Cast away the immutability — UB!
    let ptr = value as *const i32 as *mut i32;
    unsafe { ptr.write(ptr.read() + 1) };
    unsafe { ptr.read() }
}

// ── TODO 6 ─────────────────────────────────────────────────────
//
// Fix: the function should accept &mut i32 instead, or work on
// a local copy.

pub fn fixed6_shared_to_mut(value: &i32) -> i32 {
    todo!(
        "Fix bug6: either take &mut, or copy the value and \
         work on the copy (don't mutate a shared reference)"
    )
}

// ══════════════════════════════════════════════════════════════
// Bug 7 — Stacked Borrows: raw pointer stacking violation
// ══════════════════════════════════════════════════════════════
//
// Even with raw pointers, Stacked Borrows tracks the "permission
// stack".  Writing through a pointer can invalidate older pointers
// derived from the same source.
//
// Miri error: "borrow stack" / "tag ... was created here ...
// but then invalidated"

/// **BUGGY** — do NOT change this function.
pub fn bug7_stacked_borrows_violation() -> [i32; 2] {
    let mut array = [1i32, 2];

    // Get a raw pointer from the array.
    let ptr = array.as_mut_ptr();

    // Create a new &mut borrow — this invalidates `ptr` on the
    // borrow stack because the new reference takes priority.
    let mref = &mut array;
    mref[0] = 10;
    mref[1] = 20;

    // Writing through the old raw pointer — its tag has been
    // popped from the stack by the &mut borrow above.  UB!
    unsafe { *ptr = 11 };

    array
}

// ── TODO 7 ─────────────────────────────────────────────────────
//
// Fix: don't use a raw pointer after a new &mut borrow
// invalidates it.  Either:
//   (a) Do all raw-pointer work before creating new references, or
//   (b) Re-derive the pointer after the mutable borrow ends.

pub fn fixed7_stacked_borrows_violation() -> [i32; 2] {
    todo!(
        "Fix bug7: derive pointers from non-overlapping borrows \
         (split_at_mut) or from a single raw pointer"
    )
}

// ══════════════════════════════════════════════════════════════
// Bug 8 — Unaligned pointer access
// ══════════════════════════════════════════════════════════════
//
// Creating a reference to unaligned data is UB — even `*const`
// dereference of an unaligned pointer is UB.
//
// Miri error: "accessing memory ... but alignment 4 is required"

/// **BUGGY** — do NOT change this function.
pub fn bug8_unaligned_read() -> u32 {
    let data: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

    // Casting a byte pointer at offset 1 to *const u32 — misaligned!
    let ptr = unsafe { data.as_ptr().add(1) as *const u32 };
    unsafe { *ptr }
}

// ── TODO 8 ─────────────────────────────────────────────────────
//
// Fix: use `ptr::read_unaligned` instead of a regular dereference.
//
// Hint: std::ptr::read_unaligned(ptr)

pub fn fixed8_unaligned_read() -> u32 {
    todo!("Fix bug8: use read_unaligned for the misaligned pointer")
}

// ── Tests ──────────────────────────────────────────────────────
//
// The `bugN` tests are `#[ignore]`d by default — they contain
// intentional UB.  Run them under Miri to see the errors:
//
//     cargo miri test ex15_bug1 -- --ignored
//
// The `fixedN` tests run normally and under Miri:
//
//     cargo miri test ex15_fixed

#[cfg(test)]
mod tests {
    use super::*;

    // ── Bug demonstrations (ignored — intentional UB) ──────────

    #[test]
    #[ignore = "intentional UB — run with: cargo miri test ex15_bug1 -- --ignored"]
    fn test_ex15_bug1_dangling_pointer() {
        let _ = bug1_dangling_vec_pointer();
    }

    #[test]
    #[ignore = "intentional UB — run with: cargo miri test ex15_bug2 -- --ignored"]
    fn test_ex15_bug2_aliased_mut() {
        let _ = bug2_aliased_mutable_refs();
    }

    #[test]
    #[ignore = "intentional UB — run with: cargo miri test ex15_bug3 -- --ignored"]
    fn test_ex15_bug3_read_uninit() {
        let _ = bug3_read_uninit();
    }

    #[test]
    #[ignore = "intentional UB — run with: cargo miri test ex15_bug4 -- --ignored"]
    fn test_ex15_bug4_oob() {
        let _ = bug4_oob_access();
    }

    #[test]
    #[ignore = "intentional UB — run with: cargo miri test ex15_bug5 -- --ignored"]
    fn test_ex15_bug5_use_after_free() {
        let _ = bug5_use_after_free();
    }

    #[test]
    #[ignore = "intentional UB — run with: cargo miri test ex15_bug6 -- --ignored"]
    fn test_ex15_bug6_shared_to_mut() {
        let val = 10;
        let _ = bug6_shared_to_mut(&val);
    }

    #[test]
    #[ignore = "intentional UB — run with: cargo miri test ex15_bug7 -- --ignored"]
    fn test_ex15_bug7_stacked_borrows() {
        let _ = bug7_stacked_borrows_violation();
    }

    #[test]
    #[ignore = "intentional UB — run with: cargo miri test ex15_bug8 -- --ignored"]
    fn test_ex15_bug8_unaligned() {
        let _ = bug8_unaligned_read();
    }

    // ── Your fixes (should pass under both cargo test AND Miri) ─

    #[test]
    fn test_ex15_fixed1() {
        assert_eq!(fixed1_dangling_vec_pointer(), 3); // 1 + 2
    }

    #[test]
    fn test_ex15_fixed2() {
        assert_eq!(fixed2_aliased_mutable_refs(), (11, 13));
    }

    #[test]
    fn test_ex15_fixed3() {
        // The fixed version should write a known value and read it back.
        let val = fixed3_read_uninit();
        // Accept any deterministic value — the point is no UB.
        assert_eq!(val, val);
    }

    #[test]
    fn test_ex15_fixed4() {
        assert_eq!(fixed4_oob_access(), 60); // 10 + 20 + 30
    }

    #[test]
    fn test_ex15_fixed5() {
        assert_eq!(fixed5_use_after_free(), 42);
    }

    #[test]
    fn test_ex15_fixed6() {
        let val = 10;
        // Fixed version should return val + 1 = 11 (or work on a copy)
        assert_eq!(fixed6_shared_to_mut(&val), 11);
    }

    #[test]
    fn test_ex15_fixed7() {
        assert_eq!(fixed7_stacked_borrows_violation(), [11, 20]);
    }

    #[test]
    fn test_ex15_fixed8() {
        // Same bytes read from offset 1, just done safely.
        let result = fixed8_unaligned_read();
        let expected = bug8_expected_value();
        assert_eq!(result, expected);
    }

    /// Helper: compute what the unaligned read should produce.
    fn bug8_expected_value() -> u32 {
        let data: [u8; 8] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut bytes = [0u8; 4];
        bytes.copy_from_slice(&data[1..5]);
        u32::from_ne_bytes(bytes)
    }
}
