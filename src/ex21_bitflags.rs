//! # Exercise 21: Bitflags & C-Style Enums
//!
//! **Concept:** C APIs pervasively use bitwise-OR flags:
//!
//! ```c
//! int fd = open("file", O_RDONLY | O_CREAT | O_TRUNC);
//! SDL_Init(SDL_INIT_VIDEO | SDL_INIT_AUDIO);
//! ```
//!
//! Flags are typically `#define` constants or anonymous enums, and
//! the combined flag set is passed as an integer (`int`, `uint32_t`,
//! etc.).
//!
//! In Rust, the idiomatic approach is to create a **newtype** around
//! the integer and implement bitwise operators.  This gives us
//! type-safety (can't accidentally pass a flag where a length is
//! expected) while remaining ABI-compatible with C.
//!
//! ## Your task
//!
//! 1. Define `OpenFlags` as a newtype over `u32` with `#[repr(transparent)]`.
//! 2. Define individual flag constants.
//! 3. Implement `BitOr`, `BitAnd`, `BitOrAssign`, and a `contains`
//!    method.
//! 4. Use the flags in `extern "C"` APIs for a simulated file system.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex21
//! ```

use std::ffi::{c_char, CStr};
use std::fmt;

// ══════════════════════════════════════════════════════════════
// Part A — The Bitflag type
// ══════════════════════════════════════════════════════════════

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Define `OpenFlags` as a newtype wrapping `u32`.
// Use `#[repr(transparent)]` so the ABI is identical to `u32`.
//
// Define these flag constants (matching typical C conventions):
//
//   OPEN_READ     = 0x01   (open for reading)
//   OPEN_WRITE    = 0x02   (open for writing)
//   OPEN_CREATE   = 0x04   (create if not exists)
//   OPEN_TRUNCATE = 0x08   (truncate existing content)
//   OPEN_APPEND   = 0x10   (writes go to end)
//
// Hint:
//   #[repr(transparent)]
//   #[derive(Clone, Copy, PartialEq, Eq)]
//   pub struct OpenFlags(pub u32);
//
//   impl OpenFlags {
//       pub const READ:     OpenFlags = OpenFlags(0x01);
//       pub const WRITE:    OpenFlags = OpenFlags(0x02);
//       ...
//       pub const NONE:     OpenFlags = OpenFlags(0x00);
//   }

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct OpenFlags(pub u32);

impl OpenFlags {
    // TODO: define constants READ, WRITE, CREATE, TRUNCATE, APPEND, NONE
}

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Implement bitwise operators so flags can be combined naturally:
//
//   let flags = OpenFlags::READ | OpenFlags::WRITE;
//   flags |= OpenFlags::CREATE;
//   if flags.contains(OpenFlags::READ) { ... }
//
// Implement:
//   - `BitOr<Output = OpenFlags>`
//   - `BitAnd<Output = OpenFlags>`
//   - `BitOrAssign`
//   - `OpenFlags::contains(self, other: OpenFlags) -> bool`
//     → `(self.0 & other.0) == other.0`
//   - `OpenFlags::is_empty(self) -> bool`
//     → `self.0 == 0`

impl std::ops::BitOr for OpenFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        todo!("Self(self.0 | rhs.0)")
    }
}

impl std::ops::BitAnd for OpenFlags {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        todo!("Self(self.0 & rhs.0)")
    }
}

impl std::ops::BitOrAssign for OpenFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        todo!("self.0 |= rhs.0")
    }
}

impl OpenFlags {
    /// Returns `true` if all bits in `other` are set in `self`.
    pub fn contains(self, other: OpenFlags) -> bool {
        todo!("(self.0 & other.0) == other.0")
    }

    /// Returns `true` if no bits are set.
    pub fn is_empty(self) -> bool {
        todo!("self.0 == 0")
    }
}

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Implement `Debug` (or `Display`) for OpenFlags that prints the
// flag names, e.g.: "READ | WRITE | CREATE".
//
// This is optional but very useful for debugging FFI issues.

impl fmt::Debug for OpenFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!(
            "Build a string of flag names separated by ' | ', \
             or show '(empty)' / the raw hex if no names match"
        )
    }
}

// ══════════════════════════════════════════════════════════════
// Part B — Simulated file-system API using flags
// ══════════════════════════════════════════════════════════════

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Define a simple `FileEntry` and `FileHandle` system to exercise
// the flags.  The focus is on the flag checking logic, not on
// actually implementing a file system.
//
// `FileHandle` stores:
//   - `name: String`
//   - `flags: OpenFlags`
//   - `content: Vec<u8>`
//   - `position: usize`
//
// Then implement these extern "C" functions:
//
//   file_open(name, flags) → *mut FileHandle   (or null on error)
//   file_write(handle, data, len) → i32         (bytes written or -1)
//   file_read(handle, buf, len) → i32           (bytes read or -1)
//   file_close(handle) → i32                    (0 or -1)
//
// Rules enforced by flags:
//   - READ must be set to allow `file_read`.
//   - WRITE (or APPEND) must be set to allow `file_write`.
//   - TRUNCATE clears any existing content at open time.
//   - APPEND makes writes go to the end regardless of position.
//   - CREATE allows opening a new file; without it, only
//     "pre-existing" content can be opened (for this exercise,
//     all files are new, so CREATE is always needed — but check
//     the flag anyway).

pub struct FileHandle {
    // TODO: add fields for name, flags, content, position
}

/// Open a file with the given flags.
///
/// Returns null if:
///   - `name` is null
///   - `flags` is empty (no permission bits)
///
/// # Safety
/// `name` must be a valid NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn file_open(
    name: *const c_char,
    flags: OpenFlags,
) -> *mut FileHandle {
    todo!(
        "Validate name and flags, create FileHandle, \
         if TRUNCATE clear content, Box::into_raw"
    )
}

/// Write `len` bytes from `data` into the file.
///
/// Returns the number of bytes written, or -1 if:
///   - handle is null
///   - WRITE or APPEND flag was not set at open time
///
/// If APPEND is set, writes always go to the end.
///
/// # Safety
/// - `handle` must be from `file_open`.
/// - `data` must point to at least `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn file_write(
    handle: *mut FileHandle,
    data: *const u8,
    len: usize,
) -> i32 {
    todo!(
        "Check WRITE/APPEND flag, write bytes into content, \
         update position, return bytes written"
    )
}

/// Read up to `len` bytes from the file into `buf`.
///
/// Returns the number of bytes actually read, or -1 if:
///   - handle is null
///   - READ flag was not set
///
/// # Safety
/// - `handle` must be from `file_open`.
/// - `buf` must be writable for at least `len` bytes.
#[no_mangle]
pub unsafe extern "C" fn file_read(
    handle: *mut FileHandle,
    buf: *mut u8,
    len: usize,
) -> i32 {
    todo!(
        "Check READ flag, copy bytes from content[position..], \
         update position, return bytes read"
    )
}

/// Close the file handle and free resources.
///
/// # Safety
/// `handle` must have been returned by `file_open`.
#[no_mangle]
pub unsafe extern "C" fn file_close(handle: *mut FileHandle) -> i32 {
    todo!("Box::from_raw(handle), return 0")
}

// ══════════════════════════════════════════════════════════════
// Part C — C-style enum for status codes (bonus)
// ══════════════════════════════════════════════════════════════

// ── TODO 5 ─────────────────────────────────────────────────────
//
// Define a `Permission` enum that combines with `OpenFlags`:
//
//   #[repr(u32)]
//   pub enum Permission {
//       ReadOnly  = 0x01,           // READ
//       WriteOnly = 0x02,           // WRITE
//       ReadWrite = 0x01 | 0x02,    // READ | WRITE
//   }
//
// Implement `From<Permission>` for `OpenFlags`.
//
// This shows how C enums (non-combinable "mode" values) and
// bitflags (combinable) coexist — a very common pattern.

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    // TODO: define ReadOnly, WriteOnly, ReadWrite
    ReadOnly = 0x01,
    WriteOnly = 0x02,
    ReadWrite = 0x03,
}

impl From<Permission> for OpenFlags {
    fn from(p: Permission) -> Self {
        todo!("OpenFlags(p as u32)")
    }
}

// ══════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    // ── Part A: Bitwise operations ────────────────────────────

    #[test]
    fn test_ex21_bitor() {
        let flags = OpenFlags::READ | OpenFlags::WRITE;
        assert_eq!(flags.0, 0x03);
    }

    #[test]
    fn test_ex21_bitand() {
        let flags = OpenFlags::READ | OpenFlags::WRITE | OpenFlags::CREATE;
        let masked = flags & OpenFlags::WRITE;
        assert_eq!(masked.0, 0x02);
    }

    #[test]
    fn test_ex21_bitor_assign() {
        let mut flags = OpenFlags::READ;
        flags |= OpenFlags::WRITE;
        flags |= OpenFlags::CREATE;
        assert!(flags.contains(OpenFlags::READ));
        assert!(flags.contains(OpenFlags::WRITE));
        assert!(flags.contains(OpenFlags::CREATE));
    }

    #[test]
    fn test_ex21_contains() {
        let flags = OpenFlags::READ | OpenFlags::CREATE;
        assert!(flags.contains(OpenFlags::READ));
        assert!(flags.contains(OpenFlags::CREATE));
        assert!(!flags.contains(OpenFlags::WRITE));
        // contains with combined flags:
        assert!(flags.contains(OpenFlags::READ | OpenFlags::CREATE));
        assert!(!flags.contains(OpenFlags::READ | OpenFlags::WRITE));
    }

    #[test]
    fn test_ex21_is_empty() {
        assert!(OpenFlags::NONE.is_empty());
        assert!(!OpenFlags::READ.is_empty());
    }

    #[test]
    fn test_ex21_debug_format() {
        let flags = OpenFlags::READ | OpenFlags::WRITE;
        let s = format!("{:?}", flags);
        assert!(s.contains("READ"), "Debug output: {s}");
        assert!(s.contains("WRITE"), "Debug output: {s}");
    }

    // ── Part B: File API with flag enforcement ────────────────

    #[test]
    fn test_ex21_open_close() {
        let name = CString::new("test.txt").unwrap();
        let flags = OpenFlags::READ | OpenFlags::WRITE | OpenFlags::CREATE;
        unsafe {
            let h = file_open(name.as_ptr(), flags);
            assert!(!h.is_null());
            assert_eq!(file_close(h), 0);
        }
    }

    #[test]
    fn test_ex21_open_empty_flags() {
        let name = CString::new("test.txt").unwrap();
        unsafe {
            let h = file_open(name.as_ptr(), OpenFlags::NONE);
            assert!(h.is_null(), "should fail with no flags");
        }
    }

    #[test]
    fn test_ex21_write_requires_flag() {
        let name = CString::new("readonly.txt").unwrap();
        let data = b"hello";
        unsafe {
            // Open with READ only — write should fail.
            let h = file_open(name.as_ptr(), OpenFlags::READ | OpenFlags::CREATE);
            assert!(!h.is_null());
            assert_eq!(file_write(h, data.as_ptr(), data.len()), -1);
            file_close(h);
        }
    }

    #[test]
    fn test_ex21_read_requires_flag() {
        let name = CString::new("writeonly.txt").unwrap();
        unsafe {
            let h = file_open(
                name.as_ptr(),
                OpenFlags::WRITE | OpenFlags::CREATE,
            );
            assert!(!h.is_null());
            let mut buf = [0u8; 32];
            assert_eq!(file_read(h, buf.as_mut_ptr(), buf.len()), -1);
            file_close(h);
        }
    }

    #[test]
    fn test_ex21_write_and_read() {
        let name = CString::new("rw.txt").unwrap();
        let flags = OpenFlags::READ | OpenFlags::WRITE | OpenFlags::CREATE;
        let data = b"Rust FFI";
        unsafe {
            let h = file_open(name.as_ptr(), flags);
            assert!(!h.is_null());

            // Write data
            let written = file_write(h, data.as_ptr(), data.len());
            assert_eq!(written, data.len() as i32);

            // Reset position for reading (read from position 0)
            // Our handle tracks position; after write it's at the end,
            // so we need to re-open or the impl should reset.
            // For simplicity: just re-open.
            file_close(h);

            let h = file_open(name.as_ptr(), flags);
            // New handle → new content (this is in-memory, not persistent).
            // Instead, let's write then read from a fresh handle.
        }

        // Better test: write, then read back in the same handle
        // relying on position tracking.
        unsafe {
            let h = file_open(name.as_ptr(), flags);
            let w = file_write(h, data.as_ptr(), data.len());
            assert_eq!(w, data.len() as i32);

            // Manually set position back to 0 for reading.
            // (In our simplified API, we just test that read works
            //  from position 0 right after open — write advances
            //  position, so reading may return 0 bytes.)
            //
            // This tests that position tracking works correctly.
            let mut buf = [0u8; 32];
            let r = file_read(h, buf.as_mut_ptr(), buf.len());
            // Position is at end after write → 0 bytes to read.
            assert_eq!(r, 0);

            file_close(h);
        }
    }

    #[test]
    fn test_ex21_append_flag() {
        let name = CString::new("append.txt").unwrap();
        let flags = OpenFlags::READ | OpenFlags::APPEND | OpenFlags::CREATE;
        unsafe {
            let h = file_open(name.as_ptr(), flags);
            assert!(!h.is_null());

            let d1 = b"Hello";
            let d2 = b"World";
            file_write(h, d1.as_ptr(), d1.len());
            file_write(h, d2.as_ptr(), d2.len());

            // Read from beginning: should see "HelloWorld"
            // Reset position to 0 for reading.
            (*h).position = 0;  // direct field access in test
            let mut buf = [0u8; 64];
            let r = file_read(h, buf.as_mut_ptr(), buf.len());
            assert_eq!(r, 10);
            assert_eq!(&buf[..10], b"HelloWorld");

            file_close(h);
        }
    }

    #[test]
    fn test_ex21_truncate_flag() {
        let name = CString::new("trunc.txt").unwrap();
        let flags = OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE;
        unsafe {
            let h = file_open(name.as_ptr(), flags);
            let d = b"data";
            file_write(h, d.as_ptr(), d.len());
            file_close(h);
            // (In our in-memory model, every open starts fresh anyway.
            //  This test mainly verifies TRUNCATE doesn't cause errors.)
        }
    }

    // ── Part C: Permission enum → OpenFlags ───────────────────

    #[test]
    fn test_ex21_permission_to_flags() {
        let f: OpenFlags = Permission::ReadOnly.into();
        assert!(f.contains(OpenFlags::READ));
        assert!(!f.contains(OpenFlags::WRITE));

        let f: OpenFlags = Permission::ReadWrite.into();
        assert!(f.contains(OpenFlags::READ));
        assert!(f.contains(OpenFlags::WRITE));
    }

    #[test]
    fn test_ex21_permission_combined_with_flags() {
        let mut flags: OpenFlags = Permission::ReadWrite.into();
        flags |= OpenFlags::CREATE | OpenFlags::TRUNCATE;
        assert!(flags.contains(OpenFlags::READ));
        assert!(flags.contains(OpenFlags::WRITE));
        assert!(flags.contains(OpenFlags::CREATE));
        assert!(flags.contains(OpenFlags::TRUNCATE));
    }
}
