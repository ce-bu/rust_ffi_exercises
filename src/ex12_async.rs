//! # Exercise 12: Async / Threaded FFI Interop
//!
//! **Concept:** FFI boundaries are synchronous, but real applications
//! need async.  The three most common bridging patterns are:
//!
//! 1. **Completion callback** — C calls Rust, which spawns a worker
//!    thread and fires a callback when done.
//!
//! 2. **Blocking bridge** — wrapping a blocking C function inside
//!    `tokio::task::spawn_blocking` so it doesn't stall the async
//!    runtime.
//!
//! 3. **Opaque runtime handle** — letting C manage a Tokio runtime's
//!    lifetime via create / run / destroy functions.
//!
//! ## Pre-provided (in `csrc/ex12_blocking.c`)
//!
//! ```c
//! int c_slow_compute(int input);   // blocks ~10 ms, returns input²+1
//! ```
//!
//! ## Your task
//!
//! Implement the three parts below.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex12
//! ```

use std::ffi::c_void;

extern "C" {
    fn c_slow_compute(input: i32) -> i32;
}

// ══════════════════════════════════════════════════════════════
// Part A — Completion callback
// ══════════════════════════════════════════════════════════════

pub type CompletionCb = extern "C" fn(result: i64, user_data: *mut c_void);

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Spawn a **background thread** that computes the `n`-th Fibonacci
// number.  When done, call `cb(result, user_data)` from that thread.
//
// Steps:
//   1. `std::thread::spawn(move || { ... })`.
//   2. Inside the thread: compute fib(n), then call cb(fib_n, user_data).
//
// IMPORTANT: `user_data` is a raw pointer.  To send it to another
// thread you need to wrap it in a helper struct that implements
// `Send`:
//
//   struct SendPtr(*mut c_void);
//   unsafe impl Send for SendPtr {}

/// # Safety
/// - `user_data` must remain valid until `cb` is called.
/// - `cb` must be safe to call from any thread.
#[no_mangle]
pub unsafe extern "C" fn ffi_compute_async(
    n: u32,
    cb: CompletionCb,
    user_data: *mut c_void,
) {
    todo!(
        "Wrap user_data in a Send newtype, spawn a thread, \
         compute fib(n), call cb(result, user_data)"
    )
}

// ══════════════════════════════════════════════════════════════
// Part B — Blocking bridge (spawn_blocking)
// ══════════════════════════════════════════════════════════════

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Wrap the blocking C function `c_slow_compute` so it can be
// called from an async context without stalling the tokio runtime.
//
// Hint:
//   tokio::task::spawn_blocking(move || unsafe { c_slow_compute(input) }).await.unwrap()

/// Async wrapper around the blocking C function.
pub async fn async_slow_compute(input: i32) -> i32 {
    todo!("Use tokio::task::spawn_blocking to call c_slow_compute")
}

// ══════════════════════════════════════════════════════════════
// Part C — Opaque runtime handle
// ══════════════════════════════════════════════════════════════

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Expose a Tokio runtime as an opaque handle that C can manage.
//
// Define a struct `TokioRuntime` wrapping `tokio::runtime::Runtime`.
// Then implement:
//   ffi_runtime_new()  → *mut TokioRuntime
//   ffi_runtime_block_on(*mut TokioRuntime, input: i32) → i32
//       — runs async_slow_compute(input) on the runtime
//   ffi_runtime_free(*mut TokioRuntime)

pub struct TokioRuntime {
    // TODO: wrap a tokio::runtime::Runtime
}

/// Create a new multi-threaded Tokio runtime.
#[no_mangle]
pub extern "C" fn ffi_runtime_new() -> *mut TokioRuntime {
    todo!("Build a Runtime, Box it, return raw pointer")
}

/// Block on `async_slow_compute(input)` using the given runtime.
///
/// # Safety
/// `rt` must be a valid pointer from `ffi_runtime_new`.
#[no_mangle]
pub unsafe extern "C" fn ffi_runtime_block_on(
    rt: *mut TokioRuntime,
    input: i32,
) -> i32 {
    todo!("(*rt).runtime.block_on(async_slow_compute(input))")
}

/// Destroy the runtime.
///
/// # Safety
/// `rt` must be from `ffi_runtime_new`, not used after this call.
#[no_mangle]
pub unsafe extern "C" fn ffi_runtime_free(rt: *mut TokioRuntime) {
    todo!("Box::from_raw, drop")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex, Condvar};

    #[test]
    fn test_ex12_compute_async() {
        // We'll use a condvar to wait for the callback.
        let pair = Arc::new((Mutex::new(None::<i64>), Condvar::new()));
        let pair2 = Arc::clone(&pair);

        extern "C" fn on_done(result: i64, ctx: *mut c_void) {
            let pair = unsafe { &*(ctx as *const (Mutex<Option<i64>>, Condvar)) };
            *pair.0.lock().unwrap() = Some(result);
            pair.1.notify_one();
        }

        unsafe {
            ffi_compute_async(
                10,
                on_done,
                Arc::as_ptr(&pair2) as *mut c_void,
            );
        }

        let (lock, cvar) = &*pair;
        let mut result = lock.lock().unwrap();
        while result.is_none() {
            result = cvar.wait(result).unwrap();
        }
        assert_eq!(result.unwrap(), 55); // fib(10) = 55
    }

    #[tokio::test]
    async fn test_ex12_async_slow_compute() {
        let result = async_slow_compute(5).await;
        assert_eq!(result, 26); // 5*5 + 1
    }

    #[test]
    fn test_ex12_runtime_handle() {
        let rt = ffi_runtime_new();
        assert!(!rt.is_null());

        let result = unsafe { ffi_runtime_block_on(rt, 4) };
        assert_eq!(result, 17); // 4*4 + 1

        unsafe { ffi_runtime_free(rt) };
    }
}
