//! # Exercise 13: `#[repr(C)]` Tagged Unions
//!
//! **Concept:** Rust enums with data (sum types) have no direct C
//! equivalent.  The standard FFI approach is a **tagged union**:
//!
//! ```c
//! struct TaggedValue {
//!     uint32_t tag;     // discriminant
//!     union {
//!         int64_t  integer;
//!         double   floating;
//!         char    *string;
//!     } data;
//! };
//! ```
//!
//! In Rust this translates to a `#[repr(C)]` struct containing a
//! `#[repr(u32)]` tag enum and a `#[repr(C)] union`.
//!
//! ## Your task
//!
//! 1. Define the tag, union, and tagged-union struct.
//! 2. Write constructors for each variant.
//! 3. Write a safe inspection function.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex13
//! ```

use std::ffi::{c_char, CStr, CString};

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Define a tag enum with these variants:
//   TAG_INTEGER  = 0
//   TAG_FLOAT    = 1
//   TAG_STRING   = 2
//
// Use `#[repr(u32)]` so the discriminant is a C-compatible u32.

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueTag {
    Integer = 0,
    Float = 1,
    String = 2,
}

// ── Pre-provided (needed by tests) ─────────────────────────────
//
// A `#[repr(C)]` union with the possible payloads.
// Rust unions require `unsafe` to read fields, and all fields
// must be `Copy` (raw pointers are `Copy`).

#[repr(C)]
pub union ValueData {
    pub integer: i64,
    pub floating: f64,
    pub string: *mut c_char,
}

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Define the tagged union struct:
//
//   #[repr(C)]
//   pub struct TaggedValue {
//       pub tag: ValueTag,
//       pub data: ValueData,
//   }

#[repr(C)]
pub struct TaggedValue {
    pub tag: ValueTag,
    pub data: ValueData,
}

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Implement constructors.

/// Create an integer variant.
#[no_mangle]
pub extern "C" fn tagged_value_int(val: i64) -> TaggedValue {
    todo!("Return TaggedValue with tag=Integer and data.integer=val")
}

/// Create a float variant.
#[no_mangle]
pub extern "C" fn tagged_value_float(val: f64) -> TaggedValue {
    todo!("Return TaggedValue with tag=Float and data.floating=val")
}

/// Create a string variant.  The string is **cloned** into a new
/// heap allocation — caller must eventually free the TaggedValue
/// with `tagged_value_free`.
///
/// # Safety
/// `s` must be a valid C string.
#[no_mangle]
pub unsafe extern "C" fn tagged_value_string(s: *const c_char) -> TaggedValue {
    todo!("Clone the string via CString, store data.string = into_raw()")
}

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Free a TaggedValue.  Only the String variant needs cleanup
// (freeing the heap-allocated CString).

/// # Safety
/// `tv` must be a valid TaggedValue.  If it's a String variant,
/// the string pointer must have been allocated by `tagged_value_string`.
#[no_mangle]
pub unsafe extern "C" fn tagged_value_free(tv: *mut TaggedValue) {
    todo!("If tag == String, CString::from_raw(data.string)")
}

// ── TODO 6 ─────────────────────────────────────────────────────
//
// Describe a TaggedValue as a Rust String (for debugging).
// e.g. "Integer(42)", "Float(3.14)", "String(hello)"

pub unsafe fn tagged_value_describe(tv: &TaggedValue) -> String {
    todo!("Match on tag, read the appropriate union field, format as string")
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ex13_integer() {
        let tv = tagged_value_int(42);
        assert_eq!(tv.tag, ValueTag::Integer);
        assert_eq!(unsafe { tv.data.integer }, 42);
    }

    #[test]
    fn test_ex13_float() {
        let tv = tagged_value_float(3.14);
        assert_eq!(tv.tag, ValueTag::Float);
        assert!((unsafe { tv.data.floating } - 3.14).abs() < 1e-10);
    }

    #[test]
    fn test_ex13_string() {
        let s = CString::new("hello").unwrap();
        let mut tv = unsafe { tagged_value_string(s.as_ptr()) };
        assert_eq!(tv.tag, ValueTag::String);

        let result = unsafe { CStr::from_ptr(tv.data.string) }
            .to_str()
            .unwrap();
        assert_eq!(result, "hello");

        unsafe { tagged_value_free(&mut tv) };
    }

    #[test]
    fn test_ex13_describe() {
        let tv_int = tagged_value_int(-7);
        assert_eq!(unsafe { tagged_value_describe(&tv_int) }, "Integer(-7)");

        let tv_float = tagged_value_float(2.5);
        assert_eq!(unsafe { tagged_value_describe(&tv_float) }, "Float(2.5)");

        let s = CString::new("world").unwrap();
        let mut tv_str = unsafe { tagged_value_string(s.as_ptr()) };
        assert_eq!(
            unsafe { tagged_value_describe(&tv_str) },
            "String(world)"
        );
        unsafe { tagged_value_free(&mut tv_str) };
    }
}
