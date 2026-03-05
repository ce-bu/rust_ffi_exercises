# C++ Vtable Layout and ABI — A Guide for Rust FFI

This document explains how C++ virtual function dispatch works at the
binary level, and how Rust code can call C++ virtual methods directly
by reading the vtable pointer from an object.  It covers the
**Itanium C++ ABI** (used by GCC, Clang, and most compilers on Linux,
macOS, and BSDs).

> **Scope:**  64-bit platforms (x86-64, AArch64) using the Itanium ABI.
> This does **not** apply to MSVC — see the [MSVC section](#msvc-differences)
> at the end.

---

## Table of Contents

1. [What is a vtable?](#1-what-is-a-vtable)
2. [Object layout (single inheritance)](#2-object-layout-single-inheritance)
3. [Vtable layout (Itanium ABI)](#3-vtable-layout-itanium-abi)
4. [Virtual destructors — the two-slot rule](#4-virtual-destructors--the-two-slot-rule)
5. [Multiple inheritance and pointer adjustment](#5-multiple-inheritance-and-pointer-adjustment)
6. [Calling convention on x86-64 Linux](#6-calling-convention-on-x86-64-linux)
7. [Calling virtual methods from Rust](#7-calling-virtual-methods-from-rust)
8. [In/out arguments through the vtable](#8-inout-arguments-through-the-vtable)
9. [Destruction through the vtable](#9-destruction-through-the-vtable)
10. [Investigating vtable layouts yourself](#10-investigating-vtable-layouts-yourself)
11. [MSVC differences](#11-msvc-differences)
12. [Portability and safety checklist](#12-portability-and-safety-checklist)

---

## 1. What is a vtable?

When a C++ class has at least one `virtual` function, the compiler
generates a **virtual function table** (vtable) — a static array of
function pointers, one per virtual method.  Each instance of the class
contains a hidden pointer to the vtable, called the **vptr**.

```
  C++ source                 Compiler output
 ┌──────────────────┐       ┌──────────────────────────────────────┐
 │ class IShape {   │       │ IShape vtable (static, read-only):   │
 │   virtual double │       │   [0] → IShape::~IShape() [D1]      │
 │     area();      │       │   [1] → IShape::~IShape() [D0]      │
 │   virtual double │       │   [2] → IShape::area()              │
 │     perimeter(); │       │   [3] → IShape::perimeter()         │
 │ };               │       └──────────────────────────────────────┘
 └──────────────────┘
```

When you write `shape->area()`, the compiler generates code like:

```c
// Pseudocode — what the compiler actually emits:
vptr   = *(void***)shape;         // read vptr from object
fn_ptr = vptr[2];                 // index slot 2 (area)
result = fn_ptr(shape);           // call with `this`
```

This is **exactly** what we replicate in Rust.

---

## 2. Object layout (single inheritance)

A minimal polymorphic object has the vptr as its first member,
followed by the class's data fields:

```
class Circle : public IShape {
    double cx_, cy_, r_;        // 3 doubles = 24 bytes
};

┌──────────────────────────────────────────────────────┐
│ Circle instance (sizeof = 32 on 64-bit)              │
├──────────┬───────────────────────────────────────────┤
│ offset 0 │ vptr ──────────┐                          │
│ offset 8 │ double cx_     │  points to               │
│ offset 16│ double cy_     │                          │
│ offset 24│ double r_      │  Circle vtable           │
└──────────┴────────────────┘  (with overridden        │
                                function pointers)     │
                               ┌───────────────────────┘
                               ▼
                   ┌──────────────────────────┐
                   │ offset_to_top: 0         │  (ptrdiff_t)
                   │ typeinfo: &Circle_ti     │  (void*)
                   ├──────────────────────────┤ ◀── vptr points here
                   │ slot 0: dtor_complete    │
                   │ slot 1: dtor_deleting    │
                   │ slot 2: Circle::area     │
                   │ slot 3: Circle::perimeter│
                   └──────────────────────────┘
```

**Key points:**

- The vptr is always at offset 0 in the object (for the primary base).
- The vptr points to the **first function-pointer entry**, not to the
  beginning of the vtable data.
- `offset_to_top` and `typeinfo` live at *negative* offsets from where
  the vptr points.

---

## 3. Vtable layout (Itanium ABI)

The full vtable structure (relative to where vptr points) is:

```
  Offset from vptr     Content                         Size
 ─────────────────────────────────────────────────────────────
  vptr − 16            offset_to_top (ptrdiff_t)       8 bytes
  vptr − 8             typeinfo pointer (void*)         8 bytes
 ─────────────────────────────────────────────────────────────
  vptr + 0             slot 0: first virtual function   8 bytes
  vptr + 8             slot 1: second virtual function  8 bytes
  vptr + 16            slot 2: third virtual function   8 bytes
  ...                  ...                              ...
```

**Slot ordering rule:** virtual functions appear in the order they are
**first declared** in the class hierarchy, starting with the topmost
base class, depth-first left-to-right.

For our `IShape` interface:

```cpp
class IShape {
    virtual ~IShape() = default;        // → slots 0, 1 (two destructor entries)
    virtual double area() const = 0;    // → slot 2
    virtual double perimeter() const;   // → slot 3
    virtual void bounding_box(...);     // → slot 4
    virtual void scale(...);            // → slot 5
};
```

In Rust, we mirror this as a `#[repr(C)]` struct:

```rust
#[repr(C)]
struct IShapeVtable {
    dtor_complete: unsafe extern "C" fn(*mut c_void),      // slot 0
    dtor_deleting: unsafe extern "C" fn(*mut c_void),      // slot 1
    area:          unsafe extern "C" fn(*const c_void) -> f64,  // slot 2
    perimeter:     unsafe extern "C" fn(*const c_void) -> f64,  // slot 3
    bounding_box:  unsafe extern "C" fn(*const c_void, ...),    // slot 4
    scale:         unsafe extern "C" fn(*mut c_void, ...),      // slot 5
}
```

---

## 4. Virtual destructors — the two-slot rule

A single virtual destructor occupies **two** vtable slots on the
Itanium ABI:

| Slot | Name | What it does |
|------|------|--------------|
| 0 | **Complete destructor (D1)** | Runs the destructor body + destroys bases. Does **not** free memory. |
| 1 | **Deleting destructor (D0)** | Runs the destructor body + destroys bases + calls `operator delete`. |

```
                    ┌──────────────────────────────────────┐
                    │  Vtable                              │
                    │                                      │
   vptr ──────────▶│  [0] ~Shape() { body; ~bases; }      │  D1 — no dealloc
                    │  [1] ~Shape() { body; ~bases;        │  D0 — with dealloc
                    │                 operator delete(p); }│
                    │  [2] area()                          │
                    │  ...                                 │
                    └──────────────────────────────────────┘
```

**Why two entries?**

- **D1** is used when destroying sub-objects (e.g., a base class
  within a derived class) — the memory belongs to the enclosing
  object, so we must not free it.
- **D0** is used for `delete ptr;` — we need to free the memory after
  running the destructor.

**From Rust:**  When you own the object (it was `new`'d), call
**slot 1** (D0, the deleting destructor).  This correctly destroys the
object *and* frees the memory.

---

## 5. Multiple inheritance and pointer adjustment

When a class inherits from multiple bases, the object contains
**multiple vptrs** — one for each polymorphic base.  The critical
consequence is that casting to a non-primary base **adjusts the
pointer**.

### Example

```cpp
class LabeledRect : public IShape, public ILabeled {
    double x_, y_, w_, h_;
    char label_[64];
};
```

**Object layout:**

```
 LabeledRect instance (sizeof = 112 bytes)
┌──────────────────────────────────────────────────────────────┐
│ offset  0 │ vptr_IShape  ─────────────────▶ IShape vtable   │
│ offset  8 │ vptr_ILabeled ────────────────▶ ILabeled vtable  │
│ offset 16 │ double x_                                        │
│ offset 24 │ double y_                                        │
│ offset 32 │ double w_                                        │
│ offset 40 │ double h_                                        │
│ offset 48 │ char label_[64]                                  │
└──────────────────────────────────────────────────────────────┘
     ▲                ▲
     │                │
  shape_ptr      labeled_ptr
 (no adjustment) (+8 adjustment)
```

When you cast a `LabeledRect*` to its bases:

```cpp
LabeledRect *obj = new LabeledRect(...);
IShape   *shape   = obj;   // No adjustment — same address
ILabeled *labeled = obj;   // Adjusted by +8 — different address!

// shape   == obj          (primary base, offset 0)
// labeled == (char*)obj + 8   (secondary base, offset 8)
```

**This is why the factory returns TWO pointers.** Rust must use the
correct pointer for each interface:

```
shape ────── for calling IShape methods   (area, perimeter, ...)
labeled ──── for calling ILabeled methods (get_label, set_label)
```

### `offset_to_top`

The vtable metadata at `vptr[-2]` stores the **offset from the
sub-object back to the complete object**:

```
Primary vtable (IShape):
    offset_to_top = 0       ← already at the start of the object

Secondary vtable (ILabeled):
    offset_to_top = -8      ← "go back 8 bytes to reach the full object"
```

You can read this from Rust to recover the original object address:

```rust
let meta = read_vtable_meta(labeled_ptr);
let full_object = (labeled_ptr as *const u8)
    .offset(meta.offset_to_top);  // -8 → goes back to shape_ptr
```

### Thunks

The secondary vtable may contain **thunk** functions instead of
direct pointers to the override.  A thunk is a tiny compiler-generated
function that adjusts `this` before calling the real implementation:

```asm
; Thunk for LabeledRect::~LabeledRect() via ILabeled
_thunk:
    sub rdi, 8          ; adjust this from ILabeled* to LabeledRect*
    jmp _real_destructor
```

From Rust's perspective, thunks are transparent — you just call the
function pointer in the vtable with the correct sub-object pointer,
and the thunk handles the rest.

---

## 6. Calling convention on x86-64 Linux

On x86-64 Linux (and macOS), C and C++ non-static member functions use
the **same calling convention** — the System V AMD64 ABI:

| Register | Purpose |
|----------|---------|
| `rdi` | 1st integer/pointer arg ← **`this`** |
| `rsi` | 2nd integer/pointer arg |
| `rdx` | 3rd integer/pointer arg |
| `rcx` | 4th integer/pointer arg |
| `r8` | 5th integer/pointer arg |
| `r9` | 6th integer/pointer arg |
| `xmm0`–`xmm7` | floating-point args |
| `rax` / `xmm0` | return value |

**Crucially**, there is no special `__thiscall` convention on x86-64
Linux (unlike 32-bit Windows).  `this` is simply the first argument.
This means Rust's `extern "C"` function pointers work directly for
calling C++ virtual methods:

```rust
// Rust: extern "C" fn(this, factor, inout_area)
//   → this in rdi, factor in xmm0, inout_area in rsi

// C++:  void IShape::scale(double factor, double* inout_area)
//   → this in rdi, factor in xmm0, inout_area in rsi

// ✓ Same registers, same convention!
```

### Example: `void bounding_box(double*, double*, double*, double*) const`

```
Argument        C++ register    Rust `extern "C"` register
──────────────────────────────────────────────────────────
this (const)    rdi             rdi              ← 1st ptr
out_x           rsi             rsi              ← 2nd ptr
out_y           rdx             rdx              ← 3rd ptr
out_w           rcx             rcx              ← 4th ptr
out_h           r8              r8               ← 5th ptr
```

---

## 7. Calling virtual methods from Rust

The complete recipe:

### Step 1: Mirror the vtable as a `#[repr(C)]` struct

```rust
#[repr(C)]
pub struct IShapeVtable {
    pub dtor_complete: unsafe extern "C" fn(*mut c_void),
    pub dtor_deleting: unsafe extern "C" fn(*mut c_void),
    pub area:          unsafe extern "C" fn(*const c_void) -> f64,
    pub perimeter:     unsafe extern "C" fn(*const c_void) -> f64,
    // ... more slots in declaration order
}
```

### Step 2: Read the vptr

```rust
unsafe fn read_vptr<V>(subobject: *const c_void) -> *const V {
    *(subobject as *const *const V)
}
```

### Step 3: Call through the vtable

```rust
let vtable = unsafe { &*read_vptr::<IShapeVtable>(shape_ptr) };
let area   = unsafe { (vtable.area)(shape_ptr) };
```

That's it.  Three lines of code replace what would otherwise be an
`extern "C"` trampoline function defined in C++.

### Visualization of the call path

```
                Rust code                          Memory
           ┌───────────────────┐
           │ let vtable =      │
           │   *read_vptr(ptr) │─── reads 8 bytes at ptr ──▶ ┌──────────┐
           │                   │                              │   vptr   │
           │ (vtable.area)(ptr)│◀── gets fn ptr from ────────│ slot [2] │
           │                   │    vtable.area               └──────────┘
           │                   │                                   │
           │    area = result  │◀──── calls fn(ptr) ───────────────┘
           └───────────────────┘              │
                                              ▼
                                   C++ compiled code for
                                   ConcreteClass::area()
```

---

## 8. In/out arguments through the vtable

The calling convention for in/out arguments is the same whether you
call through a trampoline or directly through the vtable.

### Out-parameters (callee writes)

```cpp
// C++
virtual void bounding_box(double *out_x, double *out_y,
                          double *out_w, double *out_h) const = 0;
```

```rust
// Rust — caller provides mutable references
let (mut x, mut y, mut w, mut h) = (0.0f64, 0.0, 0.0, 0.0);
unsafe {
    let vt = &*read_vptr::<IShapeVtable>(shape_ptr);
    (vt.bounding_box)(shape_ptr, &mut x, &mut y, &mut w, &mut h);
}
// x, y, w, h now contain the values written by C++
```

### In/out-parameters (caller writes in, callee writes out)

```cpp
// C++
virtual void scale(double factor, double *inout_area) = 0;
// On entry: *inout_area = caller's current area
// On exit:  *inout_area = new area after scaling
```

```rust
// Rust
let mut area = current_area;           // "in" value
unsafe {
    let vt = &*read_vptr::<IShapeVtable>(shape_ptr);
    (vt.scale)(shape_ptr, factor, &mut area);
}
let new_area = area;                   // "out" value
```

---

## 9. Destruction through the vtable

Since the deleting destructor is at vtable slot 1, you can destroy and
free a C++ object **without any extern "C" helper**:

```rust
impl Drop for CppShape {
    fn drop(&mut self) {
        unsafe {
            let vt = &*read_vptr::<IShapeVtable>(self.shape_ptr);
            (vt.dtor_deleting)(self.shape_ptr);
            //  ^^^^^^^^^^^^^^
            //  slot 1 = D0: destroys object + calls operator delete
        }
    }
}
```

**Important:** Always call the deleting destructor through the
**primary base's** vtable.  The primary base pointer equals the
original `new`'d address, so `operator delete` gets the right pointer.

```
 ┌──────────────────────────────────────────────────────────────┐
 │  ✓ CORRECT:  (primary_vtable.dtor_deleting)(shape_ptr)      │
 │               shape_ptr == allocation address                │
 │                                                              │
 │  ⚠ RISKY:    (secondary_vtable.dtor_deleting)(labeled_ptr)  │
 │               Works due to thunk, but adds unnecessary       │
 │               complexity; avoid in production code.          │
 └──────────────────────────────────────────────────────────────┘
```

---

## 10. Investigating vtable layouts yourself

Before writing the Rust vtable struct, you should verify the actual
compiler output.  Here are several practical techniques:

### Clang: dump vtable layouts

```bash
clang++ -Xclang -fdump-vtable-layouts -c myfile.cpp 2>&1 | c++filt
```

Sample output:

```
Vtable for 'LabeledRect' (13 entries).
   0 | offset_to_top (0)
   1 | LabeledRect RTTI
       -- (IShape, 0) vtable address --
   2 | LabeledRect::~LabeledRect() [complete]
   3 | LabeledRect::~LabeledRect() [deleting]
   4 | LabeledRect::area() const
   5 | LabeledRect::perimeter() const
   6 | LabeledRect::bounding_box(...) const
   7 | LabeledRect::scale(...)
   8 | offset_to_top (-8)
   9 | LabeledRect RTTI
       -- (ILabeled, 8) vtable address --
  10 | LabeledRect::~LabeledRect() [complete] [thunk]
  11 | LabeledRect::~LabeledRect() [deleting] [thunk]
  12 | LabeledRect::get_label(...) const [thunk]
  13 | LabeledRect::set_label(...) [thunk]
```

Note how: entries 0–1 are metadata for the primary base, entries 2–7
are IShape function slots, and entries 8–13 are the secondary
(ILabeled) vtable with its own metadata and thunks.

### GCC: dump class layout

```bash
g++ -fdump-lang-class -c myfile.cpp
cat myfile.cpp.*.class    # look for "Vtable for ..."
```

### GDB: inspect at runtime

```gdb
(gdb) info vtbl *shape_ptr
(gdb) x/8gx *(void**)shape_ptr - 2    # show vtable with metadata
```

### `pahole`: show struct layout with offsets

```bash
g++ -g -c myfile.cpp -o myfile.o
pahole myfile.o
```

### Writing a C++ verification helper

You can add a minimal `extern "C"` function that reads vtable slots
from the C++ side, then compare in Rust tests:

```cpp
extern "C" void *cpp_vt_read_slot(const void *obj, int slot) {
    void **vptr = *(void***)obj;
    return vptr[slot];
}
```

```rust
#[test]
fn verify_area_slot() {
    let vt = unsafe { &*read_vptr::<IShapeVtable>(shape_ptr) };
    let rust_fn = vt.area as *const ();
    let cpp_fn  = unsafe { cpp_vt_read_slot(shape_ptr, 2) };
    assert_eq!(rust_fn, cpp_fn as *const ());
}
```

---

## 11. MSVC differences

The Microsoft Visual C++ compiler uses a **completely different**
vtable layout.  If you need to support MSVC, you **cannot** use the
technique described in this document.  Key differences:

| Feature | Itanium ABI (GCC/Clang) | MSVC |
|---------|------------------------|------|
| Destructor slots | 2 (D1 + D0) | 1 (with hidden flag) |
| `offset_to_top` | In vtable metadata | Separate "vbase" table |
| `typeinfo` | In vtable metadata | Separate RTTI structure |
| Thunks for MI | Adjust `this` pointer | "Adjustor thunks" (similar but different encoding) |
| Vtable pointer name | vptr | vfptr / vbptr |
| 32-bit calling convention | cdecl (this in first arg) | `__thiscall` (this in ecx) |
| 64-bit calling convention | SysV ABI | Microsoft x64 ABI |

**On 64-bit Windows with MSVC:**  The calling convention difference
(Microsoft x64 vs SysV) means you also can't just use
`extern "C" fn(this, ...)` — the register assignments differ
(rcx/rdx/r8/r9 vs rdi/rsi/rdx/rcx).

### Recommendation for portability

If you need cross-platform support:

1. **Use `extern "C"` trampoline functions** — they work with all
   compilers and ABIs.
2. **Use a binding generator** like `cxx` or `autocxx` that abstracts
   over compiler differences.
3. **Use direct vtable access only** when you control the target
   platform and compiler (e.g., a Linux-only embedded system).

---

## 12. Portability and safety checklist

Before using direct vtable access in production, verify:

- [ ] **Compiler:** Using GCC or Clang (Itanium ABI)
- [ ] **Architecture:** 64-bit (vtable slot sizing assumes 8-byte pointers)
- [ ] **No `-fapple-kext`:** This flag changes vtable layout on macOS
- [ ] **No virtual base classes:** Virtual inheritance adds additional
      vtable entries (vbase offsets) that change the slot numbering
- [ ] **Slot verification:** Tests compare Rust slot reads against C++
      introspection helper
- [ ] **RTTI setting (`-fno-rtti`):** Does NOT change vtable layout,
      but the `typeinfo` slot will be null — safe to ignore
- [ ] **Exception safety:** If C++ methods can throw, wrap calls with
      `extern "C-unwind"` (Rust 1.71+) or ensure `-fno-exceptions`
- [ ] **Class changes:** Any modification to the virtual method
      declarations (reordering, adding, removing) will silently
      change slot indices — always re-verify after C++ header changes

### When to use this technique

| Scenario | Recommended approach |
|----------|---------------------|
| Controlled Linux environment, performance-critical | ✅ Direct vtable access |
| Cross-platform library | ❌ Use trampolines or `cxx` |
| Calling 1–2 methods occasionally | ❌ Trampoline is simpler |
| Wrapping a large C++ interface with many methods | ✅ Avoids writing dozens of trampolines |
| Prototyping / exploring a C++ library | ✅ Fast iteration |

---

## References

- [Itanium C++ ABI — Virtual Table Layout](https://itanium-cxx-abi.github.io/cxx-abi/abi.html#vtable)
- [System V AMD64 ABI](https://gitlab.com/x86-psABIs/x86-64-ABI)
- [LLVM Blog — C++ Vtables](https://blog.llvm.org/posts/2021-01-05-vtable-interleaving/)
- The `cxx` crate: <https://cxx.rs/>
