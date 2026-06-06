// tests/ex27_test.rs — Integration tests for C++ STL wrapping
#![cfg(not(test))]
use rust_ffi_exercises::ex27_cpp_stl::*;

/* ══════════════════════════════════════════════════════════════
 * Lifecycle: new / drop / clone
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn new_stack_is_empty() {
    let s = StringStack::new();
    assert_eq!(s.len(), 0);
    assert!(s.is_empty());
}

#[test]
fn drop_runs_without_panic() {
    let mut s = StringStack::new();
    s.push("hello");
    s.push("world");
    drop(s); // explicit — should not leak
}

#[test]
fn clone_produces_independent_copy() {
    let mut original = StringStack::new();
    original.push("alpha");
    original.push("bravo");

    let mut cloned = original.clone();

    // Modify clone — original must be unaffected.
    cloned.push("charlie");
    assert_eq!(original.len(), 2);
    assert_eq!(cloned.len(), 3);
}

/* ══════════════════════════════════════════════════════════════
 * Push / Pop / Peek
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn push_and_pop() {
    let mut s = StringStack::new();
    s.push("first");
    s.push("second");
    assert_eq!(s.len(), 2);

    let top = s.pop().unwrap();
    assert_eq!(top, "second");
    assert_eq!(s.len(), 1);

    let next = s.pop().unwrap();
    assert_eq!(next, "first");
    assert!(s.is_empty());
}

#[test]
fn pop_empty_returns_error() {
    let mut s = StringStack::new();
    match s.pop() {
        Err(StackError::Empty) => {} // expected
        other => panic!("expected Empty, got {:?}", other),
    }
}

#[test]
fn peek_without_copy() {
    let mut s = StringStack::new();
    s.push("hello");

    let guard = s.peek().unwrap();
    assert_eq!(guard.as_str(), "hello");
    assert_eq!(guard.len(), 5);

    // Display trait
    assert_eq!(format!("{guard}"), "hello");
}

#[test]
fn peek_empty_returns_error() {
    let s = StringStack::new();
    match s.peek() {
        Err(StackError::Empty) => {}
        other => panic!("expected Empty, got {:?}", other),
    }
}

#[test]
fn peek_borrows_immutably() {
    // This test verifies that peek() prevents mutation at compile time.
    // The guard holds &StringStack, so push/pop (which need &mut) are
    // blocked while the guard exists.
    let mut s = StringStack::new();
    s.push("hello");

    let guard = s.peek().unwrap();
    let _val = guard.as_str(); // use the guard
    drop(guard); // explicitly drop before mutation

    s.push("world"); // now mutation is allowed
    assert_eq!(s.len(), 2);
}

/* ══════════════════════════════════════════════════════════════
 * Join
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn join_elements() {
    let mut s = StringStack::new();
    s.push("a");
    s.push("b");
    s.push("c");
    assert_eq!(s.join(", "), "a, b, c");
}

#[test]
fn join_single_element() {
    let mut s = StringStack::new();
    s.push("only");
    assert_eq!(s.join(", "), "only");
}

#[test]
fn join_empty_stack() {
    let s = StringStack::new();
    assert_eq!(s.join(", "), "");
}

/* ══════════════════════════════════════════════════════════════
 * Batch insertion — push_many
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn push_many_items() {
    let mut s = StringStack::new();
    s.push_many(&["x", "y", "z"]);
    assert_eq!(s.len(), 3);
    assert_eq!(s.join("-"), "x-y-z");
}

#[test]
fn push_many_empty() {
    let mut s = StringStack::new();
    s.push_many(&[]);
    assert!(s.is_empty());
}

/* ══════════════════════════════════════════════════════════════
 * Callback iteration — to_vec
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn to_vec_collects_all() {
    let mut s = StringStack::new();
    s.push("alpha");
    s.push("bravo");
    s.push("charlie");
    let v = s.to_vec();
    assert_eq!(v, vec!["alpha", "bravo", "charlie"]);
}

#[test]
fn to_vec_empty() {
    let s = StringStack::new();
    assert!(s.to_vec().is_empty());
}

/* ══════════════════════════════════════════════════════════════
 * Factory — from_csv
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn from_csv_parses_items() {
    let s = StringStack::from_csv("one, two, three");
    assert_eq!(s.len(), 3);
    let v = s.to_vec();
    assert_eq!(v, vec!["one", "two", "three"]);
}

#[test]
fn from_csv_single() {
    let s = StringStack::from_csv("solo");
    assert_eq!(s.len(), 1);
    assert_eq!(s.to_vec(), vec!["solo"]);
}

#[test]
fn from_csv_empty_string() {
    let s = StringStack::from_csv("");
    assert_eq!(s.len(), 0);
}

/* ══════════════════════════════════════════════════════════════
 * Unicode strings
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn unicode_roundtrip() {
    let mut s = StringStack::new();
    s.push("日本語");
    s.push("Ñoño");
    s.push("🦀🔥");

    let top = s.pop().unwrap();
    assert_eq!(top, "🦀🔥");

    let guard = s.peek().unwrap();
    assert_eq!(guard.as_str(), "Ñoño");
    drop(guard);

    assert_eq!(s.to_vec(), vec!["日本語", "Ñoño"]);
}

/* ══════════════════════════════════════════════════════════════
 * Default trait
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn default_creates_empty_stack() {
    let s = StringStack::default();
    assert!(s.is_empty());
}

/* ══════════════════════════════════════════════════════════════
 * Error Display
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn stack_error_display() {
    let e = StackError::Empty;
    assert_eq!(format!("{e}"), "stack is empty");

    let e = StackError::BufferTooSmall { needed: 42 };
    assert!(format!("{e}").contains("42"));
}
