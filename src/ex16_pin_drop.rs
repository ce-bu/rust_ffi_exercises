//! # Exercise 16: Pin, Drop Guarantees, and FFI
//!
//! **Concept:** When C code holds a pointer to a Rust object, that
//! object **must not move** — otherwise C's pointer dangles.  Rust's
//! `Pin<P>` type encodes this invariant at the type level.
//!
//! ## The problem
//!
//! Consider a Rust struct that *registers itself* with a C library
//! (e.g., a callback registry, event loop, or linked list).  The C
//! side stores a raw pointer back to the struct.  If Rust moves the
//! struct (via `mem::swap`, `Vec` realloc, returning from a function,
//! etc.), the C pointer is invalidated → **use-after-free**.
//!
//! ## The solution: Pin + Drop guarantee
//!
//! 1. **`Pin<Box<T>>`** guarantees the heap allocation won't move.
//! 2. **`Drop`** must *deregister* from C before the memory is freed.
//! 3. These two together form the **drop guarantee**: once pinned,
//!    `Drop::drop()` is always called before deallocation, giving
//!    the struct a chance to clean up its C-side registration.
//!
//! ## Simulated C registry
//!
//! This exercise simulates a C library that maintains a global list
//! of registered "listeners."  Each listener is a raw pointer that
//! C dereferences when dispatching events.
//!
//! ## Your task
//!
//! 1. Implement a `Listener` that registers/deregisters with the
//!    simulated C registry.
//! 2. Ensure it is always behind `Pin<Box<…>>` so it can't move.
//! 3. Implement `Drop` to deregister, upholding the drop guarantee.
//! 4. Implement a `PinnedListener` safe wrapper that makes misuse
//!    impossible.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex16
//! ```

use std::pin::Pin;
use std::marker::PhantomPinned;
use std::sync::Mutex;

// ══════════════════════════════════════════════════════════════
// Simulated C registry (pre-provided — do NOT modify)
// ══════════════════════════════════════════════════════════════
//
// In a real project this would be C code.  Here we simulate it
// in Rust so the exercise is self-contained and Miri can check it.

/// Represents one entry the C side knows about.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CListenerEntry {
    /// Raw pointer to the Rust `Listener` struct.
    pub ptr: *const Listener,
    /// Unique id assigned at registration.
    pub id: u64,
}

// SAFETY: The pointers are only dereferenced on the same thread in tests.
unsafe impl Send for CListenerEntry {}

static REGISTRY: Mutex<Vec<CListenerEntry>> = Mutex::new(Vec::new());
static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Simulates `c_register_listener(ptr)` → returns an id.
///
/// The C side stores `ptr` and will dereference it later during
/// `c_dispatch`.  The pointer MUST remain valid until deregistered.
pub fn c_register(ptr: *const Listener) -> u64 {
    let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    REGISTRY.lock().unwrap().push(CListenerEntry { ptr, id });
    id
}

/// Simulates `c_deregister_listener(id)`.  Removes the entry so
/// the pointer is no longer used.
pub fn c_deregister(id: u64) {
    REGISTRY.lock().unwrap().retain(|e| e.id != id);
}

/// Simulates C dispatching an event to all registered listeners.
/// Returns the collected results.
///
/// # Safety
/// Every registered pointer must still be valid (not moved/freed).
pub unsafe fn c_dispatch(event: i32) -> Vec<String> {
    let reg = REGISTRY.lock().unwrap();
    let mut results = Vec::new();
    for entry in reg.iter() {
        // C dereferences the raw pointer here!
        let listener = &*entry.ptr;
        results.push(format!("{}:{}", listener.name, event * listener.weight));
    }
    results
}

/// Returns the number of currently registered listeners.
pub fn c_registry_len() -> usize {
    REGISTRY.lock().unwrap().len()
}

/// Clears the entire registry (for test isolation).
pub fn c_registry_clear() {
    REGISTRY.lock().unwrap().clear();
}

// ══════════════════════════════════════════════════════════════
// The Listener struct
// ══════════════════════════════════════════════════════════════

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Complete the `Listener` struct:
//
//  - `name: String`     — identifier for this listener.
//  - `weight: i32`      — multiplier applied to events.
//  - `registry_id: Option<u64>` — the id returned by `c_register`,
//       or `None` if not yet registered.
//  - `_pin: PhantomPinned` — makes this type `!Unpin`, so it
//       cannot be moved out of a `Pin`.
//
// `PhantomPinned` is a zero-sized marker that opts OUT of `Unpin`.
// Without it, `Pin<Box<Listener>>` would still allow moves.

pub struct Listener {
    pub name: String,
    pub weight: i32,
    // TODO: add `registry_id` and `_pin` fields
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Implement an associated function that creates a `Listener`
// **already pinned on the heap** and registered with the C
// registry.
//
// Steps:
//   1. Create the struct with `registry_id: None`.
//   2. Box it: `Box::pin(listener)`.
//      — Wait, we need to register AFTER pinning so we can take
//        the stable pointer.  But `Pin<Box<T>>` prevents &mut
//        access when T: !Unpin…
//
//   The trick:
//     a) Box::new(listener) first.
//     b) Get the stable pointer: `let ptr: *const Listener = &*boxed;`
//     c) Register with C: `c_register(ptr)` → id.
//     d) Set registry_id: `boxed.registry_id = Some(id);`
//     e) Convert to Pin: `Pin::new_unchecked(boxed)`
//        (safe here because we won't move it again).
//
// Return: `Pin<Box<Listener>>`

impl Listener {
    /// Create a pinned, registered listener.
    pub fn new_pinned(name: &str, weight: i32) -> Pin<Box<Listener>> {
        todo!(
            "Create Listener, Box it, register the stable pointer \
             with c_register, set registry_id, then Pin::new_unchecked"
        )
    }
}

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Implement `Drop` for `Listener`.
//
// This is the **drop guarantee** in action: when the `Pin<Box<…>>`
// is dropped, `Drop::drop` runs BEFORE the memory is freed.
// This gives us a chance to call `c_deregister(id)` so the C
// registry no longer holds a dangling pointer.
//
// Steps:
//   1. If `self.registry_id` is `Some(id)`, call `c_deregister(id)`.
//   2. That's it — the rest is automatic.

// impl Drop for Listener { ... }

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Implement a safe `PinnedListener` wrapper that:
//   - Cannot be moved (wraps `Pin<Box<Listener>>`).
//   - Provides read access to name and weight.
//   - Automatically deregisters on drop (via Listener's Drop impl).
//
// This is the "safe API" you'd expose to the rest of the codebase.

pub struct PinnedListener {
    // TODO: store Pin<Box<Listener>>
}

impl PinnedListener {
    /// Create a new pinned listener, registered with the C side.
    pub fn new(name: &str, weight: i32) -> Self {
        todo!("Delegate to Listener::new_pinned, wrap in PinnedListener")
    }

    /// Get the listener's name.
    pub fn name(&self) -> &str {
        todo!("Access self.inner.name through Pin")
    }

    /// Get the listener's weight.
    pub fn weight(&self) -> i32 {
        todo!("Access self.inner.weight through Pin")
    }
}

// ══════════════════════════════════════════════════════════════
// Part B — Why Pin matters: the broken version
// ══════════════════════════════════════════════════════════════

// ── TODO 5 ─────────────────────────────────────────────────────
//
// This function shows what goes wrong WITHOUT Pin.
//
// `UnpinnedListener` is a version that IS Unpin (no PhantomPinned).
// After registering, we MOVE it — now C's pointer is dangling.
//
// Your task: read the broken code, then write `safe_dispatch` that
// uses `PinnedListener` correctly to avoid the problem.

/// A version WITHOUT `PhantomPinned` — CAN be moved.  Dangerous!
pub struct UnpinnedListener {
    pub name: String,
    pub weight: i32,
    pub registry_id: Option<u64>,
}

impl UnpinnedListener {
    pub fn new_registered(name: &str, weight: i32) -> Box<UnpinnedListener> {
        let mut listener = Box::new(UnpinnedListener {
            name: name.to_string(),
            weight,
            registry_id: None,
        });
        let ptr = &*listener as *const UnpinnedListener;
        // This cast is wrong in general because the structs differ,
        // but for demonstration we DON'T actually register it.
        // The point is to show the concept.
        listener.registry_id = Some(0); // placeholder
        listener
    }
}

/// Use `PinnedListener` to safely register two listeners, dispatch
/// an event, and return the results.  The listeners are automatically
/// deregistered when this function returns.
///
/// This demonstrates correct usage: objects stay pinned for their
/// entire registered lifetime, and Drop cleans up.
pub fn safe_dispatch(event: i32) -> Vec<String> {
    todo!(
        "1. Create two PinnedListeners (e.g. 'alpha' weight=2, 'beta' weight=3)\n\
         2. Call c_dispatch(event) (unsafe — but safe because our\n\
            listeners are pinned and alive)\n\
         3. Return the results\n\
         4. PinnedListeners drop here → auto-deregister"
    )
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Reset the registry between tests.
    fn setup() {
        c_registry_clear();
    }

    #[test]
    fn test_ex16_register_and_dispatch() {
        setup();
        let listener = Listener::new_pinned("sensor", 10);
        assert_eq!(c_registry_len(), 1);

        let results = unsafe { c_dispatch(3) };
        assert_eq!(results, vec!["sensor:30"]);

        drop(listener); // should deregister
        assert_eq!(c_registry_len(), 0);
    }

    #[test]
    fn test_ex16_auto_deregister_on_drop() {
        setup();
        {
            let _a = Listener::new_pinned("a", 1);
            let _b = Listener::new_pinned("b", 2);
            assert_eq!(c_registry_len(), 2);
        }
        // Both dropped → registry empty.
        assert_eq!(c_registry_len(), 0);
    }

    #[test]
    fn test_ex16_multiple_dispatch() {
        setup();
        let _x = Listener::new_pinned("x", 5);
        let _y = Listener::new_pinned("y", -1);

        let r1 = unsafe { c_dispatch(2) };
        assert!(r1.contains(&"x:10".to_string()));
        assert!(r1.contains(&"y:-2".to_string()));

        let r2 = unsafe { c_dispatch(0) };
        assert!(r2.contains(&"x:0".to_string()));
        assert!(r2.contains(&"y:0".to_string()));
    }

    #[test]
    fn test_ex16_partial_drop() {
        setup();
        let a = Listener::new_pinned("first", 1);
        let _b = Listener::new_pinned("second", 2);
        assert_eq!(c_registry_len(), 2);

        drop(a); // drop only the first
        assert_eq!(c_registry_len(), 1);

        let results = unsafe { c_dispatch(5) };
        assert_eq!(results, vec!["second:10"]);
    }

    #[test]
    fn test_ex16_pinned_listener_wrapper() {
        setup();
        let p = PinnedListener::new("wrapped", 7);
        assert_eq!(p.name(), "wrapped");
        assert_eq!(p.weight(), 7);
        assert_eq!(c_registry_len(), 1);

        let results = unsafe { c_dispatch(3) };
        assert_eq!(results, vec!["wrapped:21"]);

        drop(p);
        assert_eq!(c_registry_len(), 0);
    }

    #[test]
    fn test_ex16_safe_dispatch() {
        setup();
        let results = safe_dispatch(4);
        // After safe_dispatch returns, everything is deregistered.
        assert_eq!(c_registry_len(), 0);
        // Should have two results.
        assert_eq!(results.len(), 2);
        assert!(results.contains(&"alpha:8".to_string()));  // 4 * 2
        assert!(results.contains(&"beta:12".to_string()));  // 4 * 3
    }

    #[test]
    fn test_ex16_cannot_move_out_of_pin() {
        // This is a compile-time guarantee.  If you uncomment the
        // line below, it should NOT compile because Listener: !Unpin.
        //
        // let pinned = Listener::new_pinned("test", 1);
        // let moved = *pinned;  // ERROR: cannot move out of Pin<Box<Listener>>
        //
        // ^ Try uncommenting this to verify Pin prevents moving.
    }
}
