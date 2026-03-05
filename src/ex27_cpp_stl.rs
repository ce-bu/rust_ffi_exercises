// src/ex27_cpp_stl.rs
//
// Safe Rust wrappers around a C++ `CppStringStack` class that
// uses std::vector<std::string> internally.  Demonstrates:
//
// - Opaque pointer pattern (create / destroy / clone)
// - std::string ↔ &str / String conversion
// - Caller-provided buffer for owned string output
// - **Borrowed pointer return** (peek — zero-copy, with lifetime)
// - Callback-based iteration
// - Batch insertion (push_many)
// - Factory function (from CSV)
//
// The C++ class is never exposed directly; Rust only ever holds
// an opaque `*mut CppStringStack` inside the RAII wrapper.

use std::ffi::{c_char, c_void};

/* ══════════════════════════════════════════════════════════════
 * Error codes (must match C++ defines)
 * ══════════════════════════════════════════════════════════════ */

pub const CPP_STK_OK: i32 = 0;
pub const CPP_STK_ERR_EMPTY: i32 = -1;
pub const CPP_STK_ERR_BUF: i32 = -2;
pub const CPP_STK_ERR_NULL: i32 = -3;
pub const CPP_STK_ERR_OOM: i32 = -4;
#[allow(dead_code)]
pub const CPP_STK_ERR_OTHER: i32 = -99;

/* ══════════════════════════════════════════════════════════════
 * Opaque C++ type (never instantiated on the Rust side)
 * ══════════════════════════════════════════════════════════════ */

#[repr(C)]
pub struct CppStringStack {
    _private: [u8; 0],
}

/* ══════════════════════════════════════════════════════════════
 * extern "C" declarations
 * ══════════════════════════════════════════════════════════════ */

type CppStkIterFn = extern "C" fn(*const c_char, usize, *mut c_void);

extern "C" {
    fn cpp_stk_new() -> *mut CppStringStack;
    fn cpp_stk_destroy(s: *mut CppStringStack);
    fn cpp_stk_clone(s: *const CppStringStack) -> *mut CppStringStack;

    fn cpp_stk_push(s: *mut CppStringStack, str: *const c_char, len: usize) -> i32;
    fn cpp_stk_pop(
        s: *mut CppStringStack,
        out_buf: *mut c_char,
        buf_len: usize,
        out_len: *mut usize,
    ) -> i32;
    fn cpp_stk_peek(
        s: *const CppStringStack,
        out_ptr: *mut *const c_char,
        out_len: *mut usize,
    ) -> i32;
    fn cpp_stk_size(s: *const CppStringStack) -> usize;

    fn cpp_stk_join(
        s: *const CppStringStack,
        sep: *const c_char,
        sep_len: usize,
        out_buf: *mut c_char,
        buf_len: usize,
        out_len: *mut usize,
    ) -> i32;

    fn cpp_stk_push_many(
        s: *mut CppStringStack,
        strings: *const *const c_char,
        lengths: *const usize,
        count: usize,
    ) -> i32;

    fn cpp_stk_for_each(
        s: *const CppStringStack,
        callback: CppStkIterFn,
        ctx: *mut c_void,
    );

    fn cpp_stk_from_csv(csv: *const c_char, len: usize) -> *mut CppStringStack;
}

/* ══════════════════════════════════════════════════════════════
 * Error type
 * ══════════════════════════════════════════════════════════════ */

#[derive(Debug, Clone)]
pub enum StackError {
    Empty,
    BufferTooSmall { needed: usize },
    NullPointer,
    OutOfMemory,
    Other(i32),
}

impl std::fmt::Display for StackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "stack is empty"),
            Self::BufferTooSmall { needed } => {
                write!(f, "buffer too small (need {} bytes)", needed)
            }
            Self::NullPointer => write!(f, "null pointer"),
            Self::OutOfMemory => write!(f, "out of memory"),
            Self::Other(code) => write!(f, "unknown error (code {})", code),
        }
    }
}

impl std::error::Error for StackError {}

fn map_err(code: i32) -> StackError {
    match code {
        CPP_STK_ERR_EMPTY => StackError::Empty,
        CPP_STK_ERR_BUF => StackError::BufferTooSmall { needed: 0 },
        CPP_STK_ERR_NULL => StackError::NullPointer,
        CPP_STK_ERR_OOM => StackError::OutOfMemory,
        other => StackError::Other(other),
    }
}

/* ══════════════════════════════════════════════════════════════
 * StringStack — safe RAII wrapper
 * ══════════════════════════════════════════════════════════════ */

/// Owns a C++ `CppStringStack` via an opaque pointer.
/// Destroyed automatically on drop.
pub struct StringStack {
    ptr: *mut CppStringStack,
}

// Safety: the C++ class has no thread affinity.
unsafe impl Send for StringStack {}

impl Drop for StringStack {
    fn drop(&mut self) {
        unsafe { cpp_stk_destroy(self.ptr) }
    }
}

impl Clone for StringStack {
    fn clone(&self) -> Self {
        let p = unsafe { cpp_stk_clone(self.ptr) };
        assert!(!p.is_null(), "CppStringStack clone failed (OOM)");
        Self { ptr: p }
    }
}

impl StringStack {
    /* ── Construction ───────────────────────────────────────── */

    /// Create a new, empty stack.
    pub fn new() -> Self {
        let p = unsafe { cpp_stk_new() };
        assert!(!p.is_null(), "CppStringStack allocation failed");
        Self { ptr: p }
    }

    /// Factory: parse a CSV string into stack items.
    ///
    /// `"alpha, bravo, charlie"` →  stack with three items
    /// (bottom → top: alpha, bravo, charlie).
    pub fn from_csv(csv: &str) -> Self {
        let p = unsafe { cpp_stk_from_csv(csv.as_ptr() as *const c_char, csv.len()) };
        assert!(!p.is_null(), "CppStringStack::from_csv failed");
        Self { ptr: p }
    }

    /* ── Basic operations ───────────────────────────────────── */

    pub fn push(&mut self, s: &str) {
        let rc = unsafe { cpp_stk_push(self.ptr, s.as_ptr() as *const c_char, s.len()) };
        assert_eq!(rc, CPP_STK_OK);
    }

    /// Pop the top element.  Returns the string or `StackError::Empty`.
    pub fn pop(&mut self) -> Result<String, StackError> {
        // Two-phase: first call to get needed length, second to fill.
        let mut needed: usize = 0;
        let rc = unsafe { cpp_stk_pop(self.ptr, std::ptr::null_mut(), 0, &mut needed) };
        if rc == CPP_STK_ERR_EMPTY {
            return Err(StackError::Empty);
        }
        // rc == CPP_STK_ERR_BUF means "buffer too small, needed written"
        // Allocate the right size.
        let buf_len = needed + 1; // +1 for NUL
        let mut buf = vec![0u8; buf_len];
        let rc = unsafe {
            cpp_stk_pop(
                self.ptr,
                buf.as_mut_ptr() as *mut c_char,
                buf_len,
                &mut needed,
            )
        };
        if rc != CPP_STK_OK {
            return Err(map_err(rc));
        }
        buf.truncate(needed);
        // The C++ side wrote UTF-8 (or at least ASCII); use lossy for safety.
        Ok(String::from_utf8(buf).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned()))
    }

    /// Number of items on the stack.
    pub fn len(&self) -> usize {
        unsafe { cpp_stk_size(self.ptr) }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /* ── Peek — borrowed return ────────────────────────────── */

    /// Peek at the top element **without copying**.
    ///
    /// Returns a `PeekGuard` that borrows `self` immutably,
    /// preventing mutation (and thus pointer invalidation)
    /// for the guard's lifetime.
    pub fn peek(&self) -> Result<PeekGuard<'_>, StackError> {
        let mut ptr: *const c_char = std::ptr::null();
        let mut len: usize = 0;
        let rc = unsafe { cpp_stk_peek(self.ptr, &mut ptr, &mut len) };
        if rc != CPP_STK_OK {
            return Err(map_err(rc));
        }
        Ok(PeekGuard {
            ptr: ptr as *const u8,
            len,
            _stack: self,
        })
    }

    /* ── Bulk / higher-level ────────────────────────────────── */

    /// Join all elements with a separator.
    pub fn join(&self, sep: &str) -> String {
        // First call to get the length.
        let mut needed: usize = 0;
        let _ = unsafe {
            cpp_stk_join(
                self.ptr,
                sep.as_ptr() as *const c_char,
                sep.len(),
                std::ptr::null_mut(),
                0,
                &mut needed,
            )
        };
        let buf_len = needed + 1;
        let mut buf = vec![0u8; buf_len];
        let rc = unsafe {
            cpp_stk_join(
                self.ptr,
                sep.as_ptr() as *const c_char,
                sep.len(),
                buf.as_mut_ptr() as *mut c_char,
                buf_len,
                &mut needed,
            )
        };
        assert_eq!(rc, CPP_STK_OK, "join failed unexpectedly");
        buf.truncate(needed);
        String::from_utf8(buf).unwrap_or_default()
    }

    /// Push multiple strings at once.
    pub fn push_many(&mut self, items: &[&str]) {
        let ptrs: Vec<*const c_char> = items
            .iter()
            .map(|s| s.as_ptr() as *const c_char)
            .collect();
        let lens: Vec<usize> = items.iter().map(|s| s.len()).collect();
        let rc = unsafe {
            cpp_stk_push_many(self.ptr, ptrs.as_ptr(), lens.as_ptr(), items.len())
        };
        assert_eq!(rc, CPP_STK_OK);
    }

    /// Iterate all elements (bottom to top) via a callback.
    /// Collects into a `Vec<String>`.
    pub fn to_vec(&self) -> Vec<String> {
        let mut result: Vec<String> = Vec::new();

        extern "C" fn collect(s: *const c_char, len: usize, ctx: *mut c_void) {
            let vec = unsafe { &mut *(ctx as *mut Vec<String>) };
            let bytes = unsafe { std::slice::from_raw_parts(s as *const u8, len) };
            vec.push(String::from_utf8_lossy(bytes).into_owned());
        }

        unsafe {
            cpp_stk_for_each(
                self.ptr,
                collect,
                &mut result as *mut Vec<String> as *mut c_void,
            );
        }
        result
    }

    /// Get the raw pointer (for advanced use / tests).
    pub fn as_ptr(&self) -> *const CppStringStack {
        self.ptr
    }
}

impl Default for StringStack {
    fn default() -> Self {
        Self::new()
    }
}

/* ══════════════════════════════════════════════════════════════
 * PeekGuard — encodes C++ borrow lifetime in Rust's type system
 * ══════════════════════════════════════════════════════════════ */

/// A zero-copy view into the top element of a `StringStack`.
///
/// The guard borrows the stack immutably, so the compiler
/// prevents any `&mut self` call (push, pop) that could
/// invalidate the pointer.
pub struct PeekGuard<'a> {
    ptr: *const u8,
    len: usize,
    _stack: &'a StringStack,
}

impl<'a> PeekGuard<'a> {
    /// View the peeked string as a `&str`.
    pub fn as_str(&self) -> &str {
        let bytes = unsafe { std::slice::from_raw_parts(self.ptr, self.len) };
        std::str::from_utf8(bytes).unwrap_or("<invalid utf-8>")
    }

    /// Length in bytes.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl<'a> std::fmt::Display for PeekGuard<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'a> std::fmt::Debug for PeekGuard<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PeekGuard({:?})", self.as_str())
    }
}
