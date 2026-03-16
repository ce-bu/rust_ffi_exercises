//! # Exercise 23: `transmute` and Type Reinterpretation in FFI
//!
//! **Concept:** `std::mem::transmute` reinterprets the bits of one
//! type as another type.  It is *the most dangerous function in Rust*
//! — almost any misuse is instant undefined behavior.  Yet there are
//! a few FFI scenarios where it is the **only stable option**:
//!
//! | Use case | Why `transmute`? |
//! |----------|-----------------|
//! | `*mut c_void` ↔ function pointer | `as` casts between data and fn ptrs are not allowed |
//! | Nullable callback (`Option<extern "C" fn()>`) | Layout-guaranteed but sometimes you receive raw bits |
//! | Integer → `#[repr(C)]` enum | Only way without a match (but `TryFrom` is safer!) |
//! | Byte buffer → `#[repr(C)]` struct | Network/file deserialization (prefer `ptr::read` instead) |
//!
//! Modern Rust has **safer alternatives** for most cases.  This
//! exercise teaches both the `transmute` way and the safe way, so
//! you know *when* `transmute` is truly needed and *when* to avoid it.
//!
//! ## Your task
//!
//! Implement all TODO sections.  For each part, you'll see the
//! `transmute` approach AND the preferred safe alternative.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex23
//! ```

use std::ffi::c_void;
use std::mem;

// ══════════════════════════════════════════════════════════════
// Part A — Function pointer ↔ *mut c_void  (transmute REQUIRED)
// ══════════════════════════════════════════════════════════════
//
// In C, `dlsym()` returns `void*` which may point to a function.
// The C standard technically forbids data↔function pointer casts,
// but POSIX requires it.  Rust's `as` follows the C standard and
// forbids it:
//
//     let fp: extern "C" fn() = ptr as extern "C" fn();  // ERROR
//
// `transmute` is the only stable way to perform this cast.

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Implement `fn_ptr_to_void` and `void_to_fn_ptr`.
//
// `fn_ptr_to_void`:
//   Takes an `extern "C" fn(i32) -> i32` and returns `*mut c_void`.
//   Use `transmute`.
//
// `void_to_fn_ptr`:
//   Takes a `*mut c_void` and returns `extern "C" fn(i32) -> i32`.
//   Use `transmute`.
//
// Safety: The caller must guarantee the pointer actually points to
// a function with the correct signature.
//
// Hint:
//   unsafe { mem::transmute(fp) }

/// Convert a function pointer to a void pointer.
///
/// # Safety
/// The resulting pointer must only be cast back to the original
/// function signature.
pub unsafe fn fn_ptr_to_void(fp: extern "C" fn(i32) -> i32) -> *mut c_void {
    todo!("transmute fp to *mut c_void")
}

/// Convert a void pointer back to a function pointer.
///
/// # Safety
/// `ptr` must have originated from a function with signature
/// `extern "C" fn(i32) -> i32`.
pub unsafe fn void_to_fn_ptr(ptr: *mut c_void) -> extern "C" fn(i32) -> i32 {
    todo!("transmute ptr back to extern \"C\" fn(i32) -> i32")
}

// ══════════════════════════════════════════════════════════════
// Part B — Nullable function pointers (Option<fn>)
// ══════════════════════════════════════════════════════════════
//
// The Rust compiler guarantees that `Option<extern "C" fn(...)>`
// has the same layout as a raw pointer — `None` is null, `Some(fp)`
// is the function address.  This is FFI-safe and extremely useful
// for optional callbacks.
//
// Most of the time you do NOT need transmute for this — just
// declare the FFI function with `Option<extern "C" fn(...)>`.
// But sometimes you receive a raw `usize` or `*mut c_void` from
// C and need to check if it's a valid function pointer.

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Implement `raw_to_optional_fn` — convert a raw `usize` (which may
// be 0/null) to `Option<extern "C" fn(i32) -> i32>`.
//
// If `raw == 0`, return `None`.
// Otherwise, transmute `raw` to `extern "C" fn(i32) -> i32` and
// wrap in `Some`.
//
// Also implement the safe alternative `invoke_optional_callback`
// which uses `Option<extern "C" fn(...)>` directly — no transmute.

/// Convert a raw address to an optional function pointer.
///
/// # Safety
/// If `raw != 0`, it must be a valid function with the correct
/// signature.
pub unsafe fn raw_to_optional_fn(raw: usize) -> Option<extern "C" fn(i32) -> i32> {
    todo!("if raw == 0 {{ None }} else {{ Some(transmute(raw)) }}")
}

/// Invoke an optional callback — the SAFE way.
///
/// `Option<extern "C" fn(...)>` is FFI-safe and handles null
/// natively.  No transmute needed.
pub fn invoke_optional_callback(cb: Option<extern "C" fn(i32) -> i32>, arg: i32) -> Option<i32> {
    todo!("cb.map(|f| f(arg))")
}

// ══════════════════════════════════════════════════════════════
// Part C — Integer → #[repr(C)] enum  (transmute vs TryFrom)
// ══════════════════════════════════════════════════════════════
//
// C APIs return status codes as integers.  We want to convert them
// to Rust enums.
//
// `transmute` WORKS but is **dangerous**: if the integer doesn't
// match any variant, it's instant UB (the enum has an invalid
// discriminant).
//
// The SAFE alternative is `TryFrom<i32>` with a match.

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok = 0,
    NotFound = -1,
    PermissionDenied = -2,
    IoError = -3,
}

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Implement two conversion functions:
//
// (a) `status_from_int_transmute` — uses `transmute`.
//     This is DANGEROUS: any value outside {0, -1, -2, -3} is UB.
//     Guard with an assertion or match BEFORE transmuting.
//
// (b) `status_from_int_safe` — uses a match and returns
//     `Result<Status, i32>`.  This is the PREFERRED approach.
//
// The exercise teaches both so you understand why `TryFrom` is
// almost always better.

/// Convert integer to Status using transmute.
///
/// # Safety
/// `code` MUST be one of: 0, -1, -2, -3.  Any other value is **UB**.
pub unsafe fn status_from_int_transmute(code: i32) -> Status {
    todo!("Assert code is valid, then mem::transmute(code)")
}

/// Convert integer to Status safely using a match.
/// Returns `Err(code)` for unrecognised values.
pub fn status_from_int_safe(code: i32) -> Result<Status, i32> {
    todo!("match code {{ 0 => Ok(Status::Ok), -1 => ..., _ => Err(code) }}")
}

// Also implement TryFrom for idiomatic usage.
impl TryFrom<i32> for Status {
    type Error = i32;
    fn try_from(code: i32) -> Result<Self, Self::Error> {
        todo!("Delegate to status_from_int_safe")
    }
}

// ══════════════════════════════════════════════════════════════
// Part D — Byte buffer → #[repr(C)] struct (transmute vs ptr::read)
// ══════════════════════════════════════════════════════════════
//
// Network protocols and binary file formats often require
// interpreting a byte buffer as a C struct.  transmute can do
// this but has strict requirements:
//   - Exact size match
//   - Correct alignment
//   - All bit patterns must be valid for the target type
//
// `ptr::read` / `ptr::read_unaligned` are usually better because
// they don't require the source to be a perfectly-typed reference.

/// A network packet header.  All fields are integers, so every
/// bit pattern is valid — this is one of the FEW cases where
/// transmute from bytes is not *immediately* UB.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PacketHeader {
    pub magic: u16,
    pub version: u8,
    pub msg_type: u8,
    pub length: u32,
}

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Implement two deserialization functions:
//
// (a) `packet_from_bytes_transmute` — uses transmute.
//     Requires: `bytes.len() == size_of::<PacketHeader>()`.
//     DANGER: If the struct had a bool or enum field, invalid
//     bit patterns would be UB.
//
// (b) `packet_from_bytes_safe` — uses `ptr::read_unaligned`.
//     Safer: works with unaligned data, and clearly communicates
//     intent.
//
// Both should return `None` if the buffer is too small.

/// Deserialize a PacketHeader from raw bytes using transmute.
///
/// # Safety
/// The byte slice must contain a valid PacketHeader representation.
/// For this struct (all integer fields) any bit pattern is valid,
/// but this would be UB if the struct contained a bool or enum.
pub unsafe fn packet_from_bytes_transmute(bytes: &[u8]) -> Option<PacketHeader> {
    todo!(
        "Check len == size_of, then transmute a reference to the \
         first size_of bytes. Hint: *(bytes.as_ptr() as *const _) \
         via transmute or ptr::read"
    )
}

/// Deserialize a PacketHeader from raw bytes using ptr::read_unaligned.
/// This is the PREFERRED approach.
pub fn packet_from_bytes_safe(bytes: &[u8]) -> Option<PacketHeader> {
    todo!(
        "Check len >= size_of, then \
         ptr::read_unaligned(bytes.as_ptr() as *const PacketHeader)"
    )
}

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Implement serialization: PacketHeader → bytes.
//
// Use `std::slice::from_raw_parts` to view the struct as bytes.
// This is a common FFI pattern for sending structs over the wire.

/// Serialize a PacketHeader to a byte vector.
pub fn packet_to_bytes(header: &PacketHeader) -> Vec<u8> {
    todo!(
        "Cast header pointer to *const u8, use \
         slice::from_raw_parts(ptr, size_of::<PacketHeader>()), \
         convert to Vec"
    )
}

// ══════════════════════════════════════════════════════════════
// Part E — When NOT to use transmute (safe alternatives)
// ══════════════════════════════════════════════════════════════
//
// These functions demonstrate casts that look like they need
// transmute but actually don't.

// ── TODO 6 ─────────────────────────────────────────────────────
//
// Implement these WITHOUT transmute — use `as` casts, `From`,
// or safe pointer operations instead.

/// Convert a `*mut T` to `*mut c_void`.  (Just use `as`!)
pub fn ptr_to_void<T>(p: *mut T) -> *mut c_void {
    todo!("p as *mut c_void")
}

/// Convert a `*mut c_void` back to `*mut T`.  (Just use `as`!)
pub fn void_to_ptr<T>(p: *mut c_void) -> *mut T {
    todo!("p as *mut T")
}

/// Convert `u32` to `f32` preserving bit pattern.
/// Use `f32::from_bits` instead of transmute.
pub fn u32_to_f32_bits(bits: u32) -> f32 {
    todo!("f32::from_bits(bits)")
}

/// Convert `f32` to `u32` preserving bit pattern.
/// Use `f32::to_bits` instead of transmute.
pub fn f32_to_u32_bits(val: f32) -> u32 {
    todo!("val.to_bits()")
}

/// Convert between integer types.
/// Use `as` casts or `From`/`TryFrom` — never transmute!
/// (Sizes may differ → transmute would be UB.)
pub fn i32_to_u32(x: i32) -> u32 {
    todo!("x as u32")
}

// ══════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    // ── Part A: fn ptr ↔ void ptr ─────────────────────────────

    extern "C" fn double_it(x: i32) -> i32 {
        x * 2
    }

    #[test]
    fn test_ex23_fn_ptr_roundtrip() {
        unsafe {
            let vp = fn_ptr_to_void(double_it);
            assert!(!vp.is_null());
            let fp = void_to_fn_ptr(vp);
            assert_eq!(fp(21), 42);
        }
    }

    #[test]
    fn test_ex23_fn_ptr_identity() {
        // Round-trip should preserve the exact function.
        unsafe {
            let vp = fn_ptr_to_void(double_it);
            let fp = void_to_fn_ptr(vp);
            // Call multiple times to verify it's stable.
            assert_eq!(fp(0), 0);
            assert_eq!(fp(-5), -10);
            assert_eq!(fp(100), 200);
        }
    }

    // ── Part B: Nullable function pointers ─────────────────────

    extern "C" fn add_ten(x: i32) -> i32 {
        x + 10
    }

    #[test]
    fn test_ex23_raw_to_optional_fn_some() {
        let fp: extern "C" fn(i32) -> i32 = add_ten;
        let raw = fp as usize;
        unsafe {
            let opt = raw_to_optional_fn(raw);
            assert!(opt.is_some());
            assert_eq!(opt.unwrap()(5), 15);
        }
    }

    #[test]
    fn test_ex23_raw_to_optional_fn_none() {
        unsafe {
            let opt = raw_to_optional_fn(0);
            assert!(opt.is_none());
        }
    }

    #[test]
    fn test_ex23_invoke_optional_some() {
        let result = invoke_optional_callback(Some(add_ten), 7);
        assert_eq!(result, Some(17));
    }

    #[test]
    fn test_ex23_invoke_optional_none() {
        let result = invoke_optional_callback(None, 7);
        assert_eq!(result, None);
    }

    // ── Part C: int → enum ────────────────────────────────────

    #[test]
    fn test_ex23_status_transmute_valid() {
        unsafe {
            assert_eq!(status_from_int_transmute(0), Status::Ok);
            assert_eq!(status_from_int_transmute(-1), Status::NotFound);
            assert_eq!(status_from_int_transmute(-2), Status::PermissionDenied);
            assert_eq!(status_from_int_transmute(-3), Status::IoError);
        }
    }

    #[test]
    fn test_ex23_status_safe_valid() {
        assert_eq!(status_from_int_safe(0), Ok(Status::Ok));
        assert_eq!(status_from_int_safe(-1), Ok(Status::NotFound));
        assert_eq!(status_from_int_safe(-3), Ok(Status::IoError));
    }

    #[test]
    fn test_ex23_status_safe_invalid() {
        assert_eq!(status_from_int_safe(42), Err(42));
        assert_eq!(status_from_int_safe(-99), Err(-99));
    }

    #[test]
    fn test_ex23_status_try_from() {
        assert_eq!(Status::try_from(0), Ok(Status::Ok));
        assert_eq!(Status::try_from(-2), Ok(Status::PermissionDenied));
        assert!(Status::try_from(999).is_err());
    }

    // ── Part D: bytes ↔ struct ────────────────────────────────

    #[test]
    fn test_ex23_packet_roundtrip_transmute() {
        let header = PacketHeader {
            magic: 0xCAFE,
            version: 1,
            msg_type: 42,
            length: 1024,
        };
        let bytes = packet_to_bytes(&header);
        assert_eq!(bytes.len(), mem::size_of::<PacketHeader>());

        let recovered = unsafe { packet_from_bytes_transmute(&bytes) };
        assert_eq!(recovered, Some(header));
    }

    #[test]
    fn test_ex23_packet_roundtrip_safe() {
        let header = PacketHeader {
            magic: 0xBEEF,
            version: 2,
            msg_type: 7,
            length: 512,
        };
        let bytes = packet_to_bytes(&header);
        let recovered = packet_from_bytes_safe(&bytes);
        assert_eq!(recovered, Some(header));
    }

    #[test]
    fn test_ex23_packet_too_short() {
        let short = [0u8; 2];
        assert_eq!(unsafe { packet_from_bytes_transmute(&short) }, None,);
        assert_eq!(packet_from_bytes_safe(&short), None);
    }

    #[test]
    fn test_ex23_packet_to_bytes_content() {
        let header = PacketHeader {
            magic: 0x0102,
            version: 3,
            msg_type: 4,
            length: 0x05060708,
        };
        let bytes = packet_to_bytes(&header);
        // Verify we can read back the exact same fields.
        let h2 = packet_from_bytes_safe(&bytes).unwrap();
        let magic = h2.magic;
        assert_eq!(magic, 0x0102);
        assert_eq!(h2.version, 3);
        assert_eq!(h2.msg_type, 4);
        let length = h2.length;
        assert_eq!(length, 0x05060708);
    }

    // ── Part E: Safe alternatives ─────────────────────────────

    #[test]
    fn test_ex23_ptr_void_roundtrip() {
        let mut val: i32 = 42;
        let vp = ptr_to_void(&mut val as *mut i32);
        let p: *mut i32 = void_to_ptr(vp);
        assert_eq!(unsafe { *p }, 42);
    }

    #[test]
    fn test_ex23_f32_bits_roundtrip() {
        let original: f32 = std::f32::consts::PI;
        let bits = f32_to_u32_bits(original);
        let recovered = u32_to_f32_bits(bits);
        assert_eq!(recovered, original);
    }

    #[test]
    fn test_ex23_f32_known_bits() {
        // IEEE 754: 1.0f32 == 0x3F800000
        assert_eq!(f32_to_u32_bits(1.0), 0x3F80_0000);
        assert_eq!(u32_to_f32_bits(0x3F80_0000), 1.0);
    }

    #[test]
    fn test_ex23_i32_to_u32() {
        assert_eq!(i32_to_u32(0), 0u32);
        assert_eq!(i32_to_u32(-1), u32::MAX); // two's complement
        assert_eq!(i32_to_u32(42), 42u32);
    }
}
