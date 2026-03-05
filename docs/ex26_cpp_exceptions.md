# C++ Exceptions Across the FFI Boundary

> **Exercise 26** — companion material  
> Applies to: any language calling C++ through `extern "C"` (Rust, C, Python, etc.)

---

## 1. The Problem

A C++ `throw` triggers **stack unwinding** — destructors run, frames are
popped, and control jumps to the matching `catch`.  The Itanium ABI
defines how this works between *C++ frames*.

But an `extern "C"` function has no C++ personality — it does not
participate in C++ exception handling.  If a C++ exception propagates
into a frame that isn't C++ (a Rust frame, a C frame, a Python
extension), behaviour is **undefined**:

```
 C++ frame (throws)   ──→   extern "C" frame   ──→   Rust frame
         │                          │
         └── unwinding enters here: no personality, UB!
```

In practice this leads to:
- Immediate `std::terminate()` (GCC/Clang with `-fno-exceptions`)
- Silent corruption of Rust stack frames
- Segmentation fault during destructor execution
- "Works on my machine" but fails under optimisation

**Rule: every `extern "C"` function that calls C++ code MUST catch
all exceptions before returning.**

---

## 2. The C++ Exception Model (30-second primer)

```
throw expr;              // creates exception object, begins unwinding
                         //
  ┌─ frame N destructor  // stack unwinding: dtors called in reverse
  ├─ frame N-1 destructor
  ├─ ...
  └─ catch (Type& e)     // first matching catch handler stops unwinding
       { ... }
                         
  catch (...)            // catch-all: matches ANY thrown type
```

Key points:
- Exception objects are allocated on a **side channel** (often
  `__cxa_allocate_exception`), not on the stack.
- Unwinding uses the **unwind table** (`.eh_frame`, `.gcc_except_table`)
  to know which destructors to call.
- A frame without unwind info is a **hard stop**.

---

## 3. The Wrapper Pattern

The canonical solution is a **try/catch wrapper** on the C++ side:

```
┌────────────────────────────────────────────┐
│  Rust (safe)                               │
│   let rc = unsafe { cpp_ex_divide(a,b,&r) }│
│   if rc != 0 { return Err(get_error()) }   │
├────────────────────────────────────────────┤
│  extern "C"  cpp_ex_divide(a, b, *out)     │
│    clear_exc();                             │
│    try {                                    │
│        *out = internal::divide(a, b);       │
│        return OK;                           │  ← C++ try/catch
│    } catch (const std::domain_error& e) {   │     guards the boundary
│        store_exc(DOMAIN, e.what(), ...);    │
│        return ERR_DOMAIN;                   │
│    } catch (const std::exception& e) { ... }│
│    catch (...) { ... }                      │
├────────────────────────────────────────────┤
│  C++ internal::divide(a, b)                │
│    if (b == 0) throw std::domain_error(...) │
└────────────────────────────────────────────┘
```

### 3.1 The Error Code

Return an `int32_t`:
- `0` = success
- Negative values = error categories

```c
#define CPP_EX_OK            0
#define CPP_EX_ERR_DOMAIN   -1  // std::domain_error
#define CPP_EX_ERR_INVALID  -2  // std::invalid_argument
#define CPP_EX_ERR_CUSTOM   -3  // application-specific
#define CPP_EX_ERR_UNKNOWN  -99 // catch(...)
```

### 3.2 Error Info Storage

Store the exception details in a **thread-local** struct:

```cpp
struct ExceptionInfo {
    char    message[512];
    char    type_name[128];
    int32_t code;
    bool    active;
};

static thread_local ExceptionInfo tl_exc = {};
```

Expose retrieval through another `extern "C"` function:

```cpp
int32_t cpp_ex_get_error(char *out_msg, size_t msg_len,
                         char *out_type, size_t type_len,
                         int32_t *out_code);
```

This follows the same philosophy as:
- `errno` + `strerror()` in POSIX
- `GetLastError()` + `FormatMessage()` in Win32

### 3.3 Catch Order Matters

C++ tries `catch` blocks **top-to-bottom** and picks the **first match**.
Always order from most-specific to least-specific:

```cpp
try { ... }
catch (const ProcessingError &e) { ... }   // most specific (custom)
catch (const std::invalid_argument &e) { ... }
catch (const std::domain_error &e) { ... }
catch (const std::exception &e) { ... }    // base class fallback
catch (...) { ... }                         // MANDATORY catch-all
```

### 3.4 The `catch(...)` Requirement

C++ can throw **anything** — not just `std::exception` subclasses:

```cpp
throw 42;                    // int
throw "oops";                // const char*
throw MyPODStruct{1, 2, 3}; // arbitrary type
```

If you only catch `std::exception&`, these slip through.
**Always end with `catch(...)`.**

---

## 4. Rust Side: From Return Code to `Result`

```rust
#[derive(Debug, Clone)]
pub struct CppException {
    pub message: String,
    pub type_name: String,
    pub code: i32,
}

impl std::fmt::Display for CppException { ... }
impl std::error::Error for CppException {}

fn check(rc: i32) -> Result<(), CppException> {
    if rc == 0 { Ok(()) }
    else { Err(get_last_cpp_error(rc)) }
}

pub fn divide(a: f64, b: f64) -> Result<f64, CppException> {
    let mut result = 0.0;
    let rc = unsafe { cpp_ex_divide(a, b, &mut result) };
    check(rc)?;
    Ok(result)
}
```

The `?` operator gives clean, idiomatic error propagation.

---

## 5. Callbacks: Rust Closures Called from C++

When C++ calls a Rust callback, two boundaries are crossed:

```
Rust  ──→  extern "C" C++  ──→  callback (Rust)  ──→  back to C++
```

**Dangers:**
1. If the Rust callback **panics**, the unwind crosses back into C++: **UB**.
2. If C++ code around the callback **throws**, it must still be caught.

**Solutions:**
- The Rust callback returns an error code (never panics).
- Alternatively, use `std::panic::catch_unwind()` inside the callback.
- The C++ wrapper still has try/catch around the whole loop.

```cpp
for (size_t i = 0; i < len; ++i) {
    int32_t rc = callback(input[i], &output[i], ctx);
    if (rc != 0) {
        store_exc(rc, "callback error", "callback");
        return rc;           // propagate callback error
    }
}
```

On the Rust side, the closure returns `Result<T, i32>`:

```rust
pub fn map_array<F>(input: &[f64], f: F) -> Result<Vec<f64>, CppException>
where
    F: FnMut(f64) -> Result<f64, i32>,
{
    extern "C" fn trampoline<F2>(val: f64, out: *mut f64, ctx: *mut c_void) -> i32
    where F2: FnMut(f64) -> Result<f64, i32>
    {
        let closure = unsafe { &mut *(ctx as *mut F2) };
        match closure(val) {
            Ok(v)    => { unsafe { *out = v }; 0 }
            Err(code) => code,
        }
    }
    // ...
}
```

---

## 6. Thread Safety

The thread-local pattern (`thread_local ExceptionInfo`) is inherently
thread-safe because each thread has its own copy.  No mutex is needed.

However, beware of **single-threaded C++ code** called from multiple
Rust threads.  If the C++ library uses global state internally, you
need synchronisation on the Rust side (e.g. a `Mutex<()>` guard).

---

## 7. `extern "C-unwind"` — The Nightly Escape Hatch

Rust nightly provides `extern "C-unwind"` which *allows* foreign
exceptions to propagate through Rust frames:

```rust
extern "C-unwind" {
    fn might_throw();
}
```

This is being stabilised incrementally (see [RFC 2945]).  Even with
`C-unwind`, you still need to handle the exception *somewhere*.  The
try/catch wrapper is still the recommended pattern for production code.

[RFC 2945]: https://rust-lang.github.io/rfcs/2945-c-unwind-abi.html

---

## 8. Comparison with Rust Panics

| Aspect            | C++ exception               | Rust panic                         |
|-------------------|-----------------------------|------------------------------------|
| Mechanism         | `throw` + LSDA tables      | `panic!()` + `eh_personality`      |
| Catch             | `catch (Type&)`             | `catch_unwind()`                   |
| Across FFI?       | UB → must wrap              | UB → must `catch_unwind()`         |
| Type              | Any C++ type                | `Box<dyn Any + Send>`              |
| Destructors       | C++ dtors via unwinding     | Rust `Drop` via unwinding          |
| Abort option      | `std::terminate()`          | `panic = "abort"` in Cargo.toml    |

Both are UB across an `extern "C"` boundary.  The solution is the same:
catch before the boundary.

---

## 9. Performance Notes

- The **zero-cost exceptions** model means try/catch has nearly zero
  overhead on the happy path (no branches, no checks).
- The *throw* path is expensive: ~1–10 µs for unwinding, allocation,
  RTTI comparison.
- Our wrapper adds one function call + memcpy for the error message.
- For hot paths, prefer returning error codes *from C++ itself* and
  reserve exceptions for truly exceptional situations.

---

## 10. Checklist

- [ ] Every `extern "C"` function has `try { ... } catch (...) { ... }`
- [ ] `catch(...)` is always the last handler (catch-all)
- [ ] Catch order: specific → general → catch-all
- [ ] Exception info stored in thread-local (or out-parameter)
- [ ] Rust side checks return code and retrieves error info
- [ ] Callbacks never panic (or use `catch_unwind`)
- [ ] No `throw` specifications on `extern "C"` functions (use `noexcept` if needed)
- [ ] Thread-local storage is appropriate for the threading model
