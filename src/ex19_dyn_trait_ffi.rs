//! # Exercise 19: Boxed `dyn Trait` Across FFI
//!
//! **Concept:** Rust trait objects (`Box<dyn Trait>`) provide
//! polymorphism, but they are **fat pointers** — two words wide
//! (data ptr + vtable ptr).  C only deals with single-word pointers,
//! so you cannot cast `Box<dyn Trait>` directly to `*mut c_void`.
//!
//! The standard solution (used by `rustls`, `wgpu`, plugin systems,
//! etc.) is:
//!
//! ```text
//!  ┌──────────────────────────────────────┐
//!  │  Wrapper struct (concrete, Sized)     │
//!  │    inner: Box<dyn Trait>              │ ← fat pointer lives INSIDE
//!  └──────────────┬───────────────────────┘
//!                 │ Box::into_raw → thin *mut Wrapper
//!  ┌──────────────▼───────────────────────┐
//!  │  C side: opaque handle (*mut c_void)  │
//!  └──────────────────────────────────────┘
//! ```
//!
//! **Why is this common?**  Almost every Rust library that exposes a
//! C API with dynamic dispatch uses this pattern:
//! - **`rustls`** — `Arc<dyn …>` inside `rustls_client_config`.
//! - **Plugin systems** — `Box<dyn Plugin>` behind an opaque handle.
//! - **Async runtimes** — `Box<dyn Future>` as a task handle.
//! - **Logging / tracing** — `Box<dyn Log>` registered globally.
//!
//! ## Your task
//!
//! Build a logging framework exposed to C:
//!
//! 1. Define a `Logger` trait and two implementations.
//! 2. Wrap `Box<dyn Logger>` in a concrete `LoggerHandle` struct.
//! 3. Write `extern "C"` functions that take `*mut LoggerHandle`
//!    and dispatch through the trait object.
//! 4. Implement a **composable** logger that wraps another
//!    `Box<dyn Logger>` — showing trait objects that *own* other
//!    trait objects.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex19
//! ```

use std::ffi::{c_char, c_int, CStr};
use std::sync::Mutex;

// ══════════════════════════════════════════════════════════════
// The Logger Trait (pre-provided — do NOT modify)
// ══════════════════════════════════════════════════════════════

/// Log severity levels, C-compatible.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}

/// The trait that all loggers must implement.
pub trait Logger {
    /// Write a log message.
    fn log(&self, level: LogLevel, message: &str);

    /// Flush buffered output (if any). Default is a no-op.
    fn flush(&self) {}

    /// Return the name of this logger (for diagnostics).
    fn name(&self) -> &str;
}

// ══════════════════════════════════════════════════════════════
// Part A — Two Logger implementations
// ══════════════════════════════════════════════════════════════

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Implement two types that implement `Logger`:
//
// (a) `PrintLogger` — logs to a shared `Vec<String>` (wrapped in
//     `Arc<Mutex<Vec<String>>>`) so tests can inspect the output.
//     Each message should be formatted as: "LEVEL: message"
//     (e.g., "INFO: hello").
//
// (b) `NullLogger` — discards all messages (useful as a default).
//
// Both must implement `Logger`.

/// A logger that appends formatted messages to a shared buffer.
pub struct PrintLogger {
    // TODO: add a field for the shared message buffer.
    //   e.g., `buffer: std::sync::Arc<Mutex<Vec<String>>>`
    // and a field for the logger name.
}

impl PrintLogger {
    pub fn new(
        name: &str,
        buffer: std::sync::Arc<Mutex<Vec<String>>>,
    ) -> Self {
        todo!("Store the name and buffer")
    }

    /// Access the shared buffer (for test assertions).
    pub fn messages(&self) -> std::sync::Arc<Mutex<Vec<String>>> {
        todo!("Clone the Arc and return it")
    }
}

impl Logger for PrintLogger {
    fn log(&self, level: LogLevel, message: &str) {
        todo!(
            "Format as 'LEVEL: message' and push to the buffer. \
             Use the level_str() helper below."
        )
    }

    fn name(&self) -> &str {
        todo!("Return the logger name")
    }
}

/// A logger that silently discards all messages.
pub struct NullLogger;

impl Logger for NullLogger {
    fn log(&self, _level: LogLevel, _message: &str) {
        // intentionally empty
    }

    fn name(&self) -> &str {
        "null"
    }
}

/// Helper: convert LogLevel to a static string.
fn level_str(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
    }
}

// ══════════════════════════════════════════════════════════════
// Part B — The opaque handle wrapper
// ══════════════════════════════════════════════════════════════

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Define `LoggerHandle` — a **concrete, Sized** struct that wraps
// a `Box<dyn Logger>`.  This is the key pattern:
//
//   pub struct LoggerHandle {
//       inner: Box<dyn Logger>,
//   }
//
// Because `LoggerHandle` is `Sized`, `Box::into_raw(Box::new(handle))`
// produces a thin (single-word) pointer that C can store.
//
// WHY can't we just do `Box::into_raw(boxed_dyn)` directly?
// Because `Box<dyn Logger>` is a fat pointer — `into_raw` returns
// `*mut dyn Logger` which is TWO words.  C only understands
// single-word pointers.  The wrapper solves this by putting the fat
// pointer inside a concrete struct.

pub struct LoggerHandle {
    // TODO: add `inner: Box<dyn Logger>`
}

// ══════════════════════════════════════════════════════════════
// Part C — extern "C" API
// ══════════════════════════════════════════════════════════════

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Implement the FFI functions that C would call.
//
// Pattern:
//   1. `logger_new` — accept a `Box<dyn Logger>` (from Rust
//      constructors), wrap in LoggerHandle, return raw pointer.
//   2. `logger_log` — dereference the handle, call `.log()`.
//   3. `logger_flush` — dereference the handle, call `.flush()`.
//   4. `logger_name` — return the name as a C string.
//   5. `logger_free` — reclaim the Box, drop everything.
//
// Note: `logger_new` takes a Rust type (Box<dyn Logger>), not a
// C type.  In a real library you would also have C-facing
// constructors like `logger_new_print(...)`, etc.

/// Create a new logger handle from any `Box<dyn Logger>`.
///
/// This is the **Rust-side** constructor that packages a trait
/// object into a thin pointer.  In a real C API you'd also provide
/// C-facing constructors like:
///   `logger_new_print(name: *const c_char, ...) -> *mut LoggerHandle`
/// that internally call this function.
///
/// Note: We do NOT mark this `extern "C"` because `Box<dyn Logger>`
/// is a fat pointer and not FFI-safe.  Only the *returned handle*
/// (a thin `*mut LoggerHandle`) crosses the FFI boundary.
pub fn logger_new(boxed: Box<dyn Logger>) -> *mut LoggerHandle {
    todo!(
        "Wrap boxed in LoggerHandle, then Box::into_raw(Box::new(handle))"
    )
}

/// Write a log message through the trait object.
///
/// # Safety
/// - `handle` must be a valid pointer from `logger_new`.
/// - `message` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn logger_log(
    handle: *mut LoggerHandle,
    level: LogLevel,
    message: *const c_char,
) {
    todo!(
        "Dereference handle, convert message via CStr, \
         call inner.log(level, msg)"
    )
}

/// Flush the logger.
///
/// # Safety
/// `handle` must be a valid pointer from `logger_new`.
#[no_mangle]
pub unsafe extern "C" fn logger_flush(handle: *mut LoggerHandle) {
    todo!("Dereference handle, call inner.flush()")
}

/// Return the logger's name as a C string.
///
/// The returned pointer is valid only as long as the handle is alive.
/// The caller must NOT free it.
///
/// # Safety
/// `handle` must be a valid pointer from `logger_new`.
#[no_mangle]
pub unsafe extern "C" fn logger_name(
    handle: *const LoggerHandle,
) -> *const c_char {
    todo!(
        "Dereference handle, call inner.name(), \
         return as a pointer (hint: you'll need a CString stored \
         somewhere, or return a static &CStr).  \
         Simplest approach: store a CString in the LoggerHandle \
         alongside the inner trait object."
    )
}

/// Destroy the logger, freeing all resources.
///
/// # Safety
/// `handle` must have been returned by `logger_new` and must not
/// be used after this call.
#[no_mangle]
pub unsafe extern "C" fn logger_free(handle: *mut LoggerHandle) {
    todo!("Box::from_raw(handle) — dropping the Box drops inner too")
}

// ══════════════════════════════════════════════════════════════
// Part D — Composable logger (trait object owning trait object)
// ══════════════════════════════════════════════════════════════

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Implement `FilterLogger` — a logger that wraps another
// `Box<dyn Logger>` and only forwards messages at or above a
// minimum log level.
//
// This demonstrates that trait objects can OWN other trait objects,
// and the whole chain still goes through a single opaque handle.
//
//   struct FilterLogger {
//       min_level: LogLevel,
//       inner: Box<dyn Logger>,
//   }
//
// Implement `Logger` for `FilterLogger`:
//   - `log`: forward to `inner.log(...)` only if `level >= min_level`
//   - `flush`: forward to `inner.flush()`
//   - `name`: return something like "filter({inner_name})"

pub struct FilterLogger {
    // TODO: add fields for min_level and inner logger
}

impl FilterLogger {
    pub fn new(min_level: LogLevel, inner: Box<dyn Logger>) -> Self {
        todo!("Store min_level and inner")
    }
}

impl Logger for FilterLogger {
    fn log(&self, level: LogLevel, message: &str) {
        todo!("Only forward to inner if level >= self.min_level")
    }

    fn flush(&self) {
        todo!("Forward to inner.flush()")
    }

    fn name(&self) -> &str {
        todo!("Return a name like \"filter\" or build a dynamic name")
    }
}

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Implement `TeeLogger` — a logger that fans out to TWO inner
// loggers.  Both receive every message.
//
// This shows that a single opaque handle can hide an arbitrarily
// complex graph of trait objects.

pub struct TeeLogger {
    // TODO: add two Box<dyn Logger> fields
}

impl TeeLogger {
    pub fn new(a: Box<dyn Logger>, b: Box<dyn Logger>) -> Self {
        todo!("Store both loggers")
    }
}

impl Logger for TeeLogger {
    fn log(&self, level: LogLevel, message: &str) {
        todo!("Forward to both inner loggers")
    }

    fn flush(&self) {
        todo!("Flush both inner loggers")
    }

    fn name(&self) -> &str {
        todo!("Return a name like \"tee\"")
    }
}

// ══════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::sync::Arc;

    /// Helper: create a PrintLogger and its shared buffer.
    fn make_print_logger(name: &str) -> (PrintLogger, Arc<Mutex<Vec<String>>>) {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let logger = PrintLogger::new(name, buf.clone());
        (logger, buf)
    }

    // ── Part A: Basic trait implementations ───────────────────

    #[test]
    fn test_ex19_print_logger() {
        let (logger, buf) = make_print_logger("test");
        logger.log(LogLevel::Info, "hello");
        logger.log(LogLevel::Error, "oops");
        let msgs = buf.lock().unwrap();
        assert_eq!(msgs[0], "INFO: hello");
        assert_eq!(msgs[1], "ERROR: oops");
    }

    #[test]
    fn test_ex19_null_logger() {
        let logger = NullLogger;
        logger.log(LogLevel::Error, "this goes nowhere");
        assert_eq!(logger.name(), "null");
    }

    // ── Part B+C: Opaque handle round-trip ────────────────────

    #[test]
    fn test_ex19_handle_lifecycle() {
        let (logger, buf) = make_print_logger("handle-test");
        let boxed: Box<dyn Logger> = Box::new(logger);
        let handle = logger_new(boxed);
        assert!(!handle.is_null());

        let msg = CString::new("through C").unwrap();
        unsafe {
            logger_log(handle, LogLevel::Warn, msg.as_ptr());
        }

        {
            let msgs = buf.lock().unwrap();
            assert_eq!(msgs.len(), 1);
            assert_eq!(msgs[0], "WARN: through C");
        }

        unsafe { logger_free(handle) };
    }

    #[test]
    fn test_ex19_handle_multiple_messages() {
        let (logger, buf) = make_print_logger("multi");
        let handle = logger_new(Box::new(logger));

        let m1 = CString::new("first").unwrap();
        let m2 = CString::new("second").unwrap();
        let m3 = CString::new("third").unwrap();

        unsafe {
            logger_log(handle, LogLevel::Debug, m1.as_ptr());
            logger_log(handle, LogLevel::Info, m2.as_ptr());
            logger_log(handle, LogLevel::Error, m3.as_ptr());
        }

        let msgs = buf.lock().unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0], "DEBUG: first");
        assert_eq!(msgs[1], "INFO: second");
        assert_eq!(msgs[2], "ERROR: third");
        drop(msgs);

        unsafe { logger_free(handle) };
    }

    #[test]
    fn test_ex19_null_logger_handle() {
        let handle = logger_new(Box::new(NullLogger));
        let msg = CString::new("discarded").unwrap();
        unsafe {
            logger_log(handle, LogLevel::Error, msg.as_ptr());
            logger_flush(handle);
            logger_free(handle);
        }
    }

    // ── Part D: FilterLogger (trait object owning trait object) ──

    #[test]
    fn test_ex19_filter_logger() {
        let (logger, buf) = make_print_logger("filtered");
        let filtered = FilterLogger::new(
            LogLevel::Warn,
            Box::new(logger),
        );

        // Below threshold — should be suppressed
        filtered.log(LogLevel::Debug, "no");
        filtered.log(LogLevel::Info, "no");

        // At or above threshold — should pass through
        filtered.log(LogLevel::Warn, "yes-warn");
        filtered.log(LogLevel::Error, "yes-error");

        let msgs = buf.lock().unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0], "WARN: yes-warn");
        assert_eq!(msgs[1], "ERROR: yes-error");
    }

    #[test]
    fn test_ex19_filter_through_handle() {
        // FilterLogger wrapping a PrintLogger, all behind one
        // opaque handle — C sees a single *mut LoggerHandle.
        let (logger, buf) = make_print_logger("inner");
        let filtered = FilterLogger::new(LogLevel::Error, Box::new(logger));
        let handle = logger_new(Box::new(filtered));

        let m1 = CString::new("skipped").unwrap();
        let m2 = CString::new("passed").unwrap();

        unsafe {
            logger_log(handle, LogLevel::Info, m1.as_ptr());
            logger_log(handle, LogLevel::Error, m2.as_ptr());
        }

        let msgs = buf.lock().unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0], "ERROR: passed");
        drop(msgs);

        unsafe { logger_free(handle) };
    }

    // ── Part D: TeeLogger (fan-out to two trait objects) ──────

    #[test]
    fn test_ex19_tee_logger() {
        let (l1, buf1) = make_print_logger("left");
        let (l2, buf2) = make_print_logger("right");
        let tee = TeeLogger::new(Box::new(l1), Box::new(l2));

        tee.log(LogLevel::Info, "broadcast");

        assert_eq!(buf1.lock().unwrap()[0], "INFO: broadcast");
        assert_eq!(buf2.lock().unwrap()[0], "INFO: broadcast");
    }

    #[test]
    fn test_ex19_tee_through_handle() {
        let (l1, buf1) = make_print_logger("a");
        let (l2, buf2) = make_print_logger("b");
        let tee = TeeLogger::new(Box::new(l1), Box::new(l2));
        let handle = logger_new(Box::new(tee));

        let msg = CString::new("hello both").unwrap();
        unsafe {
            logger_log(handle, LogLevel::Warn, msg.as_ptr());
            logger_free(handle);
        }

        assert_eq!(buf1.lock().unwrap()[0], "WARN: hello both");
        assert_eq!(buf2.lock().unwrap()[0], "WARN: hello both");
    }

    // ── Composition: Filter + Tee ─────────────────────────────

    #[test]
    fn test_ex19_complex_composition() {
        // Build: Filter(Warn) → Tee → [PrintLogger A, PrintLogger B]
        // Only Warn+ messages reach both A and B.
        let (la, buf_a) = make_print_logger("A");
        let (lb, buf_b) = make_print_logger("B");
        let tee = TeeLogger::new(Box::new(la), Box::new(lb));
        let filtered = FilterLogger::new(LogLevel::Warn, Box::new(tee));

        let handle = logger_new(Box::new(filtered));

        let m1 = CString::new("debug msg").unwrap();
        let m2 = CString::new("warn msg").unwrap();
        let m3 = CString::new("error msg").unwrap();

        unsafe {
            logger_log(handle, LogLevel::Debug, m1.as_ptr());
            logger_log(handle, LogLevel::Warn, m2.as_ptr());
            logger_log(handle, LogLevel::Error, m3.as_ptr());
            logger_free(handle);
        }

        let a = buf_a.lock().unwrap();
        let b = buf_b.lock().unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(b.len(), 2);
        assert_eq!(a[0], "WARN: warn msg");
        assert_eq!(a[1], "ERROR: error msg");
        assert_eq!(b[0], "WARN: warn msg");
        assert_eq!(b[1], "ERROR: error msg");
    }
}
