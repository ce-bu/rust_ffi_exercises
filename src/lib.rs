//! # FFI Exercises
//!
//! Work through the exercises **in order**. Each module contains
//! `todo!()` stubs — implement them, then run:
//!
//! ```sh
//! cargo test ex01   # test a single exercise
//! cargo test        # test everything
//! ```
//!
//! ## Recommended order
//!
//!  1. `ex01_calling_c`      — Call C functions from Rust
//!  2. `ex02_rust_from_c`    — Expose Rust functions to C
//!  3. `ex03_structs`        — `#[repr(C)]` struct passing
//!  4. `ex04_strings`        — C-string interop
//!  5. `ex05_opaque`         — Opaque handle pattern
//!  6. `ex06_arrays`         — Array / slice passing
//!  7. `ex07_callbacks`      — Function-pointer callbacks
//!  8. `ex08_memory`         — Cross-boundary memory management
//!  9. `ex09_errors`         — Error codes + last-error pattern
//! 10. `ex10_maybe_uninit`   — `MaybeUninit` & `NonNull`
//! 11. `ex11_vtable`         — C++ vtable dispatch pattern
//! 12. `ex12_async`          — Async / threaded FFI interop
//! 13. `ex13_tagged_unions`  — `#[repr(C)]` tagged unions
//! 14. `ex14_panic_safety`   — `catch_unwind` at FFI boundaries
//! 15. `ex15_miri_ub`        — Detecting UB with Miri
//! 16. `ex16_pin_drop`       — Pin & drop guarantee
//! 17. `ex17_wrapping_c`     — Wrapping C opaque handles in safe Rust
//! 18. `ex18_closures`        — Invoking Rust closures from C
//! 19. `ex19_dyn_trait_ffi`   — Boxed dyn Trait across FFI
//! 20. `ex20_global_state`    — Global state & library init/shutdown
//! 21. `ex21_bitflags`        — Bitflags & C-style enums
//! 22. `ex22_lifetimes`       — Lifetime encoding with PhantomData
//! 23. `ex23_transmute`       — transmute & type reinterpretation
//! 24. `ex24_c_handles`       — C-owned opaque handles (out-parameter)
//! 25. `ex25_cpp_virtual`     — Calling C++ virtual functions (direct vtable)
//! 26. `ex26_cpp_exceptions`   — C++ exceptions across FFI boundaries
//! 27. `ex27_cpp_stl`          — Wrapping C++ classes & STL types
//! 28. `ex28_zst_ffi`          — Zero-sized types in FFI
//! 29. `ex29_unsafecell`        — UnsafeCell & aliasing in FFI

pub mod ex01_calling_c;
pub mod ex02_rust_from_c;
pub mod ex03_structs;
pub mod ex04_strings;
pub mod ex05_opaque;
// pub mod ex06_arrays;
// pub mod ex07_callbacks;
// pub mod ex08_memory;
// pub mod ex09_errors;
// pub mod ex10_maybe_uninit;
// pub mod ex11_vtable;
// pub mod ex12_async;
// pub mod ex13_tagged_unions;
// pub mod ex14_panic_safety;
// pub mod ex15_miri_ub;
// pub mod ex16_pin_drop;
// pub mod ex17_wrapping_c;
// pub mod ex18_closures;
// pub mod ex19_dyn_trait_ffi;
// pub mod ex20_global_state;
// pub mod ex21_bitflags;
// pub mod ex22_lifetimes;
// pub mod ex23_transmute;
// pub mod ex24_c_handles;
// pub mod ex25_cpp_virtual;
// pub mod ex26_cpp_exceptions;
// pub mod ex27_cpp_stl;
// pub mod ex28_zst_ffi;
// pub mod ex29_unsafecell;
