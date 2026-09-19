//! Property-based tests for the correctness properties of the `stringutils`
//! safe API (design.md "Correctness Properties", Properties 1-6).
//!
//! These are black-box integration tests exercising the public crate API
//! (`stringutils::...`) with the `proptest` library. Each property is
//! implemented as a single property test, tagged with its
//! `Feature: rust-cpp-ffi-wrapper, Property {n}: {text}` comment, and runs at
//! least 100 iterations.
//!
//! Property 7 (>=10,000 no-panic stress) and the valgrind leak test are added
//! separately in `tests/integration.rs` by Task 10.

use proptest::prelude::*;
use stringutils::{count_vowels, str_length, str_reverse, to_uppercase, Error};

/// Strategy producing arbitrary NUL-free Unicode strings.
///
/// Draws a vector of arbitrary `char`s (covering ASCII, empty, single-char,
/// special characters, and multi-byte / high-byte >127 sequences) and filters
/// out any interior NUL so the value is a valid C-string body.
fn nul_free_string() -> impl Strategy<Value = String> {
    proptest::collection::vec(any::<char>(), 0..50)
        .prop_map(|cs| cs.into_iter().filter(|&c| c != '\0').collect::<String>())
}

/// Strategy producing NUL-free ASCII-only strings (every byte < 128).
///
/// Used where the reversed/permuted bytes must remain valid UTF-8 (Property 2):
/// any permutation of ASCII bytes is itself valid UTF-8.
fn ascii_string() -> impl Strategy<Value = String> {
    proptest::collection::vec(1u8..128u8, 0..50)
        .prop_map(|bytes| String::from_utf8(bytes).expect("bytes < 128 are valid UTF-8"))
}

/// Strategy producing strings of only ASCII lowercase letters (Property 5).
fn ascii_lower_string() -> impl Strategy<Value = String> {
    proptest::collection::vec(proptest::char::range('a', 'z'), 0..50)
        .prop_map(|cs| cs.into_iter().collect::<String>())
}

/// Independent host-side model of the C++ ASCII vowel count: iterate the bytes
/// and count those equal to an ASCII vowel byte. Bytes > 127 can never equal an
/// ASCII vowel byte, so they are never counted (validates the >127 pass-through).
fn host_vowel_count(s: &str) -> usize {
    s.as_bytes()
        .iter()
        .filter(|&&b| matches!(b, b'a' | b'e' | b'i' | b'o' | b'u' | b'A' | b'E' | b'I' | b'O' | b'U'))
        .count()
}

/// Independent host-side model of the C++ ASCII uppercase transform: map each
/// byte, folding ASCII lowercase to uppercase and leaving every other byte
/// (including all bytes > 127) unchanged.
fn host_uppercase_bytes(s: &str) -> Vec<u8> {
    s.as_bytes()
        .iter()
        .map(|&b| if b.is_ascii_lowercase() { b - 32 } else { b })
        .collect()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 100, ..ProptestConfig::default() })]

    // Feature: rust-cpp-ffi-wrapper, Property 1: Length equals host byte length
    #[test]
    fn property_1_length_equals_host_byte_length(s in nul_free_string()) {
        // Unwrap rather than compare Results directly so the test does not depend
        // on `Error: PartialEq`.
        let len = str_length(&s).expect("str_length should succeed on NUL-free input");
        prop_assert_eq!(len, s.as_bytes().len());
    }

    // Feature: rust-cpp-ffi-wrapper, Property 2: Reverse is an involution
    //
    // The generator is ASCII-restricted (every byte < 128) on purpose: reversing
    // the BYTES of a multi-byte UTF-8 string produces invalid UTF-8, which the
    // safe API's `to_string_lossy` would repair with replacement characters,
    // breaking the involution. Any permutation of ASCII bytes is valid UTF-8, so
    // over ASCII inputs we can assert both directions exactly at the byte level:
    //   (i)  str_reverse(s) equals the host byte reversal of s, and
    //   (ii) str_reverse(str_reverse(s)) round-trips back to s.
    #[test]
    fn property_2_reverse_is_an_involution(s in ascii_string()) {
        let reversed = str_reverse(&s).expect("str_reverse should succeed on NUL-free input");

        // (i) equals the host-side byte reversal.
        let mut host = s.as_bytes().to_vec();
        host.reverse();
        prop_assert_eq!(reversed.as_bytes(), host.as_slice());

        // (ii) reversing twice reproduces the original.
        let round_trip =
            str_reverse(&reversed).expect("str_reverse should succeed on NUL-free input");
        prop_assert_eq!(round_trip, s);
    }

    // Feature: rust-cpp-ffi-wrapper, Property 3: Vowel count equals the host ASCII model
    #[test]
    fn property_3_vowel_count_equals_host_ascii_model(s in nul_free_string()) {
        let count = count_vowels(&s).expect("count_vowels should succeed on NUL-free input");
        prop_assert_eq!(count, host_vowel_count(&s));
    }

    // Feature: rust-cpp-ffi-wrapper, Property 4: Uppercase equals the host ASCII model
    #[test]
    fn property_4_uppercase_equals_host_ascii_model(s in nul_free_string()) {
        let upper = to_uppercase(&s).expect("to_uppercase should succeed on NUL-free input");
        // ASCII-uppercasing a valid UTF-8 string only changes ASCII lowercase
        // bytes and leaves all multi-byte (>127) bytes untouched, so the result
        // stays valid UTF-8 and a byte-level comparison is exact.
        let host = host_uppercase_bytes(&s);
        prop_assert_eq!(upper.as_bytes(), host.as_slice());
    }

    // Feature: rust-cpp-ffi-wrapper, Property 5: ASCII case round-trip
    #[test]
    fn property_5_ascii_case_round_trip(s in ascii_lower_string()) {
        let round_trip = to_uppercase(&s)
            .map(|u| u.to_lowercase())
            .expect("to_uppercase should succeed on NUL-free input");
        prop_assert_eq!(round_trip, s);
    }

    // Feature: rust-cpp-ffi-wrapper, Property 6: Interior NUL always yields a Conversion error
    #[test]
    fn property_6_interior_nul_yields_conversion_error(
        a in nul_free_string(),
        b in nul_free_string(),
    ) {
        // Build a string with at least one interior NUL by joining two NUL-free
        // parts around a '\0'.
        let s = format!("{a}\0{b}");

        prop_assert!(matches!(str_length(&s), Err(Error::Conversion(_))));
        prop_assert!(matches!(count_vowels(&s), Err(Error::Conversion(_))));
        prop_assert!(matches!(str_reverse(&s), Err(Error::Conversion(_))));
        prop_assert!(matches!(to_uppercase(&s), Err(Error::Conversion(_))));
    }
}
