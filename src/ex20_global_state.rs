//! # Exercise 20: Global State & Library Init/Shutdown
//!
//! **Concept:** Almost every C library follows the pattern:
//!
//! ```text
//! lib_init()          ← call once, before anything else
//! lib_do_stuff(...)   ← main API (may be called from many threads)
//! lib_shutdown()      ← call once, after everything else
//! ```
//!
//! Examples: `curl_global_init()`, `SSL_library_init()`,
//! `sqlite3_initialize()`, `pa_mainloop_new()`.
//!
//! In Rust, we must:
//! - Ensure `init` is called exactly once (`std::sync::OnceLock` or
//!   `std::sync::Once`).
//! - Protect global mutable state (`Mutex`, `RwLock`, or atomics).
//! - Make the API thread-safe (C libraries are often not — we add
//!   the safety on the Rust side).
//!
//! ## Your task
//!
//! Build a "metrics" library with a global registry, exposed via
//! `extern "C"` functions.  The library must be initialized before
//! use and cleaned up on shutdown.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex20
//! ```

use std::collections::HashMap;
use std::ffi::{c_char, c_int, CStr};
use std::sync::{Mutex, OnceLock};

// ══════════════════════════════════════════════════════════════
// Global state
// ══════════════════════════════════════════════════════════════

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Define the global metrics registry.
//
// Use `OnceLock<Mutex<MetricsState>>` so:
//   - `OnceLock` ensures one-time initialization.
//   - `Mutex` protects concurrent access.
//
// The `MetricsState` struct should contain:
//   - `counters: HashMap<String, i64>` — named counters.
//   - `initialized: bool` — set to true after init.
//
// Declare the global:
//   static METRICS: OnceLock<Mutex<MetricsState>> = OnceLock::new();

struct MetricsState {
    // TODO: add fields
}

// TODO: declare the OnceLock global
// static METRICS: OnceLock<Mutex<MetricsState>> = OnceLock::new();

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Implement `metrics_init`.  It should:
//   1. Use `METRICS.get_or_init(...)` to initialize the state
//      exactly once in a thread-safe manner.
//   2. Return 0 on success.
//   3. Return 0 (not an error) if called again — idempotent.
//
// Note: `OnceLock::get_or_init` guarantees that the closure runs
// at most once, even under concurrent calls.

/// Initialize the metrics library.  Thread-safe, idempotent.
/// Returns 0 on success.
#[no_mangle]
pub extern "C" fn metrics_init() -> c_int {
    todo!(
        "Use METRICS.get_or_init(|| Mutex::new(MetricsState {{ ... }})), \
         return 0"
    )
}

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Helper: get a reference to the initialized state, or return an
// error code (-1) if the library was not initialized.
//
// fn get_state() -> Result<&'static Mutex<MetricsState>, c_int> {
//     METRICS.get().ok_or(-1)
// }

fn get_state() -> Result<&'static Mutex<MetricsState>, c_int> {
    todo!("METRICS.get().ok_or(-1)")
}

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Implement the metrics API:
//
//   metrics_increment(name)     — add 1 to a named counter
//   metrics_add(name, delta)    — add `delta` to a named counter
//   metrics_get(name)           — return counter value (0 if absent)
//   metrics_reset(name)         — reset a counter to 0
//   metrics_reset_all()         — reset all counters
//
// All functions should:
//   - Call `get_state()` and return -1 if not initialized.
//   - Lock the Mutex, perform the operation, unlock.
//   - Return 0 on success (except `metrics_get` returns the value).

/// Increment counter `name` by 1.  Returns 0 or -1 if not initialized.
///
/// # Safety
/// `name` must be a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn metrics_increment(name: *const c_char) -> c_int {
    todo!("get_state(), lock, increment counters[name] by 1")
}

/// Add `delta` to counter `name`.  Returns 0 or -1.
///
/// # Safety
/// `name` must be a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn metrics_add(
    name: *const c_char,
    delta: i64,
) -> c_int {
    todo!("get_state(), lock, add delta to counters[name]")
}

/// Return the current value of counter `name`, or 0 if absent.
/// Returns -1 if the library is not initialized.
///
/// # Safety
/// `name` must be a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn metrics_get(name: *const c_char) -> i64 {
    todo!("get_state(), lock, return *counters.get(name).unwrap_or(&0)")
}

/// Reset counter `name` to 0.  Returns 0 or -1.
///
/// # Safety
/// `name` must be a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn metrics_reset(name: *const c_char) -> c_int {
    todo!("get_state(), lock, remove counter or set to 0")
}

/// Reset ALL counters.  Returns 0 or -1.
#[no_mangle]
pub extern "C" fn metrics_reset_all() -> c_int {
    todo!("get_state(), lock, clear the HashMap")
}

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Implement `metrics_shutdown`.
//
// In a real library this would release resources.  Since OnceLock
// doesn't support "un-initializing", clear the state instead:
// set `initialized = false` and clear all counters.
//
// After shutdown, all API calls should return -1 until
// `metrics_init()` is called again.
//
// DESIGN NOTE: In production code you might use `AtomicBool` for
// the initialized flag so get_state() can check without locking.
// For this exercise, keeping it inside the Mutex is fine.

/// Shut down the metrics library.  Returns 0 or -1.
#[no_mangle]
pub extern "C" fn metrics_shutdown() -> c_int {
    todo!(
        "get_state(), lock, set initialized = false, \
         clear counters, return 0"
    )
}

// ══════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn cstr(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    /// Reset state between tests.
    fn setup() {
        metrics_init();
        metrics_reset_all();
    }

    #[test]
    fn test_ex20_init_idempotent() {
        assert_eq!(metrics_init(), 0);
        assert_eq!(metrics_init(), 0); // second call is fine
    }

    #[test]
    fn test_ex20_increment() {
        setup();
        let name = cstr("requests");
        unsafe {
            metrics_increment(name.as_ptr());
            metrics_increment(name.as_ptr());
            metrics_increment(name.as_ptr());
            assert_eq!(metrics_get(name.as_ptr()), 3);
        }
    }

    #[test]
    fn test_ex20_add() {
        setup();
        let name = cstr("bytes");
        unsafe {
            metrics_add(name.as_ptr(), 100);
            metrics_add(name.as_ptr(), 250);
            assert_eq!(metrics_get(name.as_ptr()), 350);
        }
    }

    #[test]
    fn test_ex20_get_absent_key() {
        setup();
        let name = cstr("nonexistent");
        unsafe {
            assert_eq!(metrics_get(name.as_ptr()), 0);
        }
    }

    #[test]
    fn test_ex20_reset_single() {
        setup();
        let a = cstr("a");
        let b = cstr("b");
        unsafe {
            metrics_add(a.as_ptr(), 10);
            metrics_add(b.as_ptr(), 20);
            metrics_reset(a.as_ptr());
            assert_eq!(metrics_get(a.as_ptr()), 0);
            assert_eq!(metrics_get(b.as_ptr()), 20); // untouched
        }
    }

    #[test]
    fn test_ex20_reset_all() {
        setup();
        let a = cstr("x");
        let b = cstr("y");
        unsafe {
            metrics_add(a.as_ptr(), 5);
            metrics_add(b.as_ptr(), 10);
        }
        metrics_reset_all();
        unsafe {
            assert_eq!(metrics_get(a.as_ptr()), 0);
            assert_eq!(metrics_get(b.as_ptr()), 0);
        }
    }

    #[test]
    fn test_ex20_multiple_counters() {
        setup();
        let c1 = cstr("counter1");
        let c2 = cstr("counter2");
        let c3 = cstr("counter3");
        unsafe {
            metrics_increment(c1.as_ptr());
            metrics_add(c2.as_ptr(), 100);
            metrics_add(c3.as_ptr(), -5);
            assert_eq!(metrics_get(c1.as_ptr()), 1);
            assert_eq!(metrics_get(c2.as_ptr()), 100);
            assert_eq!(metrics_get(c3.as_ptr()), -5);
        }
    }

    #[test]
    fn test_ex20_thread_safety() {
        setup();
        let name = cstr("concurrent");
        let threads: Vec<_> = (0..10)
            .map(|_| {
                let n = name.clone();
                std::thread::spawn(move || unsafe {
                    for _ in 0..100 {
                        metrics_increment(n.as_ptr());
                    }
                })
            })
            .collect();

        for t in threads {
            t.join().unwrap();
        }

        unsafe {
            assert_eq!(metrics_get(name.as_ptr()), 1000);
        }
    }
}
