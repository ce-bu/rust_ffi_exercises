# Passing and Receiving Strings Across Rust ↔ C FFI

## The Core Problem

Rust strings (`&str`, `String`) and C strings (`char *`) are fundamentally different:

| Property           | Rust `&str` / `String`     | C `char *`                  |
|--------------------|----------------------------|-----------------------------|
| Termination        | Length-prefixed, no `\0`   | Null-terminated (`\0`)     |
| Encoding guarantee | Always valid UTF-8         | Arbitrary bytes             |
| Interior NULs      | Allowed                    | Not allowed (ends the string) |
| Ownership          | Clear (borrow vs owned)    | Convention-dependent        |

Bridging the two worlds requires explicit conversion. Rust provides two types for this:

| Type      | Owned? | Rust analogy | Purpose                               |
|-----------|--------|-------------|---------------------------------------|
| `CStr`    | No     | `&str`      | **Borrow** an existing C string       |
| `CString` | Yes    | `String`    | **Create** a null-terminated string to hand to C |

---

## Rust → C: Sending a String to a C Function

### The correct pattern

```rust
use std::ffi::{CString, c_char};

extern "C" {
    fn c_takes_string(s: *const c_char);
}

fn send_to_c(input: &str) {
    // 1. Create a CString (adds \0, checks for interior NULs)
    let c_string = CString::new(input).expect("interior NUL in string");

    // 2. Borrow a pointer — valid as long as `c_string` is alive
    let ptr: *const c_char = c_string.as_ptr();

    // 3. Call the C function
    unsafe { c_takes_string(ptr); }

    // 4. c_string drops here → memory freed automatically
}
```

### ❌ Common Error 1: Dangling pointer from temporary `CString`

```rust
// WRONG — CString is dropped at the end of the expression!
let ptr = CString::new("hello").unwrap().as_ptr();
//        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
//        temporary CString dropped here — ptr is dangling!
unsafe { c_takes_string(ptr); } // 💥 use-after-free
```

**Fix:** Bind the `CString` to a variable so it lives long enough:
```rust
let c_string = CString::new("hello").unwrap();
let ptr = c_string.as_ptr(); // borrows c_string — valid while c_string lives
unsafe { c_takes_string(ptr); }
```

### ❌ Common Error 2: Passing `&str` directly without NUL termination

```rust
let s = "hello";
// WRONG — &str has no trailing \0. C will read past the end!
unsafe { c_takes_string(s.as_ptr() as *const c_char); } // 💥 UB
```

Rust's `&str` is **not** null-terminated. The C function will scan past the
string looking for `\0` and read garbage memory (or segfault).

**Fix:** Always go through `CString` (or use `c"..."` literals, see below).

### ✅ Zero-cost alternative: C string literals (Rust 1.77+)

If the string is a compile-time constant, you can avoid allocation entirely:

```rust
let ptr = c"hello".as_ptr();       // No allocation, \0 baked in at compile time
unsafe { c_takes_string(ptr); }
```

Or with `CStr::from_bytes_with_nul`:
```rust
use std::ffi::CStr;
let cs = CStr::from_bytes_with_nul(b"hello\0").unwrap();
unsafe { c_takes_string(cs.as_ptr()); }
```

---

## C → Rust: Receiving a String from C

### Borrowing (most common — no allocation)

```rust
use std::ffi::{CStr, c_char};

unsafe fn receive_from_c(ptr: *const c_char) -> &'static str {
    assert!(!ptr.is_null(), "null pointer");
    let c_str: &CStr = unsafe { CStr::from_ptr(ptr) };  // borrows, scans for \0
    c_str.to_str().expect("invalid UTF-8")               // → &str
}
```

⚠️ The returned `&str` borrows memory owned by C. It's only valid as long as
the C side doesn't free or modify the buffer.

### Taking ownership (when Rust must free the string)

```rust
use std::ffi::{CStr, CString, c_char};

unsafe fn take_from_c(ptr: *mut c_char) -> String {
    assert!(!ptr.is_null());
    let c_string = unsafe { CString::from_raw(ptr) }; // takes ownership
    c_string.into_string().expect("invalid UTF-8")
    // CString dropped → memory freed by Rust's allocator
}
```

⚠️ Only use `CString::from_raw` on pointers that were allocated by Rust's
allocator (`CString::into_raw`). Never use it on pointers from `malloc()` or
C libraries — that's UB (mismatched allocators).

---

## Allocating a Rust Buffer for C to Fill

A very common FFI pattern: you allocate a buffer on the Rust side, pass it to a
C function that writes a string into it, then convert the result. A plain `Vec`
is the right tool — **not** `ManuallyDrop`.

### Typical C API

```c
// Returns number of bytes written (excluding \0), or -1 on error.
// If buf is NULL or buf_len is 0, returns the required buffer size.
int get_device_name(char *buf, size_t buf_len);
```

### ✅ Correct pattern: `Vec<u8>` as the buffer

```rust
use std::ffi::{CStr, c_char};

extern "C" {
    fn get_device_name(buf: *mut c_char, buf_len: usize) -> i32;
}

fn device_name() -> Result<String, &'static str> {
    // Step 1: Query required size (common convention: pass NULL/0)
    let required = unsafe { get_device_name(std::ptr::null_mut(), 0) };
    if required < 0 { return Err("query failed"); }
    let buf_len = (required as usize) + 1; // +1 for \0

    // Step 2: Allocate buffer
    let mut buf: Vec<u8> = vec![0u8; buf_len];

    // Step 3: Let C fill it
    let written = unsafe {
        get_device_name(buf.as_mut_ptr() as *mut c_char, buf_len)
    };
    if written < 0 { return Err("write failed"); }

    // Step 4: Convert to Rust string
    // buf is still owned by Rust — it will be freed when it drops.
    let c_str = CStr::from_bytes_with_nul(&buf[..=(written as usize)])
        .map_err(|_| "invalid C string")?;
    c_str.to_str().map(|s| s.to_owned()).map_err(|_| "invalid UTF-8")
}
```

### Why `ManuallyDrop` is NOT needed here

`ManuallyDrop` prevents Rust from running the destructor. You would only need it
if **C takes ownership** of the buffer and will free it later. In the "C fills
our buffer" pattern, Rust still owns the `Vec` and should drop it normally:

| Scenario                          | Use `Vec`?       | Use `ManuallyDrop<Vec>`? |
|-----------------------------------|------------------|--------------------------|
| C fills Rust's buffer, Rust reads | ✅ Yes (normal)  | ❌ No                    |
| C takes ownership of the buffer   | —                | ✅ Yes (prevent drop)    |

### When you DO need `ManuallyDrop` (rare)

If the C function takes ownership of the pointer and will call `free()` on it
(or return it later for you to free with a C-side function), you must prevent
Rust from also freeing it:

```rust
use std::mem::ManuallyDrop;

extern "C" {
    // C takes ownership of buf and will free() it internally.
    fn c_takes_ownership(buf: *mut u8, len: usize);
}

fn hand_off_to_c() {
    let mut buf = ManuallyDrop::new(vec![0u8; 256]);

    unsafe {
        c_takes_ownership(buf.as_mut_ptr(), buf.len());
    }
    // Vec's drop is NOT called — C now owns the memory.
    // ⚠️ This only works if C uses the same allocator (rare!).
    // In practice, prefer allocating from C's side if C will free it.
}
```

### ❌ Common Error: Using `ManuallyDrop` "for safety" when it isn't needed

```rust
// WRONG — unnecessary ManuallyDrop causes a memory leak!
let mut buf = ManuallyDrop::new(vec![0u8; 256]);
unsafe { get_device_name(buf.as_mut_ptr() as *mut c_char, buf.len()); }
let result = /* ... convert buf ... */;
// buf is never freed → LEAK
```

If Rust is the allocator AND the owner, just use a plain `Vec` and let it drop
normally after you're done reading from it.

### Alternative: stack buffer for small strings

If you know an upper bound on the string length, avoid heap allocation entirely:

```rust
let mut buf = [0u8; 256]; // stack-allocated
let written = unsafe {
    get_device_name(buf.as_mut_ptr() as *mut c_char, buf.len())
};
if written >= 0 {
    let name = CStr::from_bytes_with_nul(&buf[..=(written as usize)])
        .unwrap()
        .to_str()
        .unwrap();
    println!("Device: {name}");
}
// buf lives on the stack — no heap, no leak possible
```

---

## Returning Strings to C from Rust

### The pattern: `CString::into_raw` + matching free function

```rust
use std::ffi::{CString, c_char};

#[no_mangle]
pub extern "C" fn create_greeting(name: *const c_char) -> *mut c_char {
    // ... validate, build string ...
    let greeting = format!("Hello, {}!", name_str);
    match CString::new(greeting) {
        Ok(cs) => cs.into_raw(),    // caller must free this!
        Err(_) => std::ptr::null_mut(),
    }
}

/// # Safety: `s` must have come from `CString::into_raw`, or be null.
#[no_mangle]
pub unsafe extern "C" fn free_greeting(s: *mut c_char) {
    if !s.is_null() {
        unsafe { drop(CString::from_raw(s)); }
    }
}
```

### ❌ Common Error 3: Memory leak — forgetting to free `into_raw` strings

```rust
let ptr = CString::new("hello").unwrap().into_raw();
// ptr now owns heap memory
// If nobody calls CString::from_raw(ptr), this is a leak!
```

**Rule:** Every `into_raw()` must be paired with exactly one `from_raw()`.

### ❌ Common Error 4: Freeing with the wrong allocator

```c
// C side — WRONG!
char *s = create_greeting("world");
free(s);  // 💥 Rust allocated this, C's free() doesn't match
```

Always provide a Rust-side free function (`free_greeting` above) and document
that the caller **must** use it.

---

## Writing to Caller-Provided Buffers

When C provides the buffer and Rust fills it in:

```rust
use std::ffi::{CStr, c_char};

#[no_mangle]
pub unsafe extern "C" fn to_uppercase(
    input: *const c_char,
    buf: *mut c_char,
    buf_len: usize,
) -> isize {
    if input.is_null() || buf.is_null() { return -1; }

    let input_str = unsafe { CStr::from_ptr(input) }.to_str().unwrap();
    let upper = input_str.to_uppercase();

    // Need room for the string + null terminator
    if upper.len() + 1 > buf_len { return -1; }

    unsafe {
        std::ptr::copy_nonoverlapping(upper.as_ptr(), buf as *mut u8, upper.len());
        *buf.add(upper.len()) = 0; // null terminator
    }
    upper.len() as isize
}
```

Key points:
- Always account for the null terminator (`len + 1`).
- Return an error code if the buffer is too small — never write past it.

---

## Handling Null Pointers

C commonly uses `NULL` to mean "no value". Always check before converting:

```rust
pub unsafe extern "C" fn safe_length(s: *const c_char) -> usize {
    if s.is_null() { return 0; }
    unsafe { CStr::from_ptr(s) }.to_bytes().len()
}
```

`CStr::from_ptr(null)` is **instant UB** — it will dereference the null pointer.

---

## Handling Non-UTF-8 Strings

Not all C strings are valid UTF-8. When you can't guarantee UTF-8:

```rust
use std::ffi::CStr;

let c_str = unsafe { CStr::from_ptr(ptr) };

// Option A: lossy conversion (replaces invalid bytes with U+FFFD)
let s: String = c_str.to_string_lossy().into_owned();

// Option B: work with raw bytes
let bytes: &[u8] = c_str.to_bytes();

// Option C: fail explicitly
let s: &str = c_str.to_str().map_err(|e| /* handle error */)?;
```

---

## `Path` → `*const c_char` (Linux/macOS)

Filesystem paths may not be UTF-8. On Unix, use `OsStrExt`:

```rust
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

fn path_to_cstring(path: &Path) -> Result<CString, std::ffi::NulError> {
    CString::new(path.as_os_str().as_bytes())
}
```

---

## Quick Reference: Common Conversions

| From                  | To                    | How                                              |
|-----------------------|-----------------------|--------------------------------------------------|
| `&str`                | `*const c_char`       | `CString::new(s)?.as_ptr()`                      |
| `String`              | `*const c_char`       | `CString::new(s)?.as_ptr()`                      |
| `String`              | `*mut c_char` (owned) | `CString::new(s)?.into_raw()`                    |
| `*const c_char`       | `&str`                | `CStr::from_ptr(p).to_str()?`                    |
| `*const c_char`       | `String`              | `CStr::from_ptr(p).to_str()?.to_owned()`         |
| `*mut c_char`         | `CString` (reclaim)   | `CString::from_raw(p)`                           |
| Literal               | `*const c_char`       | `c"hello".as_ptr()`                              |
| `&Path`               | `CString`             | `CString::new(path.as_os_str().as_bytes())?`     |

---

## Summary of Common Mistakes

| # | Mistake                                    | Consequence         | Fix                                     |
|---|--------------------------------------------|---------------------|-----------------------------------------|
| 1 | Passing `&str.as_ptr()` to C               | Read past buffer, UB| Use `CString` or `c"..."` literal       |
| 2 | Dangling pointer from temporary `CString`  | Use-after-free      | Bind `CString` to a variable            |
| 3 | Not freeing `into_raw()` result            | Memory leak         | Always pair with `from_raw()`           |
| 4 | Freeing Rust-allocated string with C `free`| Heap corruption, UB | Provide a Rust-side free function       |
| 5 | Calling `CStr::from_ptr` on null           | Null deref, UB      | Check for null first                    |
| 6 | Ignoring interior NULs                     | Truncated string     | Handle `CString::new()` `NulError`      |
| 7 | Assuming C strings are UTF-8               | Panic on `.to_str()` | Use `.to_string_lossy()` or handle error|
| 8 | Writing past caller's buffer               | Buffer overflow, UB  | Always check `len + 1 <= buf_len`       |
