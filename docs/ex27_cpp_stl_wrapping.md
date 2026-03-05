# Wrapping C++ Classes & STL Types for Rust FFI

> **Exercise 27** — companion material  
> Pattern: opaque-pointer wrapping of non-virtual C++ classes

---

## 1. The Core Problem

C++ classes have features with no C ABI equivalent:

| C++ feature            | C ABI?  | Solution                              |
|------------------------|---------|---------------------------------------|
| Constructor            | No      | Factory function (`cpp_foo_new`)      |
| Destructor             | No      | Explicit destroy (`cpp_foo_destroy`)  |
| Copy constructor       | No      | Clone function (`cpp_foo_clone`)      |
| `std::string`          | No      | `const char* + size_t`                |
| `std::vector<T>`       | No      | Pointer + length, callback, or index  |
| Method overloading     | No      | Different C function names            |
| Templates              | No      | Explicit instantiation + C wrapper    |
| Exceptions             | No      | try/catch wrapper (see ex26)          |

The solution is always the same: **wrap with `extern "C"` functions**
that translate between C++ semantics and a flat C API.

---

## 2. The Opaque Pointer Pattern

The C++ object lives on the heap.  Rust holds a `*mut T` where `T` is
an opaque type (never instantiated on the Rust side):

```
┌─────────────────────────────────────────────────────┐
│  Rust                                               │
│                                                     │
│  struct StringStack { ptr: *mut CppStringStack }    │
│                                                     │
│  impl Drop for StringStack {                        │
│      fn drop(&mut self) {                           │
│          unsafe { cpp_stk_destroy(self.ptr) }       │
│      }                                              │
│  }                                                  │
├─────────────────────────────────────────────────────┤
│  extern "C" boundary                                │
├─────────────────────────────────────────────────────┤
│  C++                                                │
│                                                     │
│  class CppStringStack {                             │
│      std::vector<std::string> items_;  ← opaque!    │
│  };                                                 │
│                                                     │
│  CppStringStack* cpp_stk_new() {                    │
│      return new CppStringStack();                   │
│  }                                                  │
│  void cpp_stk_destroy(CppStringStack* s) {          │
│      delete s;   // calls ~CppStringStack()         │
│  }                                                  │
└─────────────────────────────────────────────────────┘
```

### Rust-side opaque type

```rust
#[repr(C)]
pub struct CppStringStack {
    _private: [u8; 0],  // zero-size, never constructed in Rust
}
```

This ensures Rust never tries to read the C++ object's layout.

---

## 3. String Passing Patterns

### 3.1 Rust → C++: `&str` as `(const char*, size_t)`

Rust `&str` is a `(pointer, length)` pair — no NUL terminator needed:

```rust
fn push(&mut self, s: &str) {
    unsafe { cpp_stk_push(self.ptr, s.as_ptr() as *const c_char, s.len()) }
}
```

C++ side:
```cpp
void push(const char *s, size_t len) {
    items_.emplace_back(s, len);  // constructs std::string from ptr+len
}
```

**Advantage over NUL-terminated `CString`:** no allocation, no scan for
interior NUL bytes, works with arbitrary binary data.

### 3.2 C++ → Rust: Caller-Provided Buffer (owned return)

For `pop()`, the caller provides a buffer:

```cpp
int32_t cpp_stk_pop(CppStringStack *s,
                     char *out_buf, size_t buf_len, size_t *out_len);
```

Rust uses a two-phase protocol:
1. First call with `buf = null, buf_len = 0` → gets `*out_len` (needed size)
2. Allocate a `Vec<u8>` of the right size
3. Second call fills the buffer

```rust
// Phase 1: get length
let mut needed = 0usize;
cpp_stk_pop(self.ptr, null_mut(), 0, &mut needed);
// Phase 2: allocate and fill
let mut buf = vec![0u8; needed + 1];
cpp_stk_pop(self.ptr, buf.as_mut_ptr() as _, buf.len(), &mut needed);
buf.truncate(needed);
String::from_utf8(buf).unwrap()
```

### 3.3 C++ → Rust: Borrowed Pointer (zero-copy peek)

`peek()` returns a pointer **directly into the C++ `std::string`**:

```cpp
int32_t cpp_stk_peek(const CppStringStack *s,
                      const char **out_ptr, size_t *out_len) {
    const std::string &ref = s->peek();
    *out_ptr = ref.c_str();      // ← pointer into C++ heap!
    *out_len = ref.size();
    return 0;
}
```

**Danger:** This pointer is invalidated by any mutation (push, pop,
destroy).  In Rust we encode this lifetime constraint with a guard type:

```rust
pub struct PeekGuard<'a> {
    ptr: *const u8,
    len: usize,
    _stack: &'a StringStack,  // borrows the stack immutably
}

impl<'a> PeekGuard<'a> {
    pub fn as_str(&self) -> &str { ... }
}
```

Because `PeekGuard` holds `&StringStack`, the Rust borrow checker
prevents calling `&mut self` methods (push, pop) while the guard exists:

```rust
let guard = stack.peek().unwrap();   // &stack borrowed
// stack.push("x");                  // COMPILE ERROR: &mut while & exists
println!("{}", guard.as_str());
drop(guard);                         // borrow released
stack.push("x");                     // now OK
```

This is one of Rust's great strengths: **encoding C++ lifetime rules
in the type system** so violations are caught at compile time.

---

## 4. Exposing `std::vector` Contents

A C++ `std::vector<T>` can be exposed in several ways:

### 4.1 Index-based Access

```cpp
int32_t cpp_stk_get(const CppStringStack *s, size_t index,
                     const char **out_ptr, size_t *out_len);
```

Simple but requires N function calls for N elements.

### 4.2 Callback-based Iteration

The C++ side iterates and calls back into Rust for each item:

```cpp
typedef void (*CppStkIterFn)(const char *str, size_t len, void *ctx);

void cpp_stk_for_each(const CppStringStack *s,
                       CppStkIterFn callback, void *ctx) {
    for (const auto &item : s->items()) {
        callback(item.c_str(), item.size(), ctx);
    }
}
```

Rust collects into `Vec<String>`:

```rust
pub fn to_vec(&self) -> Vec<String> {
    let mut result = Vec::new();
    extern "C" fn collect(s: *const c_char, len: usize, ctx: *mut c_void) {
        let vec = unsafe { &mut *(ctx as *mut Vec<String>) };
        let bytes = unsafe { std::slice::from_raw_parts(s as *const u8, len) };
        vec.push(String::from_utf8_lossy(bytes).into_owned());
    }
    unsafe { cpp_stk_for_each(self.ptr, collect, &mut result as *mut _ as _) };
    result
}
```

**Best for:** iterating over all elements, collecting, or searching.

### 4.3 Copy-out to Flat Array

For `std::vector<int>` or similar POD types:

```cpp
int32_t cpp_get_data(const CppFoo *f,
                      int32_t *out_buf, size_t buf_len, size_t *out_count);
```

Rust can then use `std::slice::from_raw_parts` on the buffer.

---

## 5. Copy / Clone Semantics

C++ copy constructors become explicit `clone()` functions:

```cpp
CppStringStack *cpp_stk_clone(const CppStringStack *s) {
    return new CppStringStack(*s);  // invokes copy constructor
}
```

Rust implements `Clone`:

```rust
impl Clone for StringStack {
    fn clone(&self) -> Self {
        let p = unsafe { cpp_stk_clone(self.ptr) };
        assert!(!p.is_null());
        Self { ptr: p }
    }
}
```

For **move-only** C++ types (deleted copy constructor), don't implement
`Clone` — the Rust wrapper is also move-only by default.

---

## 6. Factory Functions

Static factory methods like `CppStringStack::from_csv(...)` become
standalone `extern "C"` functions:

```cpp
CppStringStack *cpp_stk_from_csv(const char *csv, size_t len) {
    return new CppStringStack(CppStringStack::from_csv(csv, len));
}
```

Rust wraps it as an associated function:

```rust
impl StringStack {
    pub fn from_csv(csv: &str) -> Self {
        let p = unsafe { cpp_stk_from_csv(csv.as_ptr() as _, csv.len()) };
        assert!(!p.is_null());
        Self { ptr: p }
    }
}
```

---

## 7. Error Handling Approaches

Two common patterns for reporting C++ errors through C:

### Pattern A: Thread-Local Storage (ex26)

```
Rust calls C++  →  error code returned  →  Rust retrieves details
                                            from thread-local
```

- Pro: Simple API (just return int)
- Con: Must retrieve before next call (state is overwritten)
- Used by: OpenGL, POSIX errno, Win32 GetLastError

### Pattern B: Error Out-Parameters (this exercise)

```
Rust calls C++  →  error code returned  →  details encoded in code
```

- Pro: No hidden state, reentrant
- Con: Less detail (just an error code, no message)
- Used by: SQLite, most embedded libraries

Choose based on how much error detail you need.

---

## 8. Memory Layout Comparison

```
Rust StringStack (8 bytes on 64-bit):
┌──────────────────┐
│ ptr: *mut Opaque  │  ──→  C++ CppStringStack on heap
└──────────────────┘

C++ CppStringStack (24 bytes typical, std::vector):
┌──────────────────────────────────────┐
│ begin_: *std::string                  │  ──→ heap array of std::string
│ end_:   *std::string                  │
│ cap_:   *std::string                  │
└──────────────────────────────────────┘
        │
        ▼
┌──────────┬───────────┬──────────┐
│ string_0 │ string_1  │ string_2 │  (each std::string ~32 bytes)
└──────────┴───────────┴──────────┘
```

Rust never sees this layout — it's all behind the opaque pointer.

---

## 9. When to Use This Pattern

**Use opaque-pointer wrapping when:**
- The C++ class has constructors/destructors
- The class uses STL containers internally
- You don't need to access fields directly
- The class may change layout between versions

**Consider `#[repr(C)]` mirror structs when:**
- You need field-level access
- The type is a plain POD struct
- Performance requires avoiding function call overhead

---

## 10. Checklist

- [ ] C++ class wrapped with opaque pointer (`*mut T`, zero-size Rust type)
- [ ] `new()` / `destroy()` pair → RAII wrapper with `Drop`
- [ ] Copy constructor → `Clone` (or omit for move-only types)
- [ ] String input: `(const char*, size_t)` — no NUL requirement
- [ ] String output: caller buffer OR borrowed pointer with lifetime guard
- [ ] Vector access: callback iteration OR index OR copy-out
- [ ] All `extern "C"` wrappers have try/catch (see ex26)
- [ ] Factory methods → associated functions on Rust wrapper
- [ ] `Send` implemented only if the C++ type is thread-safe
- [ ] OOM from `new` handled (null check or abort)
