//! # Exercise 18: Invoking Rust Closures from C
//!
//! **Concept:** Rust closures capture environment and implement one of
//! `Fn`, `FnMut`, or `FnOnce`.  C only understands bare function
//! pointers, so we need a **trampoline** pattern:
//!
//! ```text
//!  ┌──────────────────────────────────┐
//!  │  Rust closure                    │
//!  │    captured state                │
//!  └─────────┬────────────────────────┘
//!            │ &/&mut/*self
//!  ┌─────────▼────────────────────────┐
//!  │  trampoline (extern "C" fn)      │
//!  │    cast void* → &(mut) Closure   │
//!  │    call the closure              │
//!  └─────────┬────────────────────────┘
//!            │ fn ptr + void* ctx
//!  ┌─────────▼────────────────────────┐
//!  │  C function                      │
//!  │    calls fn(arg, ctx)            │
//!  └──────────────────────────────────┘
//! ```
//!
//! The key insight is that each `Fn` family needs a different kind
//! of pointer cast inside the trampoline:
//!
//! | Trait    | Context pointer | Trampoline casts to   | Consumption |
//! |----------|----------------|-----------------------|-------------|
//! | `Fn`     | `*const c_void`| `&F`                  | Shared ref  |
//! | `FnMut`  | `*mut c_void`  | `&mut F`              | Mutable ref |
//! | `FnOnce` | `*mut c_void`  | `Box<F>` (via `Box::from_raw`) | Takes ownership |
//!
//! ## Pre-provided C helpers (in `csrc/ex18_closures.c`)
//!
//! ```c
//! // Fn-style:   calls f(a,ctx) + f(b,ctx)  (two calls, shared ctx)
//! int32_t c_apply_twice(int32_t a, int32_t b,
//!                       int32_t (*f)(int32_t, void*), void *ctx);
//!
//! // FnMut-style: calls next(ctx) `len` times to fill array
//! void c_generate(int32_t *out, size_t len,
//!                 int32_t (*next)(void*), void *ctx);
//!
//! // FnOnce-style: calls f(ctx) exactly once
//! int32_t c_call_once(int32_t (*f)(void*), void *ctx);
//!
//! // Fn-style:   calls f(input[i], ctx) for each element
//! void c_map_array(const int32_t *input, int32_t *output, size_t len,
//!                  int32_t (*f)(int32_t, void*), void *ctx);
//! ```
//!
//! ## Your task
//!
//! Implement the five TODO sections below.  Each one teaches a
//! different way to ship a Rust closure across the FFI boundary.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex18
//! ```

use std::ffi::c_void;

// ── C function declarations (pre-provided) ─────────────────────

extern "C" {
    fn c_apply_twice(
        a: i32,
        b: i32,
        f: extern "C" fn(i32, *mut c_void) -> i32,
        ctx: *mut c_void,
    ) -> i32;

    fn c_generate(
        out: *mut i32,
        len: usize,
        next: extern "C" fn(*mut c_void) -> i32,
        ctx: *mut c_void,
    );

    fn c_call_once(
        f: extern "C" fn(*mut c_void) -> i32,
        ctx: *mut c_void,
    ) -> i32;

    fn c_map_array(
        input: *const i32,
        output: *mut i32,
        len: usize,
        f: extern "C" fn(i32, *mut c_void) -> i32,
        ctx: *mut c_void,
    );
}

// ══════════════════════════════════════════════════════════════
// Part A — Fn: shared, immutable closure
// ══════════════════════════════════════════════════════════════

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Implement `apply_twice_with_closure`.  It should:
//
//   1. Accept a closure `f: impl Fn(i32) -> i32`.
//   2. Write a trampoline `extern "C" fn` that casts the `void*`
//      context back to `&F` and calls it.
//   3. Pass the closure (by reference) and the trampoline to
//      `c_apply_twice`.
//
// Because `Fn` only needs `&self`, we pass `&f` as the context
// pointer.  The closure is NOT consumed — it stays on the stack.
//
// Skeleton:
//
//   extern "C" fn trampoline<F: Fn(i32) -> i32>(
//       value: i32, ctx: *mut c_void,
//   ) -> i32 {
//       let closure = unsafe { &*(ctx as *const F) };
//       closure(value)
//   }
//
//   let ctx = &f as *const F as *mut c_void;
//   unsafe { c_apply_twice(a, b, trampoline::<F>, ctx) }

/// Compute `f(a) + f(b)` by dispatching through C.
pub fn apply_twice_with_closure(
    a: i32,
    b: i32,
    f: impl Fn(i32) -> i32,
) -> i32 {
    todo!(
        "Write a trampoline, pass &closure as *mut c_void, \
         call c_apply_twice"
    )
}

// ══════════════════════════════════════════════════════════════
// Part B — FnMut: mutable closure (stateful generator)
// ══════════════════════════════════════════════════════════════

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Implement `generate_with_closure`.  It should:
//
//   1. Accept a closure `f: impl FnMut() -> i32`.
//   2. Write a trampoline `extern "C" fn` that casts the `void*`
//      to `&mut F` and calls the closure.
//   3. Pass `&mut f` as the context pointer to `c_generate`.
//   4. Return the filled `Vec<i32>`.
//
// Because `FnMut` needs `&mut self`, the trampoline must cast to
// `*mut F` (mutable reference).  The closure stays on the stack
// but can mutate its captured state.
//
// Skeleton:
//
//   extern "C" fn trampoline<F: FnMut() -> i32>(
//       ctx: *mut c_void,
//   ) -> i32 {
//       let closure = unsafe { &mut *(ctx as *mut F) };
//       closure()
//   }
//
//   let mut buf = vec![0i32; count];
//   let ctx = &mut f as *mut F as *mut c_void;
//   unsafe { c_generate(buf.as_mut_ptr(), count, trampoline::<F>, ctx) }
//   buf

/// Fill a Vec of `count` elements by calling the closure repeatedly.
pub fn generate_with_closure(
    count: usize,
    f: impl FnMut() -> i32,
) -> Vec<i32> {
    todo!(
        "Write a FnMut trampoline, pass &mut closure as *mut c_void, \
         call c_generate, return the filled buffer"
    )
}

// ══════════════════════════════════════════════════════════════
// Part C — FnOnce: consuming closure (called exactly once)
// ══════════════════════════════════════════════════════════════

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Implement `call_once_with_closure`.  It should:
//
//   1. Accept a closure `f: impl FnOnce() -> i32`.
//   2. Box the closure and convert to a raw pointer:
//        `Box::into_raw(Box::new(f)) as *mut c_void`
//   3. Write a trampoline that reclaims ownership via `Box::from_raw`
//      and calls the closure, consuming it.
//   4. Pass the boxed closure to `c_call_once`.
//
// Because `FnOnce` consumes `self`, the trampoline must take
// ownership back from the raw pointer.  After the call the Box
// is dropped — the closure's captured state is freed.
//
// Skeleton:
//
//   extern "C" fn trampoline<F: FnOnce() -> i32>(
//       ctx: *mut c_void,
//   ) -> i32 {
//       let closure = unsafe { *Box::from_raw(ctx as *mut F) };
//       closure()
//   }
//
//   let ctx = Box::into_raw(Box::new(f)) as *mut c_void;
//   unsafe { c_call_once(trampoline::<F>, ctx) }

/// Invoke a consuming closure exactly once, dispatched through C.
pub fn call_once_with_closure(f: impl FnOnce() -> i32) -> i32 {
    todo!(
        "Box the closure, write a FnOnce trampoline, \
         pass to c_call_once"
    )
}

// ══════════════════════════════════════════════════════════════
// Part D — Fn + generic: map an array through a closure
// ══════════════════════════════════════════════════════════════

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Implement `map_with_closure`.  It should:
//
//   1. Accept a slice `&[i32]` and a closure `impl Fn(i32) -> i32`.
//   2. Allocate an output Vec of the same length.
//   3. Use the Fn-trampoline pattern (like TODO 1) to call
//      `c_map_array`.
//   4. Return the output Vec.

/// Map each element of `input` through `f`, dispatched through C.
pub fn map_with_closure(
    input: &[i32],
    f: impl Fn(i32) -> i32,
) -> Vec<i32> {
    todo!(
        "Write a trampoline, allocate an output buffer, \
         call c_map_array, return Vec"
    )
}

// ══════════════════════════════════════════════════════════════
// Part E — Combining all three: pipeline
// ══════════════════════════════════════════════════════════════

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Implement `closure_pipeline` which uses all three closure types
// in a single pipeline:
//
//   1. **FnMut** — Use `generate_with_closure` with a counter
//      closure that produces `start, start+1, start+2, …` to
//      create a Vec of `count` elements.
//   2. **Fn** — Use `map_with_closure` to multiply each element
//      by `factor`.
//   3. **FnOnce** — Use `call_once_with_closure` with a closure
//      that **moves** the mapped Vec and returns its sum.
//      (The closure must own the Vec — hence FnOnce.)
//
// Return the final sum.

/// Generate → Map → Sum, each step dispatched through C.
pub fn closure_pipeline(start: i32, count: usize, factor: i32) -> i32 {
    todo!(
        "Chain generate_with_closure, map_with_closure, \
         and call_once_with_closure"
    )
}

// ══════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Part A: Fn ────────────────────────────────────────────

    #[test]
    fn test_ex18_fn_add_offset() {
        // Fn closure captures `offset` by shared reference.
        let offset = 10;
        let result = apply_twice_with_closure(3, 7, |x| x + offset);
        // f(3) + f(7) = 13 + 17 = 30
        assert_eq!(result, 30);
    }

    #[test]
    fn test_ex18_fn_multiply() {
        let factor = 3;
        let result = apply_twice_with_closure(4, 5, |x| x * factor);
        // f(4) + f(5) = 12 + 15 = 27
        assert_eq!(result, 27);
    }

    #[test]
    fn test_ex18_fn_closure_not_consumed() {
        // After passing to C, the closure should still be usable.
        let offset = 100;
        let f = |x: i32| x + offset;
        let r1 = apply_twice_with_closure(1, 2, &f);
        let r2 = apply_twice_with_closure(10, 20, &f);
        assert_eq!(r1, 203); // 101 + 102
        assert_eq!(r2, 230); // 110 + 120
    }

    // ── Part B: FnMut ─────────────────────────────────────────

    #[test]
    fn test_ex18_fnmut_counter() {
        // FnMut closure captures a mutable counter.
        let mut counter = 0i32;
        let result = generate_with_closure(5, || {
            counter += 1;
            counter
        });
        assert_eq!(result, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_ex18_fnmut_fibonacci() {
        // FnMut with two captured values producing Fibonacci.
        let mut a = 0i32;
        let mut b = 1i32;
        let result = generate_with_closure(8, || {
            let val = a;
            let next = a + b;
            a = b;
            b = next;
            val
        });
        assert_eq!(result, vec![0, 1, 1, 2, 3, 5, 8, 13]);
    }

    #[test]
    fn test_ex18_fnmut_empty() {
        let result = generate_with_closure(0, || panic!("should not be called"));
        assert!(result.is_empty());
    }

    // ── Part C: FnOnce ────────────────────────────────────────

    #[test]
    fn test_ex18_fnonce_move_string() {
        // FnOnce closure moves a String into itself.
        let secret = String::from("hello");
        let result = call_once_with_closure(move || {
            // `secret` is consumed here — proves it's FnOnce.
            secret.len() as i32
        });
        assert_eq!(result, 5);
        // `secret` is no longer accessible here — it was moved.
    }

    #[test]
    fn test_ex18_fnonce_move_vec() {
        let data = vec![10, 20, 30];
        let result = call_once_with_closure(move || {
            data.into_iter().sum::<i32>()
        });
        assert_eq!(result, 60);
    }

    #[test]
    fn test_ex18_fnonce_simple() {
        let result = call_once_with_closure(|| 42);
        assert_eq!(result, 42);
    }

    // ── Part D: Fn + map ──────────────────────────────────────

    #[test]
    fn test_ex18_map_double() {
        let input = [1, 2, 3, 4, 5];
        let output = map_with_closure(&input, |x| x * 2);
        assert_eq!(output, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_ex18_map_with_capture() {
        let bias = 100;
        let input = [1, 2, 3];
        let output = map_with_closure(&input, |x| x + bias);
        assert_eq!(output, vec![101, 102, 103]);
    }

    #[test]
    fn test_ex18_map_empty() {
        let input: [i32; 0] = [];
        let output = map_with_closure(&input, |x| x + 1);
        assert!(output.is_empty());
    }

    // ── Part E: Pipeline ──────────────────────────────────────

    #[test]
    fn test_ex18_pipeline() {
        // generate: [0, 1, 2, 3, 4]  (start=0, count=5)
        // map ×10:  [0, 10, 20, 30, 40]
        // sum:      100
        assert_eq!(closure_pipeline(0, 5, 10), 100);
    }

    #[test]
    fn test_ex18_pipeline_offset() {
        // generate: [5, 6, 7]  (start=5, count=3)
        // map ×2:   [10, 12, 14]
        // sum:      36
        assert_eq!(closure_pipeline(5, 3, 2), 36);
    }

    #[test]
    fn test_ex18_pipeline_single() {
        // generate: [1]  (start=1, count=1)
        // map ×7:   [7]
        // sum:      7
        assert_eq!(closure_pipeline(1, 1, 7), 7);
    }
}
