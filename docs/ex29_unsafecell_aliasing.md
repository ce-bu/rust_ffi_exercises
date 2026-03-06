# Exercise 29: `UnsafeCell`, Aliasing, and LLVM's `noalias`

## Why `UnsafeCell` matters for FFI

`UnsafeCell<T>` is the **only** primitive in Rust that allows mutation
through a shared reference (`&T`). Every interior-mutability type —
`Cell`, `RefCell`, `Mutex`, `AtomicU32` — is built on top of it.

In FFI, `UnsafeCell` matters because **C code doesn't know about Rust's
aliasing rules**. When you hand C a pointer to data that Rust also holds
as `&T`, and C mutates that data, you have two choices:

1. **Wrap the mutable fields in `UnsafeCell`** → correct.
2. **Do nothing** → undefined behavior, even if "it works today."

The reason is LLVM.

---

## How Rust talks to LLVM about aliasing

### The `noalias` attribute

When Rust compiles a function like:

```rust
fn read_twice(x: &i32) -> i32 {
    let a = *x;
    let b = *x;
    a + b
}
```

It tells LLVM that `x` is a `noalias readonly` pointer. LLVM then
optimizes this to:

```llvm
; LLVM sees: x is readonly, no one else can write through x
%val = load i32, ptr %x
%result = add i32 %val, %val    ; only ONE load — the second was eliminated
ret i32 %result
```

This is legal because Rust guarantees `&T` means "no one is writing to
this memory through any alias."

### What happens when C violates this

Consider this FFI scenario:

```rust
#[repr(C)]
struct Sensor {
    value: i32,    // ← NO UnsafeCell
}

extern "C" {
    fn c_update_sensor(s: *mut Sensor);
}

fn read_sensor(s: &Sensor) -> (i32, i32) {
    let before = s.value;
    unsafe { c_update_sensor(s as *const _ as *mut _); }
    let after = s.value;
    (before, after)  // You expect different values...
}
```

LLVM sees that `s` is `&Sensor` (readonly, noalias). It may:

1. **Cache the load**: read `s.value` once, reuse it for both `before`
   and `after`. Result: `(42, 42)` even though C wrote `99`.
2. **Reorder the loads**: move the second load before the C call.
3. **Eliminate the second load entirely** in release mode.

This isn't a theoretical risk — it happens in practice with `opt-level=2`
and above. The scary part: it works in debug builds (no optimizations)
and breaks silently in release.

### The fix: `UnsafeCell`

```rust
use std::cell::UnsafeCell;

#[repr(C)]
struct Sensor {
    value: UnsafeCell<i32>,   // ← tells LLVM: this memory may change
}
```

When a struct contains `UnsafeCell`, Rust does **not** emit `noalias
readonly` for references to it. LLVM must assume the memory can change at
any time, so it:

- Cannot cache loads across function calls
- Cannot reorder loads past potential writes
- Cannot eliminate "redundant" loads

This is exactly what you need when C is mutating the data behind your back.

---

## LLVM IR: with and without `UnsafeCell`

### Without `UnsafeCell` (broken)

```rust
fn sum_two_reads(x: &i32) -> i32 {
    let a = *x;
    black_box_c_call();   // might write to *x through a raw pointer
    let b = *x;
    a + b
}
```

LLVM IR (simplified):

```llvm
; x: ptr noalias readonly
define i32 @sum_two_reads(ptr noalias readonly %x) {
    %a = load i32, ptr %x
    call void @black_box_c_call()
    ; LLVM knows %x is readonly → reuses %a instead of reloading
    %sum = add i32 %a, %a
    ret i32 %sum
}
```

### With `UnsafeCell` (correct)

```rust
fn sum_two_reads(x: &UnsafeCell<i32>) -> i32 {
    let a = unsafe { *x.get() };
    black_box_c_call();
    let b = unsafe { *x.get() };
    a + b
}
```

LLVM IR (simplified):

```llvm
; x: ptr  (NO noalias, NO readonly)
define i32 @sum_two_reads(ptr %x) {
    %a = load i32, ptr %x
    call void @black_box_c_call()
    %b = load i32, ptr %x          ; must reload — memory may have changed
    %sum = add i32 %a, %b
    ret i32 %sum
}
```

The only difference is the absence of `noalias readonly`. That's what
`UnsafeCell` does — it's a signal to the Rust compiler (and through it,
LLVM) that the aliasing rules are relaxed for this memory.

---

## When do you need `UnsafeCell` in FFI?

| Scenario | Need `UnsafeCell`? | Why |
|----------|-------------------|-----|
| C mutates a field while Rust holds `&T` | **Yes** | Rust assumes `&T` fields are immutable |
| Rust passes `*mut T` to C, no `&T` exists | No | Raw pointers have no aliasing assumptions |
| Opaque handle (Rust never dereferences) | No | No Rust reference = no aliasing issue |
| Struct passed by value to C (copy) | No | C has its own copy |
| C reads but never writes | No | `readonly` is correct |
| Hardware MMIO register mapped into memory | **Yes** | Reads have side effects, values change spontaneously |
| Atomic field modified by C threads | **Yes** | Use `AtomicU32` (which contains `UnsafeCell`) |
| C callback modifies a field during iteration | **Yes** | Rust holds `&self` while C writes through a pointer |

### The critical rule

> If Rust holds `&T` (directly or through `&Struct`) and anyone —
> C code, another thread, hardware — can write to that memory, the
> mutable field **must** be inside `UnsafeCell`.

---

## Patterns

### Pattern 1: C-mutated status field

```rust
use std::cell::UnsafeCell;

/// A struct shared with C. C updates `status` asynchronously.
#[repr(C)]
pub struct Device {
    pub id: u32,                          // immutable — no UnsafeCell needed
    pub status: UnsafeCell<u32>,          // C writes this
    pub error_code: UnsafeCell<i32>,      // C writes this
}

impl Device {
    pub fn status(&self) -> u32 {
        // SAFETY: We read atomically-sized u32; C may write but
        // it's UnsafeCell so LLVM won't cache the load.
        unsafe { *self.status.get() }
    }
}
```

### Pattern 2: C callback mutates during iteration

```rust
#[repr(C)]
pub struct Accumulator {
    pub total: UnsafeCell<f64>,
}

extern "C" {
    // C calls our callback with each value; callback adds to total.
    fn c_iterate(acc: *mut Accumulator, cb: extern "C" fn(*mut Accumulator, f64));
}

extern "C" fn on_value(acc: *mut Accumulator, val: f64) {
    unsafe {
        let total = (*acc).total.get();
        *total += val;
    }
}
```

### Pattern 3: Wrapping C global state with interior mutability

```rust
use std::cell::UnsafeCell;

/// Wrapper around C library state that C can modify at any time.
pub struct CLibState {
    raw: UnsafeCell<*mut std::ffi::c_void>,
}

// SAFETY: The C library is documented as thread-safe.
unsafe impl Send for CLibState {}
unsafe impl Sync for CLibState {}

impl CLibState {
    pub fn get_value(&self) -> i32 {
        // The UnsafeCell means LLVM won't assume the pointed-to
        // state is unchanged between calls.
        unsafe { c_lib_read_value(*self.raw.get()) }
    }

    pub fn update(&self) {
        // Even though we only have &self, C can mutate.
        unsafe { c_lib_update(*self.raw.get()); }
    }
}
```

### Pattern 4: Atomic fields for cross-language concurrency

```rust
use std::sync::atomic::{AtomicU32, Ordering};

/// Shared between Rust and C. C increments `counter` with
/// __atomic_fetch_add; Rust reads it with AtomicU32.
#[repr(C)]
pub struct SharedCounters {
    pub counter: AtomicU32,       // AtomicU32 contains UnsafeCell
    pub flags: AtomicU32,
}
```

`AtomicU32` is `#[repr(transparent)]` over `UnsafeCell<u32>`, so it's
layout-compatible with a C `_Atomic uint32_t` / `atomic_uint`.

---

## `UnsafeCell` vs `*mut T` vs `&mut T`

| Approach | When to use | Aliasing |
|----------|-------------|----------|
| `*mut T` | You own the pointer, no Rust reference exists simultaneously | No LLVM assumptions |
| `&mut T` | Exclusive access guaranteed, no C aliases | `noalias` — full optimization |
| `&UnsafeCell<T>` | Shared access, C may write | No `noalias` — LLVM is conservative |
| `&T` (no UnsafeCell) | Shared access, **nobody writes** | `noalias readonly` — LLVM caches aggressively |

The key insight: `*mut T` and `&UnsafeCell<T>` are similar in terms of
what LLVM is allowed to optimize, but `&UnsafeCell<T>` carries lifetime
information and participates in Rust's borrow checker. Use it when you
want the compiler to help with lifetimes while still allowing mutation.

---

## Inspecting LLVM IR yourself

To see the actual LLVM attributes Rust emits:

```sh
# Emit LLVM IR for a single function
cargo rustc --release -- --emit=llvm-ir
# Look in target/release/deps/*.ll

# Or use cargo-show-asm:
cargo install cargo-show-asm
cargo asm --lib 'my_module::my_function'
```

Search for `noalias` and `readonly` on function parameters. You'll see
them on `&T` parameters but **not** on `&UnsafeCell<T>` parameters.

---

## Stacked Borrows / Tree Borrows and Miri

Rust's formal aliasing model (Stacked Borrows, and its successor Tree
Borrows) goes beyond what LLVM does today. Miri enforces these rules:

```sh
cargo +nightly miri test ex29
```

Miri will catch violations like:
- Writing through a raw pointer that was derived from `&T` (without `UnsafeCell`)
- Creating `&T` to `UnsafeCell`-free memory while a `*mut T` write is outstanding
- Retag violations where a shared reference invalidates a mutable pointer

Even if LLVM doesn't miscompile your code *today*, Miri flags the UB
that *could* break in a future compiler version.

---

## Summary

```text
Does C (or hardware, or another thread) mutate this memory
while Rust holds &T?
    │
    ├── No  → Plain field. LLVM can optimize freely. ✓
    │
    └── Yes → Is it atomic access?
              │
              ├── Yes → Use AtomicU32/AtomicPtr/etc. (built on UnsafeCell)
              │
              └── No  → Wrap in UnsafeCell<T>. Provide safe API with
                        unsafe interior that documents the invariants.
```

`UnsafeCell` is the **foundation** of all interior mutability in Rust.
In FFI, it's your tool for telling the compiler "this memory can change
even though I only have a shared reference — don't optimize based on
immutability assumptions."
