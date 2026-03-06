# Exercise 16: Pin and the Drop Guarantee

## The problem in one sentence

When C code holds a raw pointer to a Rust object, Rust must guarantee two
things: **the object doesn't move** (so the pointer stays valid) and **cleanup
runs before the memory is freed** (so C never dereferences freed memory).

---

## What is the "drop guarantee"?

The **drop guarantee** is the promise that once a value is pinned, its
`Drop::drop()` will run before its memory is reclaimed. This gives the
value a "last chance" to deregister from external systems (C registries,
linked lists, callback tables) that hold raw pointers to it.

The standard library documents it like this:

> For pinned data where `T: !Unpin`, `Drop::drop` is guaranteed to be
> called before the memory is deallocated.

Three pieces work together to make this safe:

```text
Pin<Box<T>>       →  T cannot move after pinning
T: !Unpin         →  Pin actually enforces immovability (not just advisory)
impl Drop for T   →  cleanup runs before Box deallocates
```

Without any one of these, the guarantee breaks.

---

## Why the standard docs feel confusing

The std docs describe Pin in terms of *structural pinning*, *projection*,
and *Unpin as an auto-trait*. Those concepts matter for `async`/`Future`
where you need to pin self-referential structs on the stack. For FFI, you
can ignore most of that. Here's what matters:

| Concept | What it means for FFI |
|---------|----------------------|
| `Pin<Box<T>>` | Heap allocation that won't be moved or swapped |
| `T: !Unpin` | Pin is actually enforced (use `PhantomPinned` to opt out of Unpin) |
| `Drop` on `T` | Runs before `Box` frees the memory, so you can deregister from C |
| `Pin::as_ref()` | Get a `&T` from `Pin<Box<T>>` — safe, read-only |
| `Pin::get_unchecked_mut()` | Get `&mut T` — unsafe, you promise not to move T |

That's it. You don't need to think about structural pinning or pin
projection for the FFI use case.

---

## The pattern — step by step

### Step 1: Make the type `!Unpin`

```rust
use std::marker::PhantomPinned;

pub struct Listener {
    name: String,
    weight: i32,
    registry_id: Option<u64>,
    _pin: PhantomPinned,           // ← opts out of Unpin
}
```

`PhantomPinned` is a zero-sized type whose only purpose is to make the
struct `!Unpin`. Without it, `Pin<Box<Listener>>` provides `DerefMut`
and the value can be moved freely via `mem::swap` — defeating the point.

### Step 2: Pin before registering

```rust
impl Listener {
    pub fn new_pinned(name: &str, weight: i32) -> Pin<Box<Listener>> {
        let mut boxed = Box::new(Listener {
            name: name.to_string(),
            weight,
            registry_id: None,
            _pin: PhantomPinned,
        });

        // The Box gives us a stable heap address.
        let ptr: *const Listener = &*boxed;
        let id = c_register(ptr);       // C stores this pointer
        boxed.registry_id = Some(id);

        // SAFETY: We won't move this value again — Box owns
        // the heap allocation and Pin prevents extraction.
        unsafe { Pin::new_unchecked(boxed) }
    }
}
```

**Why modify before pinning?** Once we call `Pin::new_unchecked`, we can
no longer get `&mut Listener` through safe code (because `Listener:
!Unpin`). So we must set `registry_id` *before* creating the Pin. This
is fine — the pointer is already stable (it points into the `Box` heap
allocation), so C can start using it immediately.

### Step 3: Deregister in Drop

```rust
impl Drop for Listener {
    fn drop(&mut self) {
        if let Some(id) = self.registry_id {
            c_deregister(id);   // Remove from C's list
        }
    }
}
```

When `Pin<Box<Listener>>` goes out of scope:

1. `Listener::drop()` runs → calls `c_deregister` → C forgets the pointer
2. `Box` deallocates the memory

In that order. **The pointer is always valid when C might use it.**

### Step 4: Wrap in a safe API

```rust
pub struct PinnedListener {
    inner: Pin<Box<Listener>>,
}

impl PinnedListener {
    pub fn new(name: &str, weight: i32) -> Self {
        Self { inner: Listener::new_pinned(name, weight) }
    }

    pub fn name(&self) -> &str {
        // Pin::as_ref() gives &Listener — safe, no move possible
        &self.inner.as_ref().get_ref().name
    }
}
```

Users of `PinnedListener` never see `unsafe`, can't move the inner data,
and get automatic cleanup.

---

## What goes wrong without Pin

```rust
// No PhantomPinned, so this is Unpin — Pin has no effect
struct BadListener {
    name: String,
    registry_id: Option<u64>,
}

fn broken() {
    let mut a = Box::new(BadListener { name: "A".into(), registry_id: None });
    let ptr = &*a as *const BadListener;
    let id = c_register(ptr);
    a.registry_id = Some(id);

    // 💥 Move the value out of the Box
    let b = *a;                  // a is consumed, its memory freed
    // C still holds `ptr` → dangling pointer!
    c_dispatch(42);              // undefined behavior
}
```

With `PhantomPinned` + `Pin<Box<T>>`, the `let b = *a` line would be a
**compile error** — the value cannot be extracted from the Pin.

---

## Common usage patterns

### Pattern 1: Self-registering callback object

The most common FFI pattern. A Rust object tells a C event loop "here's
my address, call me back when something happens."

```rust
// C side (simplified):
//   void register_handler(Handler* h);
//   void unregister_handler(Handler* h);
//   void poll_events();  // calls h->on_event() for each registered h

struct Handler {
    callback: Box<dyn Fn(i32)>,
    _pin: PhantomPinned,
}

impl Drop for Handler {
    fn drop(&mut self) {
        // MUST deregister before memory is freed
        unsafe { unregister_handler(self as *mut Handler); }
    }
}
```

### Pattern 2: Intrusive linked list node

C libraries (Linux kernel, libuv) use intrusive lists where the node is
embedded in the struct. Moving the struct breaks the list links.

```rust
#[repr(C)]
struct ListNode {
    prev: *mut ListNode,
    next: *mut ListNode,
}

struct MyStruct {
    node: ListNode,       // embedded node — address must be stable
    data: Vec<u8>,
    _pin: PhantomPinned,
}

impl Drop for MyStruct {
    fn drop(&mut self) {
        // Unlink from the list before memory is freed
        unsafe { list_remove(&mut self.node); }
    }
}
```

### Pattern 3: Pinned async task registered with C I/O

Async Rust futures are self-referential and must be pinned. When they
also register with C (e.g., `io_uring`, `epoll`), you get both reasons
for pinning at once — self-references AND external pointers.

```rust
struct IoTask {
    buffer: [u8; 4096],
    // C's io_uring holds a pointer to `buffer`
    ring_entry_id: u32,
    _pin: PhantomPinned,
}

impl Drop for IoTask {
    fn drop(&mut self) {
        // Cancel the I/O submission so the kernel stops writing
        // to our buffer before we free it
        unsafe { io_uring_cancel(self.ring_entry_id); }
    }
}
```

---

## Quick reference: when do you need Pin?

| Situation | Need Pin? | Why |
|-----------|-----------|-----|
| C stores a pointer to your Rust struct | **Yes** | Prevents move, Drop deregisters |
| You `Box::into_raw` and C owns the pointer | No | C calls back to Rust to free; the Box doesn't move |
| Opaque handle pattern (ex05, ex17) | No | C holds a pointer but Rust controls the whole lifecycle through `from_raw` |
| Self-referential struct (async Future) | **Yes** | Internal pointers are invalidated by moves |
| Passing a struct by value to C | No | C gets a copy; the original can move freely |

The rule of thumb: **if the address of a Rust value is stored somewhere
you don't control (C library, kernel, hardware register), pin it and
deregister in Drop.**
