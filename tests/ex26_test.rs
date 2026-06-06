// tests/ex26_test.rs — Integration tests for C++ exception handling
#![cfg(not(test))]
use rust_ffi_exercises::ex26_cpp_exceptions::*;

/* ══════════════════════════════════════════════════════════════
 * Basic arithmetic wrappers
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn divide_success() {
    let r = divide(10.0, 3.0).unwrap();
    assert!((r - 10.0 / 3.0).abs() < 1e-12);
}

#[test]
fn divide_by_zero_returns_domain_error() {
    let e = divide(1.0, 0.0).unwrap_err();
    assert_eq!(e.code, CPP_EX_ERR_DOMAIN);
    assert!(
        e.message.contains("division by zero"),
        "msg = {}",
        e.message
    );
    assert_eq!(e.type_name, "std::domain_error");
}

#[test]
fn sqrt_success() {
    let r = sqrt_checked(25.0).unwrap();
    assert!((r - 5.0).abs() < 1e-12);
}

#[test]
fn sqrt_negative_returns_domain_error() {
    let e = sqrt_checked(-1.0).unwrap_err();
    assert_eq!(e.code, CPP_EX_ERR_DOMAIN);
    assert!(e.message.contains("negative"), "msg = {}", e.message);
}

/* ══════════════════════════════════════════════════════════════
 * String parsing
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn parse_int_success() {
    assert_eq!(parse_int("42").unwrap(), 42);
    assert_eq!(parse_int("-100").unwrap(), -100);
    assert_eq!(parse_int("0").unwrap(), 0);
}

#[test]
fn parse_int_invalid_string() {
    let e = parse_int("hello").unwrap_err();
    assert_eq!(e.code, CPP_EX_ERR_INVALID);
    assert!(e.message.contains("invalid integer"), "msg = {}", e.message);
}

#[test]
fn parse_int_empty_string() {
    let e = parse_int("").unwrap_err();
    assert_eq!(e.code, CPP_EX_ERR_INVALID);
    assert!(e.message.contains("empty"), "msg = {}", e.message);
}

/* ══════════════════════════════════════════════════════════════
 * process_data — custom exception class
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn process_data_success() {
    let data = [1u8, 2, 3, 4, 5];
    let checksum = process_data(&data).unwrap();
    assert_eq!(checksum, 15);
}

#[test]
fn process_data_empty_is_custom_error() {
    let e = process_data(&[]).unwrap_err();
    assert_eq!(e.code, CPP_EX_ERR_CUSTOM);
    assert!(e.message.contains("empty"), "msg = {}", e.message);
    assert_eq!(e.type_name, "ProcessingError");
}

#[test]
fn process_data_invalid_byte() {
    let data = [1u8, 2, 0xFF, 4];
    let e = process_data(&data).unwrap_err();
    assert_eq!(e.code, CPP_EX_ERR_CUSTOM);
    assert!(e.message.contains("0xFF"), "msg = {}", e.message);
}

#[test]
fn process_data_too_large() {
    let data = vec![1u8; 5000];
    let e = process_data(&data).unwrap_err();
    assert_eq!(e.code, CPP_EX_ERR_CUSTOM);
    assert!(e.message.contains("too large"), "msg = {}", e.message);
}

/* ══════════════════════════════════════════════════════════════
 * Non-std::exception throw (integer)
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn trigger_unknown_catches_int_throw() {
    let e = trigger_unknown().unwrap_err();
    assert_eq!(e.code, CPP_EX_ERR_UNKNOWN);
    assert!(e.message.contains("42"), "msg = {}", e.message);
    assert_eq!(e.type_name, "int");
}

/* ══════════════════════════════════════════════════════════════
 * Display + Error trait
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn exception_display_format() {
    let e = divide(1.0, 0.0).unwrap_err();
    let s = format!("{e}");
    assert!(s.contains("C++ exception"), "display = {s}");
    assert!(s.contains("division by zero"), "display = {s}");
}

#[test]
fn exception_is_std_error() {
    fn assert_error<E: std::error::Error>(_: &E) {}
    let e = divide(1.0, 0.0).unwrap_err();
    assert_error(&e);
}

/* ══════════════════════════════════════════════════════════════
 * Error info retrieval
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn get_last_error_after_success_yields_defaults() {
    // A successful call clears the error.
    let _ = divide(6.0, 2.0).unwrap();
    clear_error();
    let e = get_last_cpp_error(0);
    // After clear, message should be empty.
    assert!(e.message.is_empty(), "msg = {:?}", e.message);
}

/* ══════════════════════════════════════════════════════════════
 * Callback integration — map_array
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn map_array_double_values() {
    let input = [1.0, 2.0, 3.0, 4.0];
    let output = map_array(&input, |x| Ok(x * 2.0)).unwrap();
    assert_eq!(output, vec![2.0, 4.0, 6.0, 8.0]);
}

#[test]
fn map_array_empty() {
    let output = map_array(&[], |x| Ok(x)).unwrap();
    assert!(output.is_empty());
}

#[test]
fn map_array_callback_error() {
    let input = [1.0, -1.0, 3.0];
    let e = map_array(&input, |x| if x < 0.0 { Err(-42) } else { Ok(x.sqrt()) }).unwrap_err();
    assert_eq!(e.code, -42);
    assert!(e.message.contains("callback error"), "msg = {}", e.message);
}

/* ══════════════════════════════════════════════════════════════
 * Thread isolation — errors are per-thread
 * ══════════════════════════════════════════════════════════════ */

#[test]
fn errors_are_thread_local() {
    use std::sync::Arc;
    use std::sync::Barrier;

    let barrier = Arc::new(Barrier::new(2));

    let b1 = Arc::clone(&barrier);
    let t1 = std::thread::spawn(move || {
        let _ = divide(1.0, 0.0); // sets error on thread 1
        b1.wait();
        // After both threads have set their errors,
        // retrieve thread 1's error:
        let e = get_last_cpp_error(CPP_EX_ERR_DOMAIN);
        assert!(
            e.message.contains("division by zero"),
            "thread 1: msg = {}",
            e.message
        );
    });

    let b2 = Arc::clone(&barrier);
    let t2 = std::thread::spawn(move || {
        let _ = parse_int("abc"); // sets error on thread 2
        b2.wait();
        let e = get_last_cpp_error(CPP_EX_ERR_INVALID);
        assert!(
            e.message.contains("invalid integer"),
            "thread 2: msg = {}",
            e.message
        );
    });

    t1.join().unwrap();
    t2.join().unwrap();
}
