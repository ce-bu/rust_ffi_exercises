# Rust FFI Exercises — Study Plan

Work through the exercises in order.  Each builds on concepts from the previous ones.

Run a single exercise: `cargo test ex01`
Run all exercises: `cargo test`

---

## Exercises

- [ ] **Exercise 01 — Call C from Rust** (`src/ex01_calling_c.rs`)
  Declare C functions in an `extern "C"` block and write safe Rust wrappers.
  *Concepts:* `extern "C"`, type mapping (`int`→`i32`, `double`→`f64`), `unsafe` blocks.

- [ ] **Exercise 02 — Call Rust from C** (`src/ex02_rust_from_c.rs`)
  Write `#[no_mangle] pub extern "C"` functions that C code can call.
  *Concepts:* `#[no_mangle]`, C calling convention, symbol export.

- [ ] **Exercise 03 — `#[repr(C)]` Structs** (`src/ex03_structs.rs`)
  Define C-compatible structs and pass them by value and by pointer.
  *Concepts:* `#[repr(C)]`, pass-by-value vs pass-by-pointer, `*const T` / `*mut T`.

### Strings & Data

- [ ] **Exercise 04 — C-String Passing** (`src/ex04_strings.rs`)
  Convert between Rust `&str`/`String` and C `*const c_char`/`*mut c_char`.
  *Concepts:* `CStr` (borrow), `CString` (own), `into_raw()` / `from_raw()`, caller-provided buffers.

- [ ] **Exercise 05 — Opaque Handles** (`src/ex05_opaque.rs`)
  Hide a Rust `HashMap` behind an opaque `*mut Config` pointer.
  *Concepts:* `Box::into_raw` / `Box::from_raw`, create→use→destroy lifecycle, non-`repr(C)` types.

- [ ] **Exercise 06 — Arrays & Slices** (`src/ex06_arrays.rs`)
  Pass arrays as `(pointer, length)` pairs across the FFI boundary.
  *Concepts:* `slice::from_raw_parts`, `Vec` ↔ `(ptr, len)` via `mem::forget`, `from_raw_parts`.

- [ ] **Exercise 07 — Callbacks** (`src/ex07_callbacks.rs`)
  Function-pointer callbacks in both directions (Rust→C and C→Rust), including the `void* user_data` context pattern.
  *Concepts:* `extern "C" fn` pointers, context/"closure" via `*mut c_void`, Rust fn as C callback.

- [ ] **Exercise 08 — Memory Management** (`src/ex08_memory.rs`)
  Allocate buffers in Rust, hand them to C, free them correctly. Builder pattern with an opaque handle.
  *Concepts:* "whoever allocates must free", `mem::forget`, `Vec::from_raw_parts`, out-parameters.

- [ ] **Exercise 09 — Error Handling** (`src/ex09_errors.rs`)
  Integer return codes + `thread_local!` last-error message (like `errno` / `GetLastError()`).
  *Concepts:* `thread_local!`, `RefCell<Option<CString>>`, error code conventions.

### Advanced

- [ ] **Exercise 10 — `MaybeUninit`, `NonNull` & `ManuallyDrop`** (`src/ex10_maybe_uninit.rs`)
  Safer FFI with `MaybeUninit` for out-parameters, `NonNull` for handle wrappers, and `ManuallyDrop` for transferring ownership of Rust-allocated buffers to C without running destructors.
  *Concepts:* `MaybeUninit::write()`, `NonNull::new()`, `ManuallyDrop`, `Vec::from_raw_parts`, avoiding UB from uninitialized reads, ownership transfer.

- [ ] **Exercise 11 — C++ Vtable Pattern** (`src/ex11_vtable.rs`)
  Implement Rust types behind a C-compatible vtable (`struct of function pointers`) and call them from C++.
  *Concepts:* vtable as `#[repr(C)]` struct, `static` vtable, polymorphic dispatch, C++ interop.

- [ ] **Exercise 12 — Async / Threaded Interop** (`src/ex12_async.rs`)
  Bridge async Rust with synchronous C: completion callbacks, `spawn_blocking`, opaque runtime handles.
  *Concepts:* `std::thread::spawn`, `Send` for raw pointers, `tokio::task::spawn_blocking`, runtime lifecycle.

- [ ] **Exercise 13 — Tagged Unions** (`src/ex13_tagged_unions.rs`)
  Represent Rust enums as C-compatible tagged unions (`#[repr(C)]` struct + union).
  *Concepts:* `#[repr(u32)]`, `#[repr(C)] union`, reading union fields unsafely, discriminant checking.

- [ ] **Exercise 14 — Panic Safety** (`src/ex14_panic_safety.rs`)
  Prevent undefined behavior by catching panics at every `extern "C"` boundary.
  *Concepts:* `std::panic::catch_unwind`, `UnwindSafe`, converting panics to error codes.

- [ ] **Exercise 15 — Fixing UB with Miri** (`src/ex15_miri_ub.rs`)
  Eight functions containing **intentional undefined behavior** — dangling pointers, aliasing violations, use-after-free, unaligned access, uninitialized reads, and Stacked Borrows violations. Use `cargo miri test` to detect each bug, then write a fixed version.
  *Concepts:* Miri, Stacked Borrows, `ptr::read_unaligned`, `split_at_mut`, `ptr::write`, use-after-free.

- [ ] **Exercise 16 — Pin & Drop Guarantee** (`src/ex16_pin_drop.rs`)
  Implement a `Listener` type that registers a raw pointer with a simulated C callback registry. The object must not move (or C's pointer dangles), and `Drop` must deregister before memory is freed. Build a safe `PinnedListener` wrapper.
  *Concepts:* `Pin<Box<T>>`, `PhantomPinned`, `!Unpin`, drop guarantee, self-registering objects, safe API over unsafe internals.

- [ ] **Exercise 17 — Wrapping C Opaque Handles** (`src/ex17_wrapping_c.rs`)
  Wrap a C library (`csrc/ex17_cdb.c`) that uses opaque handles in a safe, idiomatic Rust API. Declare `extern "C"` bindings, build an RAII `Database` struct with `NonNull` + `Drop`, and convert C error codes to `Result`.
  *Concepts:* `NonNull<OpaqueType>`, RAII wrapper with `Drop`, `CString` for outbound strings, caller-provided buffers, `From<c_int>` error mapping, safe API over raw FFI.

- [ ] **Exercise 18 — Invoking Rust Closures from C** (`src/ex18_closures.rs`)
  Pass Rust closures (`Fn`, `FnMut`, `FnOnce`) through C function-pointer + `void*` context pairs using the trampoline pattern. Covers shared references for `Fn`, mutable references for `FnMut`, and `Box::into_raw`/`Box::from_raw` ownership transfer for `FnOnce`.
  *Concepts:* Trampoline functions, generic `extern "C" fn`, `Fn`/`FnMut`/`FnOnce` across FFI, `*mut c_void` as closure context, `Box::into_raw` for ownership transfer.

- [ ] **Exercise 19 — Boxed `dyn Trait` Across FFI** (`src/ex19_dyn_trait_ffi.rs`)
  Pass `Box<dyn Trait>` to C as opaque handles. Covers why fat pointers need a wrapper struct, RAII lifecycle, composable trait objects (filter, tee), a dynamic registry (plugin pattern with `Vec<Box<dyn Trait>>`), and downcasting back to concrete types via `Any`.
  *Concepts:* Fat vs thin pointers, `Box<dyn Trait>` in a concrete wrapper, opaque handle pattern, trait object composition, `Drop`-based cleanup, dynamic dispatch registry, `Any` + `downcast_ref`.

- [ ] **Exercise 20 — Global State & Library Init/Shutdown** (`src/ex20_global_state.rs`)
  Build a thread-safe metrics library with `init()` / `shutdown()` lifecycle, named counters, and concurrent access.  Models the pattern used by `curl_global_init()`, `SSL_library_init()`, etc.
  *Concepts:* `OnceLock`, `Mutex`, global mutable state, idempotent init, thread-safe `extern "C"` API.

- [ ] **Exercise 21 — Bitflags & C-Style Enums** (`src/ex21_bitflags.rs`)
  Define a `#[repr(transparent)]` newtype for bitwise-OR flags (`O_RDONLY | O_CREAT`-style).  Implement `BitOr`, `BitAnd`, `contains()`, and use flags to control a simulated file API.
  *Concepts:* `#[repr(transparent)]`, bitwise operator traits, `#[repr(u32)]` enums, flag validation, `From<Enum>` for flags.

- [ ] **Exercise 22 — Lifetime Encoding with `PhantomData`** (`src/ex22_lifetimes.rs`)
  Wrap a simulated DB + cursor API where cursors *borrow* from the database.  Use `PhantomData<&'db T>` to let the compiler prevent use-after-free.  Also covers borrowed transactions (`&mut` borrow blocks concurrent access).
  *Concepts:* `PhantomData`, lifetime parameters on FFI wrappers, borrowed vs owned handles, compile-time enforcement of C lifetime contracts.

- [ ] **Exercise 23 — `transmute` & Type Reinterpretation** (`src/ex23_transmute.rs`)
  Learn when `mem::transmute` is truly needed in FFI (fn ptr ↔ `void*`, nullable callbacks) and when safe alternatives exist (`TryFrom` for enums, `ptr::read_unaligned` for struct deserialization, `as` casts, `f32::from_bits`).  Each part shows the `transmute` approach AND the preferred safe alternative.
  *Concepts:* `mem::transmute`, `#[repr(C, packed)]`, `ptr::read_unaligned`, `Option<extern "C" fn()>` null optimization, `TryFrom` for C enums, `slice::from_raw_parts` for serialization.

- [ ] **Exercise 24 — C-Owned Opaque Handles** (`src/ex24_c_handles.rs`)
  The C library allocates an opaque `Session` handle internally and returns it via an **out-parameter** (`Session **out`). Rust receives the handle, wraps it in an RAII struct with `NonNull` + `Drop`, and builds a safe API where each C function's first `Session *` argument becomes `&self` / `&mut self`.
  *Concepts:* Out-parameter pattern (`*mut *mut T`), C-owned opaque handles, `NonNull` + `Drop` (C frees, not Rust), zero-sized opaque type, status-code → `Result` conversion, `CString` for outbound strings.

- [ ] **Exercise 25 — Calling C++ Virtual Functions (Direct Vtable Access)** (`src/ex25_cpp_virtual.rs`)
  Receive a pointer to a polymorphic C++ object (abstract interface with multiple inheritance) and call its virtual methods from Rust — **without writing any extern "C" trampoline functions**.  Instead, read the vptr from the object and call function pointers by slot index, exploiting the Itanium C++ ABI vtable layout.  Covers in/out arguments, out-parameters, destruction via the vtable's deleting destructor, and MI pointer adjustment.
  *Concepts:* Itanium C++ ABI vtable layout, vptr reading, `#[repr(C)]` vtable structs, multiple inheritance pointer adjustment, `offset_to_top`, deleting vs complete destructors, in/out and out-parameters across virtual calls.
  *See also:* `docs/ex25_cpp_vtable_abi.md` for a detailed ABI guide with diagrams.

---

## Commands

```sh
cargo test           # run all exercises
cargo test ex01      # run exercise 01 only
cargo test ex05      # run exercise 05 only
cargo build          # compile (also builds C/C++ via build.rs)
```

---

## Estimated Time to Complete

For an experienced Rust developer with **no prior FFI experience**.  Times include reading the exercise, implementing the solution, and passing all tests.

| # | Exercise | Est. Time | Difficulty |
|---|----------|-----------|------------|
| 01 | Call C from Rust | 20 min | ★☆☆☆☆ |
| 02 | Call Rust from C | 20 min | ★☆☆☆☆ |
| 03 | `#[repr(C)]` Structs | 25 min | ★☆☆☆☆ |
| 04 | C-String Passing | 30 min | ★★☆☆☆ |
| 05 | Opaque Handles (Rust-owned) | 30 min | ★★☆☆☆ |
| 06 | Arrays & Slices | 30 min | ★★☆☆☆ |
| 07 | Callbacks | 40 min | ★★☆☆☆ |
| 08 | Memory Management | 45 min | ★★★☆☆ |
| 09 | Error Handling | 40 min | ★★☆☆☆ |
| 10 | `MaybeUninit`, `NonNull` & `ManuallyDrop` | 55 min | ★★★☆☆ |
| 11 | C++ Vtable Pattern | 60 min | ★★★★☆ |
| 12 | Async / Threaded Interop | 60 min | ★★★★☆ |
| 13 | Tagged Unions | 40 min | ★★★☆☆ |
| 14 | Panic Safety | 30 min | ★★☆☆☆ |
| 15 | Fixing UB with Miri | 60 min | ★★★★☆ |
| 16 | Pin & Drop Guarantee | 60 min | ★★★★☆ |
| 17 | Wrapping C Opaque Handles | 50 min | ★★★☆☆ |
| 18 | Closures via Trampolines | 50 min | ★★★☆☆ |
| 19 | Boxed `dyn Trait` Across FFI | 65 min | ★★★★☆ |
| 20 | Global State & Init/Shutdown | 35 min | ★★☆☆☆ |
| 21 | Bitflags & C-Style Enums | 35 min | ★★☆☆☆ |
| 22 | Lifetimes with `PhantomData` | 45 min | ★★★☆☆ |
| 23 | `transmute` & Reinterpretation | 45 min | ★★★☆☆ |
| 24 | C-Owned Opaque Handles | 45 min | ★★★☆☆ |
| 25 | C++ Virtual Functions (Direct Vtable) | 75 min | ★★★★★ |
| | **Total** | **~18.5 hours** | |

> **Tip:** Exercises 01–07 build linearly — do them in order.  After that, exercises are mostly independent and can be tackled in any order based on interest.  Exercise 25 requires understanding ex11 (vtable pattern) first.

---

## Hints

Stuck?  These hints cover the most common stumbling blocks across all exercises.  Each hint gives just enough to unblock you without spoiling the solution.

### General FFI

- **`unsafe` is a contract, not a warning.**  When you write `unsafe { }`, you are *promising* that the preconditions documented on the function hold.  Read the `# Safety` doc comment before every `unsafe` call.
- **Ownership always matters.**  Before touching a raw pointer ask: "who owns this memory?"  If Rust allocated it (`Box::into_raw`, `CString::into_raw`, `Vec::as_mut_ptr` + `mem::forget`), Rust must free it.  If C allocated it, C must free it.
- **`repr(C)` ≠ `repr(Rust)`.**  If a struct crosses the FFI boundary, it needs `#[repr(C)]`.  Opaque handles that C never dereferences do NOT need `repr(C)` — only a pointer crosses.

### Strings (ex04)

- `CStr::from_ptr(ptr)` **borrows** — the pointer must outlive the reference.
- `CString::new(s).unwrap()` allocates — use `.as_ptr()` while the `CString` is alive.
- `CString::into_raw()` transfers ownership to C; reclaim with `CString::from_raw()`.
- Watch out for interior NUL bytes — `CString::new` returns `Err` if the input contains `\0`.

### Opaque Handles (ex05, ex17)

- The pattern is always: `Box::new(value)` → `Box::into_raw()` → C holds the pointer → `Box::from_raw()` to reclaim.
- The *type* behind the pointer does NOT need `#[repr(C)]` as long as C only stores the pointer and never dereferences the fields.
- To wrap a C library's handle, use `NonNull<OpaqueType>` + `Drop`.

### Callbacks & Closures (ex07, ex18)

- C function pointers are `extern "C" fn(...)` — they **cannot** capture environment.
- To pass state, use the `void *user_data` pattern: cast your state to `*mut c_void`, pass it alongside the fn pointer, cast it back inside the callback.
- For Rust closures, write a **trampoline**: a generic `extern "C" fn<F: Fn…>` that casts the `void*` back to `&F` / `&mut F` / `Box<F>` depending on the trait.
- `Fn` → pass `&closure` as context (shared borrow, not consumed).
- `FnMut` → pass `&mut closure` as context (exclusive borrow, not consumed).
- `FnOnce` → pass `Box::into_raw(Box::new(closure))` as context (consumed via `Box::from_raw`).

### dyn Trait (ex19)

- `Box<dyn Trait>` is a **fat pointer** (2 words).  C can't store that.
- Wrap it: `struct Handle { inner: Box<dyn Trait> }`.  Now `Box::into_raw(Box::new(handle))` is a single-word (thin) pointer.
- `Drop` on the wrapper frees both the wrapper and the trait object.
- **Registry pattern:** store `Vec<Box<dyn Logger>>` inside a struct behind an opaque handle.  `add()` pushes, `broadcast()` iterates and calls `.log()` on each.  C sees one thin pointer hiding an arbitrarily large collection.
- **Downcasting:** add `fn as_any(&self) -> &dyn Any` to your trait.  Each impl returns `self`.  Then `handle.inner.as_any().downcast_ref::<ConcreteType>()` recovers the concrete type.  Returns `None` if the type doesn't match.
- `Any` requires `'static` — your concrete types must not hold non-`'static` borrows.

### Memory (ex08)

- `mem::forget(vec)` prevents `Drop` — you now own the raw pointer.
- Reconstruct with `Vec::from_raw_parts(ptr, len, cap)` to free.
- **Never** mix allocators: don't `free()` Rust memory or `Box::from_raw` C memory.

### ManuallyDrop (ex10 Part D)

- `ManuallyDrop::new(v)` wraps a value and suppresses its destructor — the value stays valid but won't be freed when it goes out of scope.
- Use this when transferring a `Vec` to C: wrap in `ManuallyDrop`, extract `(ptr, len, capacity)`, hand the triple to C.
- To reclaim: `Vec::from_raw_parts(ptr, len, cap)` — the resulting `Vec` drops normally.
- **`ManuallyDrop` vs `mem::forget`:** both suppress drop, but `ManuallyDrop` lets you read fields (`.as_ptr()`, `.len()`) *after* suppressing drop.  `mem::forget` consumes the value so you can't access it afterwards.

### Error Handling (ex09)

- Return `0` for success, negative for error.  Store the full message in a `thread_local!`.
- `thread_local!` + `RefCell<Option<CString>>` is the Rust equivalent of `errno` + `strerror`.

### Panic Safety (ex14)

- Unwinding across `extern "C"` is **undefined behavior**.  Always wrap with `std::panic::catch_unwind`.
- `catch_unwind` requires `UnwindSafe`.  Use `AssertUnwindSafe(|| { ... })` if the compiler complains.
- Pattern: `catch_unwind(|| { /* real work */ }).unwrap_or_else(|_| ERROR_CODE)`.

### Global State (ex20)

- `static GLOBAL: OnceLock<Mutex<T>> = OnceLock::new();` — initialized exactly once, thread-safe.
- `OnceLock::get_or_init(|| ...)` is idempotent and lock-free after first init.
- Always check that the library was initialized before accessing state.

### Bitflags (ex21)

- Use `#[repr(transparent)]` on a newtype: `struct Flags(u32)`.  ABI identical to `u32`.
- `contains(other)`: `(self.0 & other.0) == other.0`.
- Implement `BitOr`, `BitAnd`, `BitOrAssign` — they're one-liners.

### Lifetimes & PhantomData (ex22)

- `PhantomData<&'a T>` tells the compiler "I logically borrow a `&'a T`" without storing one.
- Use it when a raw pointer *should* have a lifetime but can't (because raw pointers are `'static`).
- The compiler will then prevent use-after-free at compile time.
- `&self` borrow in `cursor()` → multiple cursors OK, no mutation.  `&mut self` borrow in `transaction()` → exclusive access.

### Transmute (ex23)

- **Only use `transmute` when there is no safe alternative.**  The most common legitimate case: `*mut c_void` ↔ function pointer — `as` casts don't work here.
- `Option<extern "C" fn(...)>` is **guaranteed** to have the same layout as a nullable pointer.  You do NOT need transmute for optional callbacks — just declare them as `Option<…>` in your `extern "C"` block.
- **Integer → `#[repr(C)]` enum:** prefer `match` + `TryFrom`.  If you transmute an integer that doesn't match any variant, it's **instant UB** — even if you never read the value.
- **Bytes → struct:** prefer `ptr::read_unaligned` + size check over transmute.  `transmute` requires exact size and alignment; `read_unaligned` is more forgiving.
- **Bytes → struct with bool/enum fields:** even `ptr::read_unaligned` can be UB if the bit pattern is invalid for those types.  Validate first or deserialize field by field.
- **Safe alternatives cheat sheet:**
  - `*mut T` ↔ `*mut c_void` → use `as`
  - `f32` ↔ `u32` bits → `f32::from_bits` / `to_bits`
  - Integer ↔ integer → `as` or `From`/`TryFrom`
  - Struct → bytes → `slice::from_raw_parts(ptr as *const u8, size_of::<T>())`

### C-Owned Opaque Handles (ex24)

- The C library calls `calloc` / `malloc` — **Rust must NOT call `Box::from_raw`**.  Call C's own `destroy` function in your `Drop`.
- The **out-parameter** pattern:  `let mut ptr: *mut CSession = std::ptr::null_mut();` then pass `&mut ptr` (which is `*mut *mut CSession`) to the C function.
- Wrap the raw pointer in `NonNull<CSession>` immediately after creation.  `NonNull::new(ptr).ok_or(Error)?` handles the null case.
- For `get_option`-style functions that fill a caller-provided buffer, use a stack array: `let mut buf = [0u8; 256];`.  After the C call, find the NUL terminator with `CStr::from_ptr(buf.as_ptr() as *const c_char)`.
- `&self` vs `&mut self`: functions that only read (`is_connected`, `get_option`) take `&self`; functions that mutate state (`connect`, `send`) take `&mut self`.

### Vtable / C++ (ex11)

- A vtable is just a `#[repr(C)]` struct of `extern "C" fn` pointers.
- Use `static` vtables (not heap-allocated) — one per concrete type.
- The `destroy` function pointer in the vtable must reconstruct the `Box` to free.

### Direct C++ Vtable Access (ex25)

- **Itanium ABI only:** This works on GCC/Clang (Linux, macOS, BSD).  It does NOT work on MSVC.
- The **vptr** is the first field of every polymorphic C++ object.  Read it: `let vptr = *(obj as *const *const VtableStruct)`.
- Virtual destructors occupy **two slots** in the Itanium ABI: slot 0 = complete destructor (D1, no dealloc), slot 1 = deleting destructor (D0, with dealloc).  Your methods start at slot 2.
- To destroy: call the **deleting destructor** (slot 1) through the **primary base's** vtable.  The primary base pointer equals the allocation address.
- **Multiple inheritance:** A class inheriting from N bases has N vptrs in its object layout.  Casting to a non-primary base **adjusts the pointer**.  Always use the correct interface pointer for each vtable.
- **Verify slot numbering:** Use `clang++ -Xclang -fdump-vtable-layouts -c file.cpp 2>&1 | c++filt` to dump the vtable.  Or write a C++ helper that reads `vptr[slot]` and compare against your `#[repr(C)]` struct in tests.
- **In/out arguments** work the same through direct vtable calls — just pass `&mut value` as `*mut f64`.  The virtual method writes through the pointer.
- `offset_to_top` at `vptr[-2]` tells you the byte offset from the sub-object back to the complete object — useful for MI diagnostics.
- See `docs/ex25_cpp_vtable_abi.md` for the full ABI reference with ASCII diagrams.

### Pin & Drop (ex16)

- Use `Pin<Box<T>>` when C stores a raw pointer back to your struct.
- `PhantomPinned` makes the type `!Unpin` so it can't be moved out of the `Pin`.
- `Drop` **must** deregister from C before the memory is freed.

### Miri (ex15)

- Run with `cargo +nightly miri test ex15`.
- Miri catches: dangling pointers, aliasing violations (`&` and `&mut` to same address), unaligned reads, use-after-free.
- `ptr::read_unaligned` for unaligned access; `split_at_mut` to get two `&mut` to non-overlapping parts.

### Debugging Tips

- **Segfault?**  You probably used a pointer after free, or cast to the wrong type.  Run under Miri or Valgrind.
- **Garbled strings?**  Check that your `CString` is still alive when C reads the pointer.  A common bug: `CString::new(s).unwrap().as_ptr()` — the `CString` is a temporary and is freed immediately!
- **Link errors?**  Make sure `build.rs` compiles your C file and the function name matches exactly (no name mangling — use `extern "C"`).
- **Use `cargo test -- --nocapture`** to see `println!` output from tests.
- **Use `#[repr(C)]` and check sizes:** `assert_eq!(std::mem::size_of::<MyStruct>(), EXPECTED_SIZE);` catches layout mismatches early.
