//! `stringutils`: a safe Rust wrapper around a minimal C++ string-utilities library.
//!
//! This module is the crate's **safe API**. No public signature contains the `unsafe`
//! keyword (Req 5.2): every function takes a `&str`, returns a [`Result`], and confines
//! the raw FFI calls to internal `unsafe` blocks that call into [`ffi`] (Req 5.3). The
//! only other place `unsafe` appears is [`FreeGuard`]'s `Drop` impl, which frees C-owned
//! memory exactly once (Req 5.7, 5.11).
//!
//! # Memory model
//! `str_reverse` and `to_uppercase` receive a freshly heap-allocated C string from the
//! C++ library. The safe API copies those bytes into an owned Rust [`String`] and then
//! releases the C allocation via the C-provided `free_string` (never Rust's allocator).
//! Because the copy happens before the C buffer is freed, the returned `String` never
//! borrows freed memory.
//!
//! # ASCII semantics
//! Uppercase conversion and vowel counting are ASCII-defined: any input byte greater than
//! 127 passes through unchanged and is never counted as a vowel.
//!
//! # Interior NUL
//! Every function converts its `&str` input to a C string first. If the input contains an
//! interior NUL byte the conversion fails and the function returns
//! [`Err(Error::Conversion)`](Error::Conversion) without calling into the C library.

mod error;
mod ffi;

pub use error::Error;

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

/// Convert a `&str` into a [`CString`], mapping an interior-NUL [`NulError`] to
/// [`Error::Conversion`] via the `From<NulError>` impl.
///
/// No `unsafe` is needed for this conversion (Req 5.4, 5.5).
///
/// [`NulError`]: std::ffi::NulError
fn to_cstring(input: &str) -> Result<CString, Error> {
    Ok(CString::new(input)?)
}

/// Owns a `*mut c_char` returned by a C_Library string-producing function and guarantees
/// it is freed exactly once, on every return path — normal return, early `?` return, and
/// unwinding during a panic — via the C-provided `free_string` (Req 5.7, 5.11).
///
/// This is the single owner of the raw pointer, which is what makes the free
/// exactly-once. The allocator must match the C++ `new[]`/`delete[]` pair, so freeing
/// goes through `free_string` rather than Rust's allocator or `libc::free`.
struct FreeGuard(*mut c_char);

impl Drop for FreeGuard {
    fn drop(&mut self) {
        // SAFETY: `self.0` came directly from a C_Library string-producing function
        // (`str_reverse` / `to_uppercase`) and is never handed out or freed elsewhere,
        // so this is the only free of that pointer. `free_string` is a no-op on null, so
        // the call is sound even for a null pointer. Called exactly once because the
        // guard is the sole owner and `drop` runs once.
        unsafe { ffi::free_string(self.0) }
    }
}

/// Returns the length of `input` in bytes, excluding the terminating NUL.
///
/// The length is the UTF-8 byte length of the input (not the number of `char`s), matching
/// `input.as_bytes().len()` for any NUL-free input.
///
/// # Errors
/// Returns [`Err(Error::Conversion)`](Error::Conversion) if `input` contains an interior
/// NUL byte; the C library is not called in that case.
///
/// # Examples
/// ```
/// assert_eq!(stringutils::str_length("hello").unwrap(), 5);
/// assert_eq!(stringutils::str_length("").unwrap(), 0);
/// // Multi-byte UTF-8 counts bytes, not characters.
/// assert_eq!(stringutils::str_length("héllo").unwrap(), "héllo".len());
///
/// // An interior NUL is a conversion error.
/// assert!(stringutils::str_length("a\0b").is_err());
/// ```
pub fn str_length(input: &str) -> Result<usize, Error> {
    let c = to_cstring(input)?;
    // SAFETY: `c` is a valid, NUL-terminated CString kept alive for the whole call, so
    // `c.as_ptr()` is a valid `*const c_char`. `str_length` only reads the string and
    // returns a count; no allocation crosses back, so nothing needs freeing.
    let n = unsafe { ffi::str_length(c.as_ptr()) };
    Ok(n as usize)
}

/// Returns the count of ASCII vowels (`a`, `e`, `i`, `o`, `u`, case-insensitive) in
/// `input`.
///
/// Vowel counting is ASCII-defined: bytes greater than 127 are never counted (so accented
/// vowels such as `é` do not count).
///
/// # Errors
/// Returns [`Err(Error::Conversion)`](Error::Conversion) if `input` contains an interior
/// NUL byte; the C library is not called in that case.
///
/// # Examples
/// ```
/// assert_eq!(stringutils::count_vowels("hello").unwrap(), 2);
/// assert_eq!(stringutils::count_vowels("AEIOU").unwrap(), 5);
/// assert_eq!(stringutils::count_vowels("").unwrap(), 0);
/// // Non-ASCII vowels are not counted (ASCII-defined semantics).
/// assert_eq!(stringutils::count_vowels("héllo").unwrap(), 1);
///
/// // An interior NUL is a conversion error.
/// assert!(stringutils::count_vowels("a\0b").is_err());
/// ```
pub fn count_vowels(input: &str) -> Result<usize, Error> {
    let c = to_cstring(input)?;
    // SAFETY: `c` is a valid, NUL-terminated CString kept alive for the whole call, so
    // `c.as_ptr()` is a valid `*const c_char`. `count_vowels` only reads the string and
    // returns a count; no allocation crosses back, so nothing needs freeing.
    let n = unsafe { ffi::count_vowels(c.as_ptr()) };
    Ok(n as usize)
}

/// Returns a new [`String`] containing the bytes of `input` in reverse byte order.
///
/// The reversal is over UTF-8 bytes, not `char`s, matching the C library's byte-level
/// behavior. The returned `String` is owned by the caller; the C allocation is copied and
/// then released internally.
///
/// # Errors
/// - Returns [`Err(Error::Conversion)`](Error::Conversion) if `input` contains an interior
///   NUL byte; the C library is not called in that case.
/// - Returns [`Err(Error::NullResult)`](Error::NullResult) if the C library reports an
///   allocation failure.
///
/// # Examples
/// ```
/// assert_eq!(stringutils::str_reverse("abc").unwrap(), "cba");
/// assert_eq!(stringutils::str_reverse("").unwrap(), "");
///
/// // An interior NUL is a conversion error.
/// assert!(stringutils::str_reverse("a\0b").is_err());
/// ```
pub fn str_reverse(input: &str) -> Result<String, Error> {
    let c = to_cstring(input)?;
    // SAFETY: `c` is a valid, NUL-terminated CString kept alive for the whole call, so
    // `c.as_ptr()` is a valid `*const c_char`. The returned pointer is either null
    // (checked below without dereferencing) or a heap buffer owned by the caller.
    let ptr = unsafe { ffi::str_reverse(c.as_ptr()) };
    from_c_owned(ptr)
}

/// Returns a new [`String`] containing the ASCII-uppercase form of `input`.
///
/// Uppercasing is ASCII-defined: ASCII lowercase letters (`a`–`z`) are folded to
/// uppercase and every byte greater than 127 passes through unchanged. So `"héllo"`
/// uppercases the ASCII letters but leaves the two `é` bytes intact. The returned
/// `String` is owned by the caller; the C allocation is copied and then released
/// internally.
///
/// # Errors
/// - Returns [`Err(Error::Conversion)`](Error::Conversion) if `input` contains an interior
///   NUL byte; the C library is not called in that case.
/// - Returns [`Err(Error::NullResult)`](Error::NullResult) if the C library reports an
///   allocation failure.
///
/// # Examples
/// ```
/// assert_eq!(stringutils::to_uppercase("hello").unwrap(), "HELLO");
/// assert_eq!(stringutils::to_uppercase("").unwrap(), "");
/// // Non-ASCII bytes pass through unchanged (ASCII-defined semantics).
/// assert_eq!(stringutils::to_uppercase("héllo").unwrap(), "HéLLO");
///
/// // An interior NUL is a conversion error.
/// assert!(stringutils::to_uppercase("a\0b").is_err());
/// ```
pub fn to_uppercase(input: &str) -> Result<String, Error> {
    let c = to_cstring(input)?;
    // SAFETY: `c` is a valid, NUL-terminated CString kept alive for the whole call, so
    // `c.as_ptr()` is a valid `*const c_char`. The returned pointer is either null
    // (checked below without dereferencing) or a heap buffer owned by the caller.
    let ptr = unsafe { ffi::to_uppercase(c.as_ptr()) };
    from_c_owned(ptr)
}

/// Turn a `*mut c_char` returned by a string-producing C function into an owned [`String`],
/// freeing the C allocation exactly once.
///
/// A null pointer signals allocation failure and yields [`Error::NullResult`] without any
/// dereference (Req 5.10). A non-null pointer is immediately wrapped in a [`FreeGuard`] so
/// the C buffer is freed on every return path; the bytes are copied into an owned `String`
/// *before* the guard drops, so the returned value never borrows freed memory
/// (Req 5.6, 5.7, 5.8, 5.11, 5.12).
fn from_c_owned(ptr: *mut c_char) -> Result<String, Error> {
    if ptr.is_null() {
        return Err(Error::NullResult);
    }
    // Sole owner of the C allocation from here on; freed exactly once when `guard` drops.
    let guard = FreeGuard(ptr);
    // SAFETY: `ptr` is non-null (checked above) and points at a NUL-terminated buffer
    // produced by the C library. `guard` keeps it alive for the duration of this borrow,
    // and `to_string_lossy().into_owned()` copies the bytes into an owned String before
    // the guard drops, so the copy never observes freed memory.
    let owned = unsafe { CStr::from_ptr(guard.0) }
        .to_string_lossy()
        .into_owned();
    // `guard` drops here, freeing the C allocation exactly once via `free_string`.
    Ok(owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Representative valid inputs (Req 7.1) ---

    #[test]
    fn representative_valid_inputs() {
        assert_eq!(str_length("hello world").unwrap(), 11);
        assert_eq!(count_vowels("hello world").unwrap(), 3);
        assert_eq!(str_reverse("hello").unwrap(), "olleh");
        assert_eq!(to_uppercase("hello").unwrap(), "HELLO");
    }

    // --- Empty-string input (Req 7.2) ---

    #[test]
    fn empty_string_input() {
        assert_eq!(str_length("").unwrap(), 0);
        assert_eq!(count_vowels("").unwrap(), 0);
        assert_eq!(str_reverse("").unwrap(), "");
        assert_eq!(to_uppercase("").unwrap(), "");
    }

    // --- Single-character input (Req 7.3) ---

    #[test]
    fn single_character_input() {
        assert_eq!(str_length("a").unwrap(), 1);
        assert_eq!(count_vowels("a").unwrap(), 1);
        assert_eq!(count_vowels("z").unwrap(), 0);
        assert_eq!(str_reverse("a").unwrap(), "a");
        assert_eq!(to_uppercase("a").unwrap(), "A");
        assert_eq!(to_uppercase("Z").unwrap(), "Z");
    }

    // --- Special-character and multi-byte Unicode input (Req 7.4, 7.10) ---

    #[test]
    fn special_and_multibyte_unicode_input_does_not_panic() {
        let input = "héllo 世界!";

        // str_length equals the input's byte length, not its char count.
        assert_eq!(str_length(input).unwrap(), input.as_bytes().len());

        // These simply must not panic on multi-byte / high-byte input.
        let _ = count_vowels(input).unwrap();
        let _ = str_reverse(input).unwrap();
        let _ = to_uppercase(input).unwrap();
    }

    #[test]
    fn uppercase_leaves_non_ascii_bytes_unchanged() {
        // The two bytes of `é` (0xC3 0xA9) are > 127 and must pass through unchanged,
        // while the surrounding ASCII letters are uppercased.
        assert_eq!(to_uppercase("héllo").unwrap(), "HéLLO");

        // The non-ASCII bytes are byte-identical to the input's non-ASCII bytes.
        let out = to_uppercase("héllo").unwrap();
        assert_eq!(&out.as_bytes()[1..3], "é".as_bytes());
    }

    #[test]
    fn vowels_do_not_count_non_ascii() {
        // Accented vowels (bytes > 127) are never counted; only the ASCII 'o' counts.
        assert_eq!(count_vowels("héllo").unwrap(), 1);
    }

    // --- Interior-NUL input returns Error::Conversion (Req 7.5) ---

    #[test]
    fn interior_nul_yields_conversion_error() {
        assert!(matches!(str_length("a\0b"), Err(Error::Conversion(_))));
        assert!(matches!(count_vowels("a\0b"), Err(Error::Conversion(_))));
        assert!(matches!(str_reverse("a\0b"), Err(Error::Conversion(_))));
        assert!(matches!(to_uppercase("a\0b"), Err(Error::Conversion(_))));
    }

    // --- str_length equals input byte length for representative inputs (Req 7.10) ---

    #[test]
    fn length_equals_byte_length() {
        for input in ["", "a", "hello", "hello world", "héllo", "世界", "🦀 rust"] {
            assert_eq!(
                str_length(input).unwrap(),
                input.as_bytes().len(),
                "byte-length mismatch for {input:?}"
            );
        }
    }
}
