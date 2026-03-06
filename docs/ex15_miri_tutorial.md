# Exercise 15: A Programmer's Guide to Miri

## What is Miri?

Miri is an **interpreter** for Rust's Mid-level Intermediate Representation
(MIR). It runs your code step-by-step and checks every memory operation against
Rust's safety rules. Unlike the compiler (which only checks at compile time) or
sanitizers (which catch bugs probabilistically at runtime), Miri **deterministically**
catches undefined behavior that may compile, run, and appear to work perfectly
— until it silently doesn't.

```sh
# Install (one-time, requires nightly)
rustup component add miri

# Run tests under Miri
cargo +nightly miri test ex15

# Run a specific test
cargo +nightly miri test ex15_bug1 -- --ignored
```

---

## How to read a Miri error

Miri errors follow a consistent structure. Here's a real example:

```
error: Undefined Behavior: memory access through pointer to alloc1352
       at offset 0, but that allocation has been freed
  --> src/ex15_miri_ub.rs:72:14
   |
72 |     unsafe { *ptr.add(0) + *ptr.add(1) }
   |              ^^^^^^^^^^^ memory access through pointer to alloc1352
   |              at offset 0, but that allocation has been freed
   |
   = help: this indicates a bug in the program: it performed an
     invalid operation, and caused Undefined Behavior
   = help: see https://doc.rust-lang.org/nightly/reference/behavior-
     considered-undefined.html for further information
   |
   = note: BACKTRACE:
   = note: inside `ex15_miri_ub::bug1_dangling_vec_pointer` at src/ex15_miri_ub.rs:72:14
```

### Anatomy of the error

| Part | What it tells you |
|------|-------------------|
| **"Undefined Behavior:"** | The category — Miri always starts with this |
| **"memory access through pointer to alloc1352"** | Which allocation is involved (Miri numbers them) |
| **"but that allocation has been freed"** | The specific violation |
| **`--> src/ex15_miri_ub.rs:72:14`** | The exact line and column |
| **`^^^^^^^^^^^ memory access...`** | The offending expression underlined |
| **BACKTRACE** | Call stack leading to the violation |

### Miri sometimes also shows the allocation history

```
   = note: alloc1352 was allocated here:
  --> src/ex15_miri_ub.rs:63:21
   |
63 |     let mut v = vec![1, 2, 3];
   |                     ^^^^^^^^^
   |
   = note: alloc1352 was freed here:
  --> src/ex15_miri_ub.rs:67:5
   |
67 |     v.push(4);
   |     ^^^^^^^^^
```

This shows you exactly when the memory was allocated and when it was freed.
The bug is using the pointer *after* that free.

---

## The 8 categories of UB that Miri catches

### 1. Dangling pointer / use-after-reallocation

**Miri says:** `"pointer to alloc... was dereferenced after this allocation got freed"`

**What happened:** You took a pointer into a container (Vec, String, etc.),
then mutated the container (causing reallocation), then used the old pointer.

```rust
// BUGGY
let mut v = vec![1, 2, 3];
let ptr = v.as_ptr();       // pointer to current buffer
v.push(4);                   // may reallocate → ptr is dangling
unsafe { *ptr }              // 💥 UB: reading freed memory
```

**Fix:** Take the pointer *after* all mutations are done:

```rust
// FIXED
let mut v = vec![1, 2, 3];
v.push(4);                   // mutate first
let ptr = v.as_ptr();        // NOW take the pointer
unsafe { *ptr }              // ✓ pointer is valid
```

**Rule:** A pointer into a Vec (or any growable container) is invalidated
by any operation that might reallocate. Re-derive the pointer after mutations.

---

### 2. Mutable aliasing violation

**Miri says:** `"trying to retag... but found SharedReadOnly"` or
`"not granting access... because that would remove..."`

**What happened:** Two `&mut` references to the same memory exist
simultaneously.

```rust
// BUGGY
let mut value: i32 = 10;
let ptr = &mut value as *mut i32;
let ref_a = unsafe { &mut *ptr };   // first &mut
let ref_b = unsafe { &mut *ptr };   // second &mut — invalidates ref_a
*ref_a += 1;   // 💥 UB: ref_a's permission was revoked
*ref_b += 2;
```

**Fix:** Use one mutable reference at a time, or use raw pointer
operations:

```rust
// FIXED (option A: sequential borrows)
let mut value: i32 = 10;
let ptr = &mut value as *mut i32;

// First operation
unsafe { *ptr += 1 };
let snapshot_a = unsafe { *ptr };   // 11

// Second operation
unsafe { *ptr += 2 };
let snapshot_b = unsafe { *ptr };   // 13

(snapshot_a, snapshot_b)
```

```rust
// FIXED (option B: raw pointer only, no references)
let mut value: i32 = 10;
let ptr = &mut value as *mut i32;
unsafe {
    ptr.write(ptr.read() + 1);  // 11
}
let a = unsafe { ptr.read() };
unsafe {
    ptr.write(ptr.read() + 2);  // 13
}
let b = unsafe { ptr.read() };
(a, b)
```

**Rule:** At any point in time, you may have *either* one `&mut T` *or*
any number of `&T` — never both, and never multiple `&mut T`.

---

### 3. Reading uninitialized memory

**Miri says:** `"using uninitialized data"` or `"type validation failed: encountered uninitialized bytes"`

**What happened:** You allocated memory (via `alloc`, `MaybeUninit`, etc.)
and read it without writing first.

```rust
// BUGGY
let layout = Layout::new::<i32>();
let ptr = unsafe { alloc(layout) as *mut i32 };
let val = unsafe { *ptr };   // 💥 UB: reading garbage
```

**Fix:** Always write before reading:

```rust
// FIXED
let layout = Layout::new::<i32>();
let ptr = unsafe { alloc(layout) as *mut i32 };
unsafe { ptr::write(ptr, 0) };        // initialize!
let val = unsafe { *ptr };             // ✓ now defined
unsafe { dealloc(ptr as *mut u8, layout) };
```

**Rule:** Memory from `alloc`, `MaybeUninit`, or C's `malloc` is
uninitialized until you write to it. Reading first is always UB, even
if "it returned zero."

---

### 4. Out-of-bounds access

**Miri says:** `"out-of-bounds pointer arithmetic"` or
`"dereferencing pointer... which is out of bounds"`

**What happened:** Pointer arithmetic went past the allocation. Note that
even *computing* an out-of-bounds pointer (without dereferencing) can be UB
with `ptr.offset()`.

```rust
// BUGGY
let data = [10, 20, 30];    // 3 elements
let ptr = data.as_ptr();
unsafe { *ptr.add(3) }      // 💥 index 3 is out of bounds
```

**Fix:** Use correct indices:

```rust
// FIXED
let data = [10, 20, 30];
let ptr = data.as_ptr();
unsafe { *ptr.add(0) + *ptr.add(1) + *ptr.add(2) }  // ✓
```

**Note:** `ptr.add(len)` (one-past-the-end) is legal to *compute* but
not to *dereference*. This is the same as C's rules.

---

### 5. Use-after-free

**Miri says:** `"pointer to alloc... was used after this allocation got freed"`

**What happened:** You saved a pointer to heap memory, freed the memory
(via `drop`, `Box::from_raw`, `dealloc`), then used the pointer.

```rust
// BUGGY
let b = Box::new(42i32);
let ptr = &*b as *const i32;
drop(b);                    // frees the heap allocation
unsafe { *ptr }             // 💥 UB: dangling pointer
```

**Fix:** Read before freeing, or don't free manually:

```rust
// FIXED
let b = Box::new(42i32);
let val = *b;               // read while alive
drop(b);                    // free — but we already have the value
val                          // ✓
```

---

### 6. Shared-to-mutable cast (Stacked Borrows)

**Miri says:** `"attempting a write through... but it is SharedReadOnly"`

**What happened:** You cast a `&T` to `*mut T` and wrote through it.
Stacked Borrows says: a shared reference grants read-only access. You
cannot "upgrade" it to writable.

```rust
// BUGGY
fn increment(value: &i32) -> i32 {
    let ptr = value as *const i32 as *mut i32;
    unsafe { *ptr += 1 };   // 💥 UB: writing through SharedReadOnly
    unsafe { *ptr }
}
```

**Fix:** Either take `&mut i32`, or work on a copy:

```rust
// FIXED (option A: correct signature)
fn increment(value: &mut i32) -> i32 {
    *value += 1;
    *value
}

// FIXED (option B: work on a copy)
fn increment(value: &i32) -> i32 {
    *value + 1   // return a new value, don't mutate the original
}
```

**Rule:** `&T` means read-only. Period. If you need to mutate, use
`&mut T`, `&UnsafeCell<T>`, or `&AtomicT`.

---

### 7. Stacked Borrows: raw pointer invalidated by a new borrow

**Miri says:** `"tag ... was created here ... but then invalidated"` or
`"trying to use ... but that tag does not exist in the borrow stack"`

This is the subtlest category. Even with raw pointers, Stacked Borrows
tracks a permission stack. Creating a new `&mut` reference pushes a new
entry and pops anything derived from older borrows.

```rust
// BUGGY
let mut array = [1i32, 2];
let ptr = array.as_mut_ptr();   // ptr pushed onto borrow stack

let mref = &mut array;          // new &mut pushes, pops ptr's permission
mref[0] = 10;
mref[1] = 20;

unsafe { *ptr = 11 };           // 💥 UB: ptr's tag was invalidated
```

**Fix:** Re-derive the pointer after the mutable borrow ends:

```rust
// FIXED (option A: re-derive pointer)
let mut array = [1i32, 2];

// Phase 1: use &mut
let mref = &mut array;
mref[0] = 10;
mref[1] = 20;
// mref goes out of scope here

// Phase 2: derive a FRESH pointer
let ptr = array.as_mut_ptr();
unsafe { *ptr = 11 };           // ✓ ptr is freshly derived

array  // [11, 20]
```

```rust
// FIXED (option B: all raw pointer, no references)
let mut array = [1i32, 2];
let ptr = array.as_mut_ptr();
unsafe {
    *ptr = 10;
    *ptr.add(1) = 20;
    *ptr = 11;                  // ✓ no intervening borrows
}
array  // [11, 20]
```

**Rule:** After creating a new `&` or `&mut` to a piece of memory,
any raw pointers derived from *earlier* borrows of that memory are
invalidated. Re-derive from the new borrow.

---

### 8. Unaligned pointer access

**Miri says:** `"accessing memory... but alignment N is required"`

**What happened:** You created a `*const T` or `*mut T` pointing to an
address not aligned to `align_of::<T>()`, then dereferenced it.

```rust
// BUGGY
let data: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
let ptr = unsafe { data.as_ptr().add(1) as *const u32 };
unsafe { *ptr }    // 💥 UB: offset 1 is not 4-byte aligned
```

**Fix:** Use `read_unaligned`:

```rust
// FIXED
let data: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
let ptr = unsafe { data.as_ptr().add(1) as *const u32 };
unsafe { ptr.read_unaligned() }    // ✓ handles any alignment
```

**Rule:** `*ptr` (dereference) requires alignment. If you can't
guarantee alignment, use `ptr::read_unaligned` / `ptr::write_unaligned`.

---

## Understanding Stacked Borrows (the mental model)

Stacked Borrows is the aliasing model Miri enforces. Think of each memory
location as having a **stack of permissions**:

```text
Memory location: 0x7f..a0 (an i32)

Permission Stack (top = active):
┌──────────────────────────┐
│ Unique(tag=3)   ← &mut   │  ← top: currently active
│ SharedRO(tag=2) ← &      │
│ Unique(tag=1)   ← &mut   │
│ Unique(tag=0)   ← owner  │
└──────────────────────────┘
```

**Rules:**
1. Creating `&mut` pushes a **Unique** tag.
2. Creating `&` pushes a **SharedRO** tag.
3. Using a tag requires it to be on the stack.
4. A write through tag N **pops everything above N**.
5. A Unique access pops everything above it that isn't its own tag.

This is why creating a new `&mut` invalidates old raw pointers — the
new Unique tag is pushed, and when it's used, the old tags above it
in logical order are popped.

---

## Tree Borrows: the alternative model

### Why a second model?

Stacked Borrows was the first formal aliasing model for Rust (designed by
Ralf Jung, 2018). It works, but developers found cases where *safe,
reasonable-looking* patterns were rejected:

- Interior mutability combined with shared references
- Two-phase borrows in more complex scenarios
- Some raw pointer patterns common in FFI

**Tree Borrows** (Neven Music, 2023) is a **more permissive** redesign that
keeps the same safety guarantees but organizes permissions as a *tree*
instead of a *stack*, avoiding some of the surprising invalidations.

### The mental model: tree instead of stack

In Stacked Borrows, permissions are a **stack** — last in, first out.
Creating a new borrow pushes onto the stack and accesses through it may pop
everything above.

In Tree Borrows, permissions are arranged as a **tree** rooted at the
original allocation. Each borrow is a node that is a child of the borrow
it was derived from:

```text
Stacked Borrows (linear):         Tree Borrows (branching):

   ┌─────────────┐                         root (alloc)
   │ Unique(tag3) │ ← top                   ├── ptr_a (raw mut)
   │ SharedRO(2)  │                          │   └── ref_b (&mut)
   │ Unique(tag1) │                          └── ptr_c (raw mut)
   │ Unique(tag0) │ ← bottom
   └─────────────┘
```

In the stack model, using `tag1` pops `tag3` and `tag2`. In the tree
model, `ptr_a` and `ptr_c` are **siblings** — using one doesn't
automatically kill the other, as long as they don't conflict (e.g., both
writing to the same location simultaneously).

### Permission states in Tree Borrows

Each node in the tree has a **permission state** that transitions based on
accesses:

```text
                    ┌──────────────────────────────────────┐
                    │          Permission States           │
                    ├──────────────────────────────────────┤
                    │                                      │
  Created from      │   Reserved ──────► Active            │
  &mut (not yet     │   (exclusive      (confirmed         │
   used)            │    potential)      exclusive write)   │
                    │       │                  │            │
                    │       ▼                  ▼            │
                    │   Frozen          Disabled            │
                    │   (read-only,     (permanently        │
                    │    still valid)    invalidated)        │
                    │       │                               │
                    │       ▼                               │
                    │   Disabled                            │
                    └──────────────────────────────────────┘
```

| State | Meaning | Reads? | Writes? |
|-------|---------|--------|---------|
| **Reserved** | Created from `&mut`, not yet written through | Yes | Yes (activates on first write) |
| **Active** | Confirmed exclusive writer | Yes | Yes |
| **Frozen** | Read-only (from `&T` or after a foreign read) | Yes | No |
| **Disabled** | Permanently dead | No | No |

The key difference from Stacked Borrows: **Reserved**. When you create
`&mut T`, the borrow starts as Reserved — it *can* write but hasn't yet.
A foreign read doesn't kill it (just freezes it). Only a **foreign write**
disables it. This is much more permissive for code that creates `&mut`
references it might not immediately write through.

### What Tree Borrows allows that Stacked Borrows rejects

#### Example 1: Reading through a parent while a child `&mut` exists

```rust
let mut val = 42;
let ptr = &mut val as *mut i32;
let _ref = unsafe { &mut *ptr };  // child &mut, state = Reserved

// In Stacked Borrows: reading through ptr MAY pop _ref's tag → UB later
// In Tree Borrows: ptr is a parent, read doesn't disable Reserved child
let x = unsafe { *ptr };          // ✓ in Tree Borrows, may be UB in SB
```

#### Example 2: Sibling raw pointers

```rust
let mut data = [1, 2, 3, 4];
let ptr = data.as_mut_ptr();

let p0 = unsafe { ptr.add(0) };
let p2 = unsafe { ptr.add(2) };

// Writing through p0 (elements 0..2) and p2 (elements 2..4)
// In Stacked Borrows: using p0 after p2 was derived is tricky
// In Tree Borrows: p0 and p2 are siblings — non-overlapping writes are fine
unsafe {
    *p2 = 30;
    *p0 = 10;   // ✓ in Tree Borrows (siblings, non-overlapping)
}
```

#### Example 3: Interior mutability patterns

```rust
use std::cell::UnsafeCell;

let cell = UnsafeCell::new(42);
let r1 = &cell;
let r2 = &cell;

// Both shared refs, both can get *mut through UnsafeCell::get()
let p1 = r1.get();
let p2 = r2.get();

unsafe { *p1 = 1; }
unsafe { *p2 = 2; }   // ✓ Tree Borrows handles this more naturally
```

### What both models agree on (always UB)

These are UB under **both** Stacked Borrows and Tree Borrows:

```rust
// 1. Writing through &T (without UnsafeCell) — always UB
let x = 42;
let ptr = &x as *const i32 as *mut i32;
unsafe { *ptr = 99; }          // 💥 UB in both models

// 2. Two &mut to the same location used for writes — always UB
let mut x = 0;
let r1 = &mut x as *mut i32;
let r2 = unsafe { &mut *r1 };
let r1 = unsafe { &mut *r1 };  // re-derive — but now two active &mut
*r1 = 1;
*r2 = 2;                       // 💥 UB in both models

// 3. Use-after-free — always UB
let ptr = Box::into_raw(Box::new(42));
unsafe { Box::from_raw(ptr); } // freed
unsafe { *ptr }                 // 💥 UB in both models

// 4. Reading uninitialized memory — always UB
```

### Running Miri with Tree Borrows

```sh
# Default: Stacked Borrows
cargo +nightly miri test ex15

# Opt in to Tree Borrows
MIRIFLAGS="-Zmiri-tree-borrows" cargo +nightly miri test ex15
```

### Which model should you target?

The Rust team has not yet declared an official aliasing model. The
practical advice:

| Goal | Recommendation |
|------|----------------|
| Maximum compatibility | Write code that passes **Stacked Borrows** (stricter) |
| Understanding borderline cases | Test under both and compare errors |
| New FFI code | Prefer raw pointer patterns that avoid creating references when not needed — these pass both models |
| Existing code that fails SB but passes TB | Document it, add a `cfg(miri)` test with Tree Borrows, and watch for the Rust team's decision |

The safest rule of thumb for FFI: **minimize the creation of `&` and
`&mut` references to memory that C code might access**. Use raw pointers
as your primary handle, create references only at the moment you need them,
and let them go immediately. This passes both models and avoids the
subtleties entirely.

---

## FFI-specific Miri tips

### Miri can't run C code

Miri interprets Rust MIR — it cannot execute actual C functions. If your
tests call `extern "C"` functions compiled from C source, Miri will error:

```
error: unsupported operation: can't call foreign function `my_c_func`
```

**Workaround:** Write pure-Rust simulations of the C API for testing (as
Exercise 15 does), or use `cfg(miri)` to substitute:

```rust
#[cfg(not(miri))]
extern "C" { fn c_process(data: *mut i32, len: usize); }

#[cfg(miri)]
unsafe fn c_process(data: *mut i32, len: usize) {
    // Pure-Rust simulation for Miri testing
    for i in 0..len {
        *data.add(i) *= 2;
    }
}
```

### Common FFI patterns that Miri flags

| Pattern | Miri error | Fix |
|---------|-----------|-----|
| `Vec::as_ptr()` then push | Dangling pointer | Take pointer after all mutations |
| Cast `&T` to `*mut T` and write | SharedReadOnly write | Use `&mut T` or `UnsafeCell` |
| `Box::from_raw` then use original ptr | Use-after-free | Don't keep aliases across `from_raw` |
| Read `*mut T` from C without init | Uninitialized read | Use `MaybeUninit` or write first |
| Cast `*const u8` to `*const u32` | Alignment violation | Use `read_unaligned` |
| Save raw ptr, create `&mut`, use old ptr | Stacked Borrows | Re-derive pointer from new borrow |

### Useful Miri flags

```sh
# Show more allocation history (helpful for dangling pointers)
MIRIFLAGS="-Zmiri-backtrace=full" cargo +nightly miri test

# Use Tree Borrows instead of Stacked Borrows
MIRIFLAGS="-Zmiri-tree-borrows" cargo +nightly miri test

# Disable data race detection (if using custom synchronization)
MIRIFLAGS="-Zmiri-disable-data-race-detector" cargo +nightly miri test

# Detect memory leaks
MIRIFLAGS="-Zmiri-leak-check" cargo +nightly miri test

# Increase timeout for slow tests
MIRIFLAGS="-Zmiri-isolation-error=warn" cargo +nightly miri test
```

---

## Cheat sheet: "Miri says X, I should..."

| Miri error message (key phrase) | Category | Typical fix |
|--------------------------------|----------|-------------|
| "allocation has been freed" | Use-after-free / dangling | Read before free; re-derive pointer after mutation |
| "trying to retag" / "not granting access" | Aliasing / Stacked Borrows | Don't hold multiple `&mut`; re-derive raw ptrs |
| "uninitialized data" / "Uninit" | Uninitialized read | Write before read; use `MaybeUninit` |
| "out of bounds" | OOB access | Fix index arithmetic |
| "SharedReadOnly" + "write" | `&T` → `*mut T` write | Use `&mut T`, `UnsafeCell`, or copy |
| "tag does not exist in the borrow stack" | Stacked Borrows (expired ptr) | Don't use old raw ptr after new `&mut` |
| "alignment N is required" | Unaligned access | `read_unaligned` / `write_unaligned` |
| "data race detected" | Data race | Add synchronization or use atomics |
| "can't call foreign function" | FFI not supported | Use `cfg(miri)` pure-Rust mock |
