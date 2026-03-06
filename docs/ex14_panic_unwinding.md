# Exercise 14: Panic Unwinding and `Drop`

## What is panic unwinding?

When a Rust program panics, the runtime **unwinds the stack** — it walks back
through each stack frame and calls the `Drop` implementation for every local
variable that is still alive, much like C++ exception unwinding calls
destructors.

```rust
fn inner() {
    let v = vec![1, 2, 3];              // heap-allocated
    let f = File::open("log.txt").unwrap();
    panic!("oops");
    // During unwinding:
    //   f.drop() runs  → closes the file handle
    //   v.drop() runs  → frees the heap allocation
}
```

Even though execution never reaches the end of the function, both `f` and `v`
are properly cleaned up.

## Why unwinding matters for `Drop`

### 1. Resource safety

Unwinding guarantees that `Drop` runs for all live values on the stack, even on
a panic. File handles get closed, mutexes get unlocked, and heap allocations get
freed — automatically.

### 2. `catch_unwind` depends on it

`std::panic::catch_unwind` works by stopping the unwind at a chosen boundary and
returning the panic payload as an `Err`. Every stack frame *between* the panic
site and the catch point is unwound normally, so all destructors run:

```rust
let result = std::panic::catch_unwind(|| {
    let guard = mutex.lock().unwrap();
    panic!("something went wrong");
    // guard.drop() still runs during unwind → the mutex is released
});
// result is Err(...), but the mutex is NOT poisoned-and-locked-forever
```

### 3. Abort vs. unwind

Rust also supports `panic = "abort"` (set in `Cargo.toml`), which kills the
process immediately without unwinding. This produces smaller binaries and is
slightly faster, but **no `Drop` code runs**. The OS reclaims memory and file
descriptors, but application-level cleanup — flushing buffers, writing
save files, releasing advisory locks — is skipped entirely.

| Mode      | `Drop` runs? | `catch_unwind` works? | Binary size |
|-----------|--------------|-----------------------|-------------|
| `unwind`  | Yes          | Yes                   | Larger      |
| `abort`   | No           | No (process dies)     | Smaller     |

### 4. FFI boundary hazard (the core of Exercise 14)

Unwinding across an `extern "C"` boundary is **undefined behavior**. C has no
concept of Rust's unwinding mechanism, so the unwinder cannot interpret C stack
frames, and C local variables have no destructors to call. If a panic escapes
through a C frame, anything can happen — corrupted stack, silent data loss, or
a segfault.

The fix is to **catch the panic before it reaches the boundary**:

```rust
#[no_mangle]
pub extern "C" fn my_func(x: i32) -> i32 {
    match std::panic::catch_unwind(|| {
        // code that might panic
        risky_computation(x)
    }) {
        Ok(val) => val,
        Err(_)  => -1,  // return a sentinel error code
    }
}
```

## `UnwindSafe` and logical invariants

Types involved in `catch_unwind` must satisfy the `UnwindSafe` trait (or you
must explicitly opt in with `AssertUnwindSafe`). This exists because a panic can
interrupt a function *in the middle of modifying data*. `Drop` still runs, so
there are no resource leaks, but the **logical invariants** of your data
structures may be violated.

For example, a `Vec` that was halfway through a `push` (length updated but
element not yet written) would be in an inconsistent state. `UnwindSafe` is a
lint-level guard reminding you to consider whether it is safe to keep using a
value after catching a panic that may have partially modified it.

## Key takeaway

> Unwinding is Rust's mechanism for making panics **safe by default**: even when
> something goes wrong, `Drop` still runs, so your program doesn't leak
> resources or leave things in a broken state — as long as you don't let the
> unwind cross an FFI boundary.
