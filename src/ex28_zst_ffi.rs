//! # Exercise 28: Zero-Sized Types in FFI
//!
//! **Concept:** Rust has zero-sized types (ZSTs): types with
//! `size_of::<T>() == 0`.  Common ZSTs include `()`, `PhantomData<T>`,
//! and structs with no fields.  ZSTs are powerful in generic and
//! type-level programming, but they interact with FFI in subtle and
//! sometimes dangerous ways.
//!
//! ## Key points
//!
//! 1. **C has no ZSTs.**  In C, `sizeof(struct S)` is always ≥ 1
//!    (even for an empty struct).  A `#[repr(C)]` struct with no
//!    fields has size 0 in Rust — a layout mismatch with C.
//!    **Never pass a `#[repr(C)]` ZST by value across FFI.**
//!
//! 2. **Opaque incomplete types.**  The `[u8; 0]` pattern (or the
//!    nightly `extern type`) is the standard way to declare a C
//!    opaque type in Rust — zero-sized, non-constructible, used
//!    only behind a pointer.
//!
//! 3. **`PhantomData` for type-safe handles.**  By adding a phantom
//!    type parameter, you can make `Handle<Database>` and
//!    `Handle<Cursor>` incompatible at the type level — while the
//!    FFI layer only ever sees `*mut c_void`.
//!
//! 4. **`PhantomData` for Send/Sync control.**  A wrapper holding
//!    a raw pointer is `Send + Sync` by default only if you
//!    explicitly opt in.  `PhantomData<*mut T>` (which is `!Send`
//!    and `!Sync`) or `PhantomData<Rc<T>>` can be used to tighten
//!    the auto-trait bounds.
//!
//! 5. **Typestate pattern.**  ZST marker types can encode protocol
//!    states (Connected vs Disconnected) so that calling methods in
//!    the wrong state is a **compile-time** error.
//!
//! ## Your task
//!
//!  A. Demonstrate the `#[repr(C)]` ZST size pitfall.
//!  B. Define an opaque type using the `[u8; 0]` idiom.
//!  C. Build type-safe handles with `PhantomData` markers.
//!  D. Use `PhantomData` to control `Send`/`Sync`.
//!  E. Implement a typestate connection wrapper.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex28
//! ```

use std::ffi::c_void;
use std::marker::PhantomData;

// ══════════════════════════════════════════════════════════════
// Part A — The #[repr(C)] ZST pitfall
// ══════════════════════════════════════════════════════════════

// ── TODO 1 ─────────────────────────────────────────────────────
//
// In C, `struct Empty {}` has `sizeof == 1` (C++ rule) or is
// disallowed / implementation-defined (C).  In Rust, a
// `#[repr(C)]` struct with no fields has `size_of == 0`.
// This means the layouts **disagree**.
//
// Demonstrate this by defining two structs:
//
//   #[repr(C)]
//   pub struct ReprCEmpty;          // size 0 in Rust
//
//   #[repr(C)]
//   pub struct ReprCNonEmpty {
//       _padding: u8,              // size 1, matches C
//   }
//
// The tests will verify the sizes and remind you of the pitfall.
//
// Your code here:

// TODO: define ReprCEmpty — a #[repr(C)] struct with no fields
//        and ReprCNonEmpty — a #[repr(C)] struct with a `_padding: u8` field.
#[repr(C)]
pub struct ReprCEmpty;

#[repr(C)]
pub struct ReprCNonEmpty {
    pub _padding: u8,
}

// ══════════════════════════════════════════════════════════════
// Part B — Opaque incomplete types using [u8; 0]
// ══════════════════════════════════════════════════════════════

// ── TODO 2 ─────────────────────────────────────────────────────
//
// When wrapping a C library, you often have an opaque type that
// C only exposes through pointers:
//
//   // C header:
//   typedef struct sqlite3 sqlite3;  // incomplete type
//
// In Rust you cannot declare an incomplete type directly, but
// you can approximate it with a non-constructible ZST:
//
//   #[repr(C)]
//   pub struct Sqlite3 {
//       _opaque: [u8; 0],
//       _marker: PhantomData<(*mut u8, std::cell::UnsafeCell<u8>)>,
//   }
//
// The `[u8; 0]` makes the type zero-sized but `#[repr(C)]`.
// The `PhantomData` opts out of `Send` and `Sync` (conservative).
// Nobody can construct a `Sqlite3` value on the stack — you can
// only have `*mut Sqlite3`.
//
// Define an opaque type `OpaqueEngine` following this pattern,
// and write a simulated create/destroy cycle using
// `Box::into_raw(Box::new(vec![1,2,3]))` cast to `*mut OpaqueEngine`.
//
// Hint: This is safe because we only use it behind a pointer and
// the real storage is the Vec on the heap; the `OpaqueEngine` type
// is just a "brand" on the pointer.
//
// Your code here:

// TODO: study this opaque type definition.
//
// `[u8; 0]` makes it zero-sized and non-constructible outside
// this module.  The PhantomData opts out of Send/Sync.
#[repr(C)]
pub struct OpaqueEngine {
    _opaque: [u8; 0],
    _marker: PhantomData<(*mut u8, std::cell::UnsafeCell<u8>)>,
}

/// Simulated "C library" create — returns an opaque pointer.
///
/// # Safety
/// Caller must later call `opaque_engine_destroy` to free.
pub unsafe fn opaque_engine_create() -> *mut OpaqueEngine {
    // Allocate some real data and cast the pointer
    let data: Vec<i32> = vec![10, 20, 30];
    Box::into_raw(Box::new(data)) as *mut OpaqueEngine
}

/// Simulated "C library" sum — reads through the opaque pointer.
///
/// # Safety
/// `engine` must be a valid pointer from `opaque_engine_create`.
pub unsafe fn opaque_engine_sum(engine: *const OpaqueEngine) -> i32 {
    let data = &*(engine as *const Vec<i32>);
    data.iter().sum()
}

/// Simulated "C library" destroy.
///
/// # Safety
/// `engine` must be a valid pointer from `opaque_engine_create`.
/// Must not be used after this call.
pub unsafe fn opaque_engine_destroy(engine: *mut OpaqueEngine) {
    let _ = Box::from_raw(engine as *mut Vec<i32>);
}

// ══════════════════════════════════════════════════════════════
// Part C — Type-safe handles with PhantomData markers
// ══════════════════════════════════════════════════════════════

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Many C libraries return `void*` handles for different resource
// types.  Without care, Rust code can mix them up:
//
//   let db: *mut c_void = db_open();
//   let cursor: *mut c_void = cursor_open(db);
//   db_close(cursor);  // BUG! passed cursor to db_close
//
// Fix this with a generic `Handle<T>` wrapper that brands the
// pointer with a phantom type parameter:
//
//   pub struct Handle<Tag> {
//       raw: *mut c_void,
//       _tag: PhantomData<Tag>,
//   }
//
// Define two **empty** (ZST) tag types:
//
//   pub struct DbTag;
//   pub struct CursorTag;
//
// And type aliases:
//
//   pub type DbHandle = Handle<DbTag>;
//   pub type CursorHandle = Handle<CursorTag>;
//
// Implement:
//   - `Handle::new(raw: *mut c_void) -> Self`
//   - `Handle::raw(&self) -> *mut c_void`
//
// The tests will verify that `DbHandle` and `CursorHandle` are
// different types and cannot be accidentally swapped.
//
// Your code here:

// TODO: implement Handle<Tag>, DbTag, CursorTag, type aliases,
//       and the new() / raw() methods.
//
// Hint:
//   pub struct DbTag;
//   pub struct CursorTag;
//   pub struct Handle<Tag> { raw: *mut c_void, _tag: PhantomData<Tag> }
//   pub type DbHandle = Handle<DbTag>;
//   pub type CursorHandle = Handle<CursorTag>;

pub struct DbTag;
pub struct CursorTag;

pub struct Handle<Tag> {
    raw: *mut c_void,
    _tag: PhantomData<Tag>,
}

pub type DbHandle = Handle<DbTag>;
pub type CursorHandle = Handle<CursorTag>;

impl<Tag> Handle<Tag> {
    pub fn new(raw: *mut c_void) -> Self {
        todo!("TODO 3: construct Handle from raw pointer")
    }

    pub fn raw(&self) -> *mut c_void {
        todo!("TODO 3: return the raw pointer")
    }
}

// ══════════════════════════════════════════════════════════════
// Part D — PhantomData for Send/Sync control
// ══════════════════════════════════════════════════════════════

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Raw pointers are `!Send` and `!Sync`.  But a struct containing
// a raw pointer doesn't automatically inherit those negative
// bounds — Rust's auto-trait rules say:
//
//   - A struct is `Send` if all its fields are `Send`.
//   - Raw pointers are `!Send` and `!Sync`.
//
// So a struct with a raw pointer field is already `!Send`/`!Sync`
// by default.  But what if you use an integer handle (like a file
// descriptor `u64`) instead of a pointer?  Then the auto-traits
// would make it `Send + Sync`, which may be wrong.
//
// Use `PhantomData<*mut ()>` to explicitly opt out:
//
//   pub struct ThreadUnsafeHandle {
//       fd: u64,
//       _not_send_sync: PhantomData<*mut ()>,
//   }
//
// And for comparison, define a handle that IS Send + Sync:
//
//   pub struct ThreadSafeHandle {
//       fd: u64,
//   }
//   unsafe impl Send for ThreadSafeHandle {}
//   unsafe impl Sync for ThreadSafeHandle {}
//
// Your code here:

// TODO: define ThreadUnsafeHandle with PhantomData<*mut ()>
//       and ThreadSafeHandle with explicit Send + Sync impls.

pub struct ThreadUnsafeHandle {
    pub fd: u64,
    _not_send_sync: PhantomData<*mut ()>,
}

impl ThreadUnsafeHandle {
    pub fn new(fd: u64) -> Self {
        todo!("TODO 4: construct ThreadUnsafeHandle")
    }
}

pub struct ThreadSafeHandle {
    pub fd: u64,
}

unsafe impl Send for ThreadSafeHandle {}
unsafe impl Sync for ThreadSafeHandle {}

impl ThreadSafeHandle {
    pub fn new(fd: u64) -> Self {
        todo!("TODO 4: construct ThreadSafeHandle")
    }
}

// ══════════════════════════════════════════════════════════════
// Part E — Typestate pattern with ZST markers
// ══════════════════════════════════════════════════════════════

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Many C libraries have protocol states:
//
//   session = session_create();
//   session_connect(session, "host");
//   session_send(session, data);       // only valid after connect
//   session_disconnect(session);
//   session_destroy(session);
//
// Calling `session_send` before `session_connect` is a runtime
// error.  In Rust we can make it a **compile-time** error using
// the typestate pattern:
//
//   pub struct Disconnected;  // ZST marker
//   pub struct Connected;     // ZST marker
//
//   pub struct Session<State> {
//       id: u64,
//       _state: PhantomData<State>,
//   }
//
// - `Session::create()` returns `Session<Disconnected>`
// - `Session<Disconnected>::connect()` consumes self, returns
//     `Session<Connected>`
// - `Session<Connected>::send()` takes `&self` (only callable
//     when connected)
// - `Session<Connected>::disconnect()` consumes self, returns
//     `Session<Disconnected>`
// - `Drop` is implemented for all states
//
// Implement this pattern.  The `send` method can just record
// the message in a `static` counter (or simply succeed).
//
// Your code here:

pub struct Disconnected;
pub struct Connected;

pub struct Session<State> {
    id: u64,
    _state: PhantomData<State>,
}

impl Session<Disconnected> {
    /// Create a new session in the Disconnected state.
    pub fn create(id: u64) -> Self {
        todo!("TODO 5: create a Session<Disconnected>")
    }

    /// Connect — consumes a Disconnected session, returns Connected.
    pub fn connect(self) -> Session<Connected> {
        todo!("TODO 5: transition Disconnected → Connected")
    }
}

impl Session<Connected> {
    /// Send a message (only available when Connected).
    pub fn send(&self, _msg: &str) {
        todo!("TODO 5: increment SEND_COUNT")
    }

    /// Disconnect — consumes a Connected session, returns Disconnected.
    pub fn disconnect(self) -> Session<Disconnected> {
        todo!("TODO 5: transition Connected → Disconnected")
    }
}

/// Global counter for sent messages (for test verification).
static SEND_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Reset the send counter (for testing).
pub fn reset_send_count() {
    SEND_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
}

/// Read the send counter.
pub fn get_send_count() -> u64 {
    SEND_COUNT.load(std::sync::atomic::Ordering::SeqCst)
}

// ══════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    // ── Part A tests ───────────────────────────────────────────

    #[test]
    fn test_ex28_repr_c_empty_is_zero() {
        // Rust's #[repr(C)] empty struct is size 0 — unlike C/C++ where it's 1.
        assert_eq!(mem::size_of::<ReprCEmpty>(), 0);
    }

    #[test]
    fn test_ex28_repr_c_padded_is_one() {
        // With a u8 padding field, it matches C's sizeof.
        assert_eq!(mem::size_of::<ReprCNonEmpty>(), 1);
    }

    // ── Part B tests ───────────────────────────────────────────

    #[test]
    fn test_ex28_opaque_engine_size() {
        // The opaque type itself is zero-sized.
        assert_eq!(mem::size_of::<OpaqueEngine>(), 0);
        // But a pointer to it is pointer-sized.
        assert_eq!(
            mem::size_of::<*mut OpaqueEngine>(),
            mem::size_of::<*mut c_void>()
        );
    }

    #[test]
    fn test_ex28_opaque_engine_lifecycle() {
        unsafe {
            let engine = opaque_engine_create();
            assert!(!engine.is_null());
            assert_eq!(opaque_engine_sum(engine), 60); // 10+20+30
            opaque_engine_destroy(engine);
        }
    }

    // ── Part C tests ───────────────────────────────────────────

    #[test]
    fn test_ex28_handle_is_pointer_sized() {
        assert_eq!(mem::size_of::<DbHandle>(), mem::size_of::<*mut c_void>());
        assert_eq!(
            mem::size_of::<CursorHandle>(),
            mem::size_of::<*mut c_void>()
        );
    }

    #[test]
    fn test_ex28_handle_roundtrip() {
        let fake: *mut c_void = 0xDEAD_BEEF as *mut c_void;
        let db = DbHandle::new(fake);
        assert_eq!(db.raw(), fake);

        let cursor = CursorHandle::new(fake);
        assert_eq!(cursor.raw(), fake);
    }

    #[test]
    fn test_ex28_handles_are_distinct_types() {
        // This is a compile-time guarantee.  We verify at runtime
        // that the TypeId differs.
        use std::any::TypeId;
        assert_ne!(TypeId::of::<DbHandle>(), TypeId::of::<CursorHandle>());
    }

    // ── Part D tests ───────────────────────────────────────────

    #[test]
    fn test_ex28_thread_unsafe_handle_not_send() {
        fn assert_not_send<T>()
        where
        // Using a trick: if T: Send compiled, this function
        // would exist.  We test the *negative* with a helper.
        {
        }
        // We can't assert !Send directly in a test, but we can
        // check the size is correct (the PhantomData is ZST).
        assert_eq!(mem::size_of::<ThreadUnsafeHandle>(), mem::size_of::<u64>());
    }

    #[test]
    fn test_ex28_thread_safe_handle_is_send() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<ThreadSafeHandle>();
        assert_sync::<ThreadSafeHandle>();
        assert_eq!(mem::size_of::<ThreadSafeHandle>(), mem::size_of::<u64>());
    }

    // ── Part E tests ───────────────────────────────────────────

    #[test]
    fn test_ex28_typestate_lifecycle() {
        reset_send_count();

        let session = Session::<Disconnected>::create(42);
        // session.send("hello");  // ← would NOT compile!

        let session = session.connect();
        session.send("hello");
        session.send("world");
        assert_eq!(get_send_count(), 2);

        let session = session.disconnect();
        // session.send("oops");  // ← would NOT compile!
        drop(session);
    }

    #[test]
    fn test_ex28_typestate_zero_sized_markers() {
        assert_eq!(mem::size_of::<Disconnected>(), 0);
        assert_eq!(mem::size_of::<Connected>(), 0);
        // Session<State> should be the same size regardless of state,
        // because the state is a ZST.
        assert_eq!(
            mem::size_of::<Session<Disconnected>>(),
            mem::size_of::<Session<Connected>>()
        );
    }

    #[test]
    fn test_ex28_session_size() {
        // Session should hold just the u64 id — the PhantomData
        // adds no runtime cost.
        assert_eq!(
            mem::size_of::<Session<Disconnected>>(),
            mem::size_of::<u64>()
        );
    }
}
