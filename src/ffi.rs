//! Raw FFI bindings to the C++ string-utilities library.
//!
//! # Safety boundary
//! This module is the ONLY place that exposes the raw C symbols. Every item here is
//! `unsafe` to call. Callers (the safe API in `lib.rs`) are responsible for upholding
//! the boundary invariants:
//! - pass valid, NUL-terminated `*const c_char` pointers into the string operations,
//! - never dereference a returned null pointer (a null return signals allocation failure),
//! - free every non-null returned pointer exactly once via `free_string`, which uses the
//!   allocator matching the C++ `new[]`/`delete[]` pair.
//!
//! Confining the raw declarations here is what lets the crate guarantee that no `unsafe`
//! appears in any public signature: this is the single unsafe boundary.
#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, dead_code)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    // Smoke check: proves the C++ <-> bindgen <-> link chain works end-to-end.
    // If the static library were not linked, this would fail to link with an
    // undefined symbol for `str_length`.
    #[test]
    fn str_length_binding_links_and_counts() {
        let c = CString::new("hello").unwrap();
        let n = unsafe { str_length(c.as_ptr()) };
        assert_eq!(n, 5);
    }
}
