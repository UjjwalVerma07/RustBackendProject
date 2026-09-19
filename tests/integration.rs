//! Integration safety tests for the `stringutils` safe API.
//!
//! These are black-box tests exercising only the public crate API
//! (`stringutils::...`). They complement the correctness properties in
//! `tests/properties.rs` (Properties 1-6) by covering the safety property that
//! could not be expressed as an input-varying correctness property:
//!
//!   * [`stress_no_panic_sustained_execution`] is the executable form of
//!     Property 7 — it drives each of the four safe-API functions over at least
//!     10,000 generated NUL-free inputs and asserts sustained execution without
//!     panicking, so the allocate/copy/free path (with its exactly-once free via
//!     `FreeGuard`) runs under load without aborting (Req 7.6, 5.11).
//!
//!   * [`leak_detection_many_allocations`] performs many `str_reverse` /
//!     `to_uppercase` calls (each of which allocates in C++ and frees via
//!     `FreeGuard`) so it can be run under valgrind to confirm zero leaked bytes
//!     and zero invalid frees (Req 7.7, 5.11).
//!
//! Both tests use a small deterministic generator rather than `proptest`: for a
//! fixed high-iteration loop a rotating-character generator is clearer, faster,
//! and reproducible, and the goal here is sustained execution rather than
//! input-space search (that is what the property tests are for).

use stringutils::{count_vowels, str_length, str_reverse, to_uppercase};

/// Build a deterministic, varied, NUL-free string for iteration `i`.
///
/// The length cycles through `0..64` and the bytes are drawn from the ASCII
/// lowercase alphabet with a rotating offset, so successive iterations produce
/// differing content and lengths (including the empty string when `i % 64 == 0`).
/// The output is pure ASCII and therefore never contains an interior NUL.
fn generated_input(i: usize) -> String {
    let len = i % 64;
    (0..len)
        .map(|j| (b'a' + (((i + j) % 26) as u8)) as char)
        .collect()
}

/// A handful of fixed edge inputs mixed into the stress loop: empty, single
/// character, a multi-byte UTF-8 string with bytes > 127, and a long string.
/// None contains an interior NUL.
fn edge_inputs() -> Vec<String> {
    vec![
        String::new(),
        "a".to_string(),
        "héllo 世界".to_string(),
        "z".repeat(4096),
    ]
}

// Feature: rust-cpp-ffi-wrapper, Property 7: Repeated invocation never panics
//
// Drives all four safe-API functions over more than 10,000 NUL-free inputs
// (10,000 generated + a repeated set of fixed edge inputs) and asserts each call
// returns `Ok`. For the string-returning functions the result is consumed via
// `std::hint::black_box` so the compiler cannot optimize away the
// allocate + copy + free round-trip; the guard's exactly-once free therefore
// runs on every iteration. Semantic outputs are intentionally NOT asserted here
// (that is covered by the unit and property tests) — the goal is sustained
// execution without panic or leak.
//
// Validates: Requirements 7.6, 5.11
#[test]
fn stress_no_panic_sustained_execution() {
    const ITERATIONS: usize = 10_000;

    for i in 0..ITERATIONS {
        let s = generated_input(i);

        // Count-returning functions: unwrap to assert Ok, and force use of the
        // returned count.
        let len = str_length(&s).expect("str_length must succeed on NUL-free input");
        let vowels = count_vowels(&s).expect("count_vowels must succeed on NUL-free input");
        std::hint::black_box((len, vowels));

        // String-returning functions: unwrap to assert Ok and black-box the
        // owned String so the allocate/copy/free path actually runs.
        let reversed = str_reverse(&s).expect("str_reverse must succeed on NUL-free input");
        let upper = to_uppercase(&s).expect("to_uppercase must succeed on NUL-free input");
        std::hint::black_box((reversed, upper));
    }

    // Mix in the fixed edge inputs (also many times) so empty / single-char /
    // multibyte / long inputs are exercised under the same sustained loop.
    for _ in 0..1_000 {
        for s in edge_inputs() {
            let len = str_length(&s).expect("str_length must succeed on NUL-free input");
            let vowels = count_vowels(&s).expect("count_vowels must succeed on NUL-free input");
            std::hint::black_box((len, vowels));

            let reversed = str_reverse(&s).expect("str_reverse must succeed on NUL-free input");
            let upper = to_uppercase(&s).expect("to_uppercase must succeed on NUL-free input");
            std::hint::black_box((reversed, upper));
        }
    }
}

/// Leak-detection test for the C++ allocation path (Req 7.7, 5.11).
///
/// Performs several thousand `str_reverse` and `to_uppercase` calls over varied
/// NUL-free inputs. Each of those calls allocates a fresh buffer in C++ and
/// releases it through `FreeGuard` (C++ `delete[]` via `free_string`) after the
/// bytes are copied into an owned `String`, so a missing or double free would
/// show up under a leak detector.
///
/// This test is designed to be run under valgrind:
///
/// ```text
/// valgrind --leak-check=full --error-exitcode=1 <integration-test-binary> \
///     leak_detection_many_allocations --test-threads=1
/// ```
///
/// A correct implementation reports "definitely lost: 0 bytes" (the C++
/// allocations are all freed) and exits zero. Under a normal `cargo test` run,
/// with no leak detector attached, it simply passes as a functional test: every
/// call must return `Ok` and produce a non-panicking result.
#[test]
fn leak_detection_many_allocations() {
    const ITERATIONS: usize = 5_000;

    for i in 0..ITERATIONS {
        let s = generated_input(i);

        // Each of these allocates in C++ and frees via FreeGuard once the bytes
        // have been copied into the owned String below.
        let reversed = str_reverse(&s).expect("str_reverse must succeed on NUL-free input");
        let upper = to_uppercase(&s).expect("to_uppercase must succeed on NUL-free input");

        // Consume the owned Strings so the allocate/copy/free round-trip is not
        // optimized away.
        std::hint::black_box((reversed, upper));
    }

    // Also exercise the multi-byte / high-byte (>127) edge input repeatedly so
    // the leak check covers the non-ASCII pass-through path.
    for _ in 0..ITERATIONS {
        let reversed = str_reverse("héllo 世界").expect("str_reverse must succeed");
        let upper = to_uppercase("héllo 世界").expect("to_uppercase must succeed");
        std::hint::black_box((reversed, upper));
    }
}
