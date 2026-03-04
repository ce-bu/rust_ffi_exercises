//! # Exercise 24: C-Owned Opaque Handles (out-parameter pattern)
//!
//! In **ex05** Rust owned the opaque handle and C just held a
//! pointer.  Here the roles are reversed: **C allocates and owns**
//! the internal struct, and Rust receives the handle through an
//! **out-parameter** — just like many real C libraries work:
//!
//! ```c
//! Session *s = NULL;
//! session_create(&s);           // C allocates, returns via out-param
//! session_connect(s, "host");   // s acts like C++ `this`
//! session_send(s, data, len);
//! session_destroy(s);           // C frees
//! ```
//!
//! This pattern is used by OpenSSL (`SSL_CTX_new`), libcurl
//! (`curl_easy_init`), SQLite (`sqlite3_open`), and many more.
//!
//! ## What you will practise
//!
//! 1. Declaring an **opaque C type** in Rust (zero-sized `#[repr(C)]`).
//! 2. The **out-parameter** pattern (`*mut *mut T`).
//! 3. Wrapping a C-owned handle in an RAII struct (`NonNull` + `Drop`).
//! 4. Converting C status codes to `Result`.
//! 5. Building a safe, idiomatic Rust API where the handle is `self`.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex24
//! ```

use std::ffi::{c_char, c_int, c_uchar, CString};
use std::ptr::NonNull;

// ── Opaque type ────────────────────────────────────────────────
//
// The C library declares `typedef struct Session Session;` —
// callers never see the internal fields.  In Rust we represent
// this as a zero-sized `#[repr(C)]` type that can only ever
// appear behind a pointer.

/// Opaque C-side session handle.
///
/// This type exists only as `*mut CSession`; you can never
/// construct or dereference it in Rust.
#[repr(C)]
pub struct CSession {
    _private: [u8; 0],
}

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Declare the `extern "C"` block with the following C functions
// from `csrc/ex24_session.c`:
//
//   int  session_create(Session **out);
//   int  session_set_option(Session *s, const char *key, const char *val);
//   int  session_get_option(Session *s, const char *key,
//                           char *out_value, size_t out_len);
//   int  session_connect(Session *s, const char *host);
//   int  session_is_connected(Session *s);
//   int  session_send(Session *s, const unsigned char *data, size_t len);
//   int  session_recv(Session *s, unsigned char *buf,
//                     size_t buf_len, size_t *out_len);
//   int  session_disconnect(Session *s);
//   void session_destroy(Session *s);
//
// Type-mapping hints:
//   Session **out           → out: *mut *mut CSession
//   const char *key         → key: *const c_char
//   const unsigned char *   → data: *const c_uchar
//   size_t *out_len         → out_len: *mut usize

extern "C" {
    // TODO: declare all 9 functions here
    fn session_create(out: *mut *mut CSession) -> c_int;
    fn session_set_option(s: *mut CSession, key: *const c_char, value: *const c_char) -> c_int;
    fn session_get_option(
        s: *mut CSession,
        key: *const c_char,
        out_value: *mut c_char,
        out_len: usize,
    ) -> c_int;
    fn session_connect(s: *mut CSession, host: *const c_char) -> c_int;
    fn session_is_connected(s: *mut CSession) -> c_int;
    fn session_send(s: *mut CSession, data: *const c_uchar, len: usize) -> c_int;
    fn session_recv(
        s: *mut CSession,
        buf: *mut c_uchar,
        buf_len: usize,
        out_len: *mut usize,
    ) -> c_int;
    fn session_disconnect(s: *mut CSession) -> c_int;
    fn session_destroy(s: *mut CSession);
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Define `SessionError` to map the C status codes:
//
//   SESSION_OK          ( 0) → not an error
//   SESSION_ERR_NULL   (-1) → NullPointer
//   SESSION_ERR_STATE  (-2) → InvalidState
//   SESSION_ERR_OVERFLOW(-3) → Overflow
//
// Implement `From<c_int> for SessionError` and a helper
// `check(code) -> Result<(), SessionError>`.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    // TODO: define variants:
    //   NullPointer,
    //   InvalidState,
    //   Overflow,
    //   Unknown(c_int),
    NullPointer,
    InvalidState,
    Overflow,
    Unknown(c_int),
}

impl std::fmt::Display for SessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!("Display each variant")
    }
}

impl std::error::Error for SessionError {}

impl From<c_int> for SessionError {
    fn from(code: c_int) -> Self {
        todo!("match code: -1 → NullPointer, -2 → InvalidState, -3 → Overflow, _ → Unknown")
    }
}

/// Convert a raw C return code (0 = OK, negative = error).
fn check(code: c_int) -> Result<(), SessionError> {
    todo!("if code == 0 return Ok(()), else Err(SessionError::from(code))")
}

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Build the safe `Session` wrapper.
//
// The struct stores a `NonNull<CSession>` — a non-null pointer
// to the C-owned opaque handle.
//
// ## Creating via out-parameter
//
// ```rust
// let mut ptr: *mut CSession = std::ptr::null_mut();
// check(unsafe { session_create(&mut ptr) })?;
// let handle = NonNull::new(ptr).ok_or(SessionError::NullPointer)?;
// ```
//
// ## Drop
//
// `Drop` calls `session_destroy` — C frees the memory it
// allocated.  Rust never calls `Box::from_raw` because it
// didn't allocate this memory.

pub struct Session {
    // TODO: store NonNull<CSession>
    handle: NonNull<CSession>,
}

impl Session {
    /// Create a new session by calling the C library.
    ///
    /// The C function writes the handle into an out-parameter:
    /// ```c
    /// int session_create(Session **out);
    /// ```
    pub fn new() -> Result<Self, SessionError> {
        todo!(
            "1. let mut ptr: *mut CSession = std::ptr::null_mut();\n\
             2. check(unsafe {{ session_create(&mut ptr) }})?;\n\
             3. NonNull::new(ptr).ok_or(SessionError::NullPointer)"
        )
    }

    /// Set a configuration option (before connecting).
    pub fn set_option(&mut self, key: &str, value: &str) -> Result<(), SessionError> {
        todo!("CString the key/value, call session_set_option")
    }

    /// Retrieve a previously-set option by key.
    pub fn get_option(&self, key: &str) -> Result<String, SessionError> {
        todo!(
            "Provide a stack buffer [0u8; 256],\n\
             call session_get_option,\n\
             convert the filled buffer to a String"
        )
    }

    /// Connect to the given host.
    pub fn connect(&mut self, host: &str) -> Result<(), SessionError> {
        todo!("CString the host, call session_connect")
    }

    /// Check whether the session is currently connected.
    pub fn is_connected(&self) -> bool {
        todo!("call session_is_connected, return true if non-zero")
    }

    /// Send a byte slice through the session.
    pub fn send(&mut self, data: &[u8]) -> Result<(), SessionError> {
        todo!("call session_send with data.as_ptr() and data.len()")
    }

    /// Receive bytes into `buf`.  Returns the number of bytes read.
    pub fn recv(&mut self, buf: &mut [u8]) -> Result<usize, SessionError> {
        todo!(
            "let mut out_len: usize = 0;\n\
             call session_recv,\n\
             return out_len on success"
        )
    }

    /// Disconnect the session (can reconnect later).
    pub fn disconnect(&mut self) -> Result<(), SessionError> {
        todo!("call session_disconnect")
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        todo!("unsafe {{ session_destroy(self.handle.as_ptr()) }}")
    }
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ex24_create_and_drop() {
        let sess = Session::new().expect("create failed");
        assert!(!sess.is_connected());
        // Drop runs here → session_destroy
    }

    #[test]
    fn test_ex24_set_and_get_option() {
        let mut sess = Session::new().unwrap();
        sess.set_option("timeout", "30").unwrap();
        sess.set_option("retries", "3").unwrap();
        assert_eq!(sess.get_option("timeout").unwrap(), "30");
        assert_eq!(sess.get_option("retries").unwrap(), "3");
    }

    #[test]
    fn test_ex24_option_not_found() {
        let sess = Session::new().unwrap();
        assert!(sess.get_option("missing").is_err());
    }

    #[test]
    fn test_ex24_cannot_set_option_after_connect() {
        let mut sess = Session::new().unwrap();
        sess.connect("localhost").unwrap();
        let err = sess.set_option("timeout", "10").unwrap_err();
        assert_eq!(err, SessionError::InvalidState);
    }

    #[test]
    fn test_ex24_connect_disconnect() {
        let mut sess = Session::new().unwrap();
        assert!(!sess.is_connected());
        sess.connect("example.com").unwrap();
        assert!(sess.is_connected());
        sess.disconnect().unwrap();
        assert!(!sess.is_connected());
    }

    #[test]
    fn test_ex24_double_connect_fails() {
        let mut sess = Session::new().unwrap();
        sess.connect("host1").unwrap();
        let err = sess.connect("host2").unwrap_err();
        assert_eq!(err, SessionError::InvalidState);
    }

    #[test]
    fn test_ex24_send_recv_echo() {
        let mut sess = Session::new().unwrap();
        sess.connect("localhost").unwrap();

        let message = b"hello FFI";
        sess.send(message).unwrap();

        let mut buf = [0u8; 64];
        let n = sess.recv(&mut buf).unwrap();
        assert_eq!(&buf[..n], message);
    }

    #[test]
    fn test_ex24_send_without_connect() {
        let mut sess = Session::new().unwrap();
        let err = sess.send(b"data").unwrap_err();
        assert_eq!(err, SessionError::InvalidState);
    }

    #[test]
    fn test_ex24_recv_empty() {
        let mut sess = Session::new().unwrap();
        sess.connect("localhost").unwrap();

        let mut buf = [0u8; 64];
        let n = sess.recv(&mut buf).unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn test_ex24_recv_buffer_too_small() {
        let mut sess = Session::new().unwrap();
        sess.connect("localhost").unwrap();
        sess.send(&[42u8; 100]).unwrap();

        let mut small = [0u8; 10]; // too small for 100 bytes
        let err = sess.recv(&mut small).unwrap_err();
        assert_eq!(err, SessionError::Overflow);
    }

    #[test]
    fn test_ex24_full_lifecycle() {
        let mut sess = Session::new().unwrap();

        // Configure
        sess.set_option("mode", "echo").unwrap();
        assert_eq!(sess.get_option("mode").unwrap(), "echo");

        // Connect
        sess.connect("10.0.0.1").unwrap();
        assert!(sess.is_connected());

        // Send + receive
        sess.send(b"ping").unwrap();
        let mut buf = [0u8; 64];
        let n = sess.recv(&mut buf).unwrap();
        assert_eq!(&buf[..n], b"ping");

        // Disconnect
        sess.disconnect().unwrap();
        assert!(!sess.is_connected());
        // Drop cleans up
    }
}
