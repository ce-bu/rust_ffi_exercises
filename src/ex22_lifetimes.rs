//! # Exercise 22: Lifetime Encoding with `PhantomData`
//!
//! **Concept:** C libraries often return "borrowed" handles — a
//! handle that is only valid as long as a parent handle is alive.
//! Examples:
//!
//! - **SQLite:** `sqlite3_stmt*` is only valid while the parent
//!   `sqlite3*` connection is open.
//! - **OpenSSL:** `SSL*` borrows from `SSL_CTX*`.
//! - **Iterators:** a cursor/iterator borrows from a collection.
//!
//! In Rust, we encode this relationship with a **lifetime parameter**
//! and `PhantomData`:
//!
//! ```text
//! struct ChildHandle<'db> {
//!     raw: *mut RawChild,
//!     _borrow: PhantomData<&'db ParentHandle>,
//! }
//! ```
//!
//! The compiler then prevents you from using the child after the
//! parent is dropped — even though the actual FFI types are just
//! raw pointers.
//!
//! ## Your task
//!
//! Build safe wrappers around a simulated "database + cursor" C API
//! where the cursor borrows from the database.
//!
//! ## Verify
//!
//! ```sh
//! cargo test ex22
//! ```

use std::marker::PhantomData;

// ══════════════════════════════════════════════════════════════
// Simulated C API (pre-provided — do NOT modify)
// ══════════════════════════════════════════════════════════════
//
// In a real project these would be `extern "C"` declarations.
// We simulate them in pure Rust so the exercise is self-contained.

/// Simulated opaque database handle.
pub struct RawDb {
    entries: Vec<(String, String)>,
}

/// Simulated opaque cursor handle.
pub struct RawCursor {
    db: *const RawDb,
    position: usize,
}

/// Simulated `db_open`.
pub fn raw_db_open() -> *mut RawDb {
    Box::into_raw(Box::new(RawDb {
        entries: Vec::new(),
    }))
}

/// Simulated `db_insert`.
pub unsafe fn raw_db_insert(
    db: *mut RawDb,
    key: &str,
    value: &str,
) {
    (*db).entries.push((key.to_owned(), value.to_owned()));
}

/// Simulated `db_cursor_open` — returns a cursor that **borrows**
/// from `db`.  The cursor is invalid after `db_close`.
pub unsafe fn raw_db_cursor_open(db: *const RawDb) -> *mut RawCursor {
    Box::into_raw(Box::new(RawCursor { db, position: 0 }))
}

/// Simulated `db_cursor_next` — returns `(key, value)` or None.
pub unsafe fn raw_db_cursor_next(
    cursor: *mut RawCursor,
) -> Option<(String, String)> {
    let c = &mut *cursor;
    let db = &*c.db;
    if c.position < db.entries.len() {
        let (k, v) = &db.entries[c.position];
        c.position += 1;
        Some((k.clone(), v.clone()))
    } else {
        None
    }
}

/// Simulated `db_cursor_reset`.
pub unsafe fn raw_db_cursor_reset(cursor: *mut RawCursor) {
    (*cursor).position = 0;
}

/// Simulated `db_cursor_close`.
pub unsafe fn raw_db_cursor_close(cursor: *mut RawCursor) {
    drop(Box::from_raw(cursor));
}

/// Simulated `db_close`.
pub unsafe fn raw_db_close(db: *mut RawDb) {
    drop(Box::from_raw(db));
}

// ══════════════════════════════════════════════════════════════
// Part A — Safe Database wrapper (owns the handle)
// ══════════════════════════════════════════════════════════════

// ── TODO 1 ─────────────────────────────────────────────────────
//
// Define `Database` — an owning wrapper around `*mut RawDb`.
//
// Implement:
//   - `Database::open() -> Self`
//   - `Database::insert(&mut self, key, value)`
//   - `Database::cursor(&self) -> Cursor<'_>`  ← borrows from self!
//   - `Drop` → calls `raw_db_close`
//
// The critical part is `cursor(&self) -> Cursor<'_>`:
// the returned cursor's lifetime is tied to `&self` so the
// compiler prevents using the cursor after the Database is dropped.

pub struct Database {
    // TODO: store *mut RawDb
}

impl Database {
    /// Open a new in-memory database.
    pub fn open() -> Self {
        todo!("Call raw_db_open, store the pointer")
    }

    /// Insert a key-value pair.
    pub fn insert(&mut self, key: &str, value: &str) {
        todo!("Call raw_db_insert")
    }

    /// Create a cursor that borrows from this database.
    ///
    /// The cursor's lifetime `'_` is tied to `&self`, so the
    /// compiler prevents:
    ///   ```compile_fail
    ///   let cursor = db.cursor();
    ///   drop(db);      // ERROR: db still borrowed by cursor
    ///   cursor.next();
    ///   ```
    pub fn cursor(&self) -> Cursor<'_> {
        todo!(
            "Call raw_db_cursor_open, wrap in Cursor with PhantomData"
        )
    }
}

impl Drop for Database {
    fn drop(&mut self) {
        todo!("Call raw_db_close")
    }
}

// ══════════════════════════════════════════════════════════════
// Part B — Cursor with lifetime tied to Database
// ══════════════════════════════════════════════════════════════

// ── TODO 2 ─────────────────────────────────────────────────────
//
// Define `Cursor<'db>` — a wrapper that borrows from a Database.
//
//   pub struct Cursor<'db> {
//       raw: *mut RawCursor,
//       _borrow: PhantomData<&'db Database>,
//   }
//
// `PhantomData<&'db Database>` tells the compiler:
// "this struct logically borrows a `&'db Database`" even though
// the actual field is a raw pointer with no lifetime.
//
// Without PhantomData, nothing prevents the cursor from outliving
// the database → use-after-free.

pub struct Cursor<'db> {
    // TODO: store *mut RawCursor and PhantomData<&'db Database>
    _lifetime: PhantomData<&'db ()>,  // placeholder; replace with proper struct
}

impl<'db> Cursor<'db> {
    /// Advance the cursor and return the next `(key, value)` pair,
    /// or `None` if exhausted.
    pub fn next(&mut self) -> Option<(String, String)> {
        todo!("Call raw_db_cursor_next")
    }

    /// Reset the cursor to the beginning.
    pub fn reset(&mut self) {
        todo!("Call raw_db_cursor_reset")
    }
}

impl<'db> Drop for Cursor<'db> {
    fn drop(&mut self) {
        todo!("Call raw_db_cursor_close")
    }
}

// ══════════════════════════════════════════════════════════════
// Part C — Iterator adapter
// ══════════════════════════════════════════════════════════════

// ── TODO 3 ─────────────────────────────────────────────────────
//
// Implement `Iterator` for `Cursor` so callers can use `for`
// loops and iterator combinators.
//
//   impl<'db> Iterator for Cursor<'db> {
//       type Item = (String, String);
//       fn next(&mut self) -> Option<Self::Item> {
//           self.next()    // delegate to the method above
//       }
//   }
//
// Problem: `next` name conflicts with `Iterator::next`!
// Rename the cursor's method to `advance` (or similar) and have
// `Iterator::next` call it.
//
// OR: keep them separate by calling `Cursor::next` through the
// method that wraps raw_db_cursor_next directly inside the
// Iterator impl (since they're in the same struct).

// impl<'db> Iterator for Cursor<'db> {
//     type Item = (String, String);
//     fn next(&mut self) -> Option<Self::Item> {
//         todo!("Call raw_db_cursor_next")
//     }
// }

// ══════════════════════════════════════════════════════════════
// Part D — Transaction (mutable borrow of Database)
// ══════════════════════════════════════════════════════════════

// ── TODO 4 ─────────────────────────────────────────────────────
//
// Define `Transaction<'db>` — a wrapper that **mutably** borrows
// a Database.  While a transaction is active, no other cursors or
// transactions can be created (enforced by Rust's borrow checker).
//
//   pub struct Transaction<'db> {
//       db: &'db mut Database,
//       committed: bool,
//   }
//
// Implement:
//   - `Database::transaction(&mut self) -> Transaction<'_>`
//   - `Transaction::insert(&mut self, key, value)`
//   - `Transaction::commit(self)` — consumes the transaction
//   - `Drop` — if not committed, the transaction could rollback
//     (for this exercise, just mark it as done)
//
// The `&mut self` borrow in `Database::transaction` prevents:
//   ```compile_fail
//   let tx = db.transaction();
//   db.insert("k", "v");   // ERROR: db mutably borrowed by tx
//   ```

pub struct Transaction<'db> {
    // TODO: store &'db mut Database and committed flag
    _lifetime: PhantomData<&'db ()>,  // placeholder
}

impl Database {
    /// Begin a transaction.  While the transaction exists, the
    /// database cannot be used directly (exclusive mutable borrow).
    pub fn transaction(&mut self) -> Transaction<'_> {
        todo!("Create Transaction {{ db: self, committed: false }}")
    }
}

impl<'db> Transaction<'db> {
    /// Insert within the transaction.
    pub fn insert(&mut self, key: &str, value: &str) {
        todo!("Call self.db.insert(key, value)")
    }

    /// Commit the transaction, consuming it.
    pub fn commit(self) {
        todo!("Set committed = true (use interior mutability or consume)")
    }
}

impl<'db> Drop for Transaction<'db> {
    fn drop(&mut self) {
        // In a real implementation, uncommitted transactions would
        // rollback here.  For this exercise, just note the drop.
        // (No todo! — this can be empty or print a debug message.)
    }
}

// ══════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ex22_open_close() {
        let _db = Database::open();
        // Drop runs automatically.
    }

    #[test]
    fn test_ex22_insert_and_cursor() {
        let mut db = Database::open();
        db.insert("name", "Ferris");
        db.insert("lang", "Rust");

        let mut cursor = db.cursor();
        let (k1, v1) = cursor.next().unwrap();
        assert_eq!(k1, "name");
        assert_eq!(v1, "Ferris");

        let (k2, v2) = cursor.next().unwrap();
        assert_eq!(k2, "lang");
        assert_eq!(v2, "Rust");

        assert!(cursor.next().is_none());
    }

    #[test]
    fn test_ex22_cursor_reset() {
        let mut db = Database::open();
        db.insert("a", "1");

        let mut cursor = db.cursor();
        assert!(cursor.next().is_some());
        assert!(cursor.next().is_none());

        cursor.reset();
        assert!(cursor.next().is_some()); // can read again
    }

    #[test]
    fn test_ex22_multiple_cursors() {
        // Multiple cursors can exist simultaneously (shared borrows).
        let mut db = Database::open();
        db.insert("x", "10");

        let mut c1 = db.cursor();
        let mut c2 = db.cursor();

        assert_eq!(c1.next().unwrap().0, "x");
        assert_eq!(c2.next().unwrap().0, "x");
    }

    #[test]
    fn test_ex22_cursor_drops_before_db() {
        let mut db = Database::open();
        db.insert("k", "v");
        {
            let mut cursor = db.cursor();
            assert!(cursor.next().is_some());
            // cursor dropped here
        }
        // db is still usable after cursor is dropped
        db.insert("k2", "v2");
    }

    // This test should NOT compile — uncomment to verify the
    // lifetime enforcement.  (It's left commented so `cargo test`
    // passes.)
    //
    // #[test]
    // fn test_ex22_cursor_outlives_db_fails() {
    //     let cursor;
    //     {
    //         let mut db = Database::open();
    //         db.insert("k", "v");
    //         cursor = db.cursor();
    //         // db dropped here
    //     }
    //     cursor.next(); // ERROR: db does not live long enough
    // }

    #[test]
    fn test_ex22_transaction_insert() {
        let mut db = Database::open();
        {
            let mut tx = db.transaction();
            tx.insert("key", "value");
            tx.commit();
        }
        // After transaction, data is visible.
        let mut cursor = db.cursor();
        let (k, v) = cursor.next().unwrap();
        assert_eq!(k, "key");
        assert_eq!(v, "value");
    }

    // This test should NOT compile — uncomment to verify.
    //
    // #[test]
    // fn test_ex22_transaction_blocks_cursor() {
    //     let mut db = Database::open();
    //     let tx = db.transaction();       // &mut borrow
    //     let cursor = db.cursor();        // ERROR: already mutably borrowed
    // }

    // This test should NOT compile — uncomment to verify.
    //
    // #[test]
    // fn test_ex22_transaction_blocks_insert() {
    //     let mut db = Database::open();
    //     let tx = db.transaction();
    //     db.insert("k", "v");  // ERROR: db mutably borrowed by tx
    // }
}
