# Implementation Plan: rust-cpp-ffi-wrapper

## Overview

This plan builds a Rust crate that wraps a minimal C++ string-utilities library through a
C-compatible FFI boundary. Work proceeds strictly in dependency order: the Cargo scaffold
first, then the native C++ layer and its C header, then the `build.rs` that compiles and
generates bindings, then the single unsafe FFI module, the error type, the safe API, the CLI
demo, the full test suite (unit + integration + property-based + leak/stress), and finally
the Docker environment and documentation. Each step builds on the prior ones and is verified
incrementally (structure compiles, bindings smoke-check after `build.rs`, safe-API tests
after the API, leak/stress tests after the string wrappers), so no code is left orphaned.

The design includes a Correctness Properties section (Properties 1–7), so property-based
tests with `proptest` are included as sub-tasks and are placed close to the code they
validate. Test-related sub-tasks are marked optional with `*`.

## Tasks

- [x] 1. Scaffold the Cargo project structure
  - Create `Cargo.toml` with `[package]` (edition 2021), `[lib]` (`name = "stringutils"`, `path = "src/lib.rs"`), `[[bin]]` (`name = "stringutils-cli"`, `path = "src/main.rs"`), `[build-dependencies]` `bindgen` and `cc` (pinned exact minor versions), and `[dev-dependencies]` `proptest`
  - Create placeholder `build.rs` (empty `fn main() {}`), and placeholder module files `src/lib.rs`, `src/main.rs`, `src/ffi.rs`, `src/error.rs`, plus empty `cpp/` and `tests/` directories, so the crate layout is complete and structurally coherent
  - _Requirements: 4.1, 5.1, 6.1, 7.8_

- [x] 2. Implement the C++ string-utility library and C-compatible header
  - [x] 2.1 Write the C-compatible header `cpp/wrapper.h`
    - Declare all five functions inside `extern "C"` guards with FFI-compatible C types: `size_t str_length(const char*)`, `size_t count_vowels(const char*)`, `char* str_reverse(const char*)`, `char* to_uppercase(const char*)`, `void free_string(char*)`; include `<stddef.h>` for `size_t`; use include guards
    - _Requirements: 2.1, 2.2, 2.3, 2.5_

  - [x] 2.2 Implement the C++ library `cpp/lib.cpp`
    - Implement all five functions matching the header declarations with C linkage; `str_length` returns byte count before the NUL; `count_vowels` counts ASCII vowels case-insensitively; `str_reverse` returns a heap copy in reverse byte order; `to_uppercase` returns a heap ASCII-uppercased copy; allocate returned strings with `new (std::nothrow) char[n+1]` returning null on failure; return a one-byte NUL-only buffer for empty input; leave bytes > 127 unchanged for uppercase and vowel counting; `free_string` releases with matching `delete[]` and is a no-op on null
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7, 1.8, 1.9, 1.10, 1.11, 2.3, 2.4_

- [x] 3. Implement the build script (`build.rs`)
  - Compile the C++ with `cc::Build::new().cpp(true).file("cpp/lib.cpp").include("cpp").compile("stringutils")` so Cargo links `libstringutils.a` automatically
  - Generate bindings with `bindgen::Builder` on `cpp/wrapper.h` using clang args `-x c++`, `allowlist_function` for the five functions, `CargoCallbacks`, writing to `$OUT_DIR/bindings.rs`; use `.expect(...)` on `generate()` and `write_to_file(...)` so any failure panics with a diagnostic and a non-zero build exit and no bindings are written on parse failure
  - Emit `cargo:rerun-if-changed=cpp/lib.cpp` and `cargo:rerun-if-changed=cpp/wrapper.h`
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7_

- [x] 4. Implement the FFI module (`src/ffi.rs`)
  - Add the module-level doc comment describing the safety boundary invariants (valid NUL-terminated pointers in, never dereference a returned null, free every non-null return exactly once) and the necessary `#![allow(...)]` attributes (`non_upper_case_globals`, `non_camel_case_types`, `non_snake_case`, `dead_code`)
  - `include!(concat!(env!("OUT_DIR"), "/bindings.rs"))` so this is the only unsafe boundary in the crate
  - _Requirements: 3.1, 3.2, 5.3_

  - [x]* 4.1 Add an FFI binding smoke check
    - Add a minimal test (behind `#[cfg(test)]`) that calls `ffi::str_length` on a known NUL-terminated pointer inside an `unsafe` block and asserts the expected count, confirming compilation, linkage, and binding generation succeeded end-to-end
    - _Requirements: 3.1, 3.2, 4.6_

- [x] 5. Implement the error type (`src/error.rs`)
  - Define `pub enum Error` with `Conversion(NulError)` and `NullResult` variants; implement `Display`, `std::error::Error` (with `source()` returning the wrapped `NulError` for `Conversion`), and `From<NulError>` so the safe API can use `?`
  - _Requirements: 5.5, 5.9, 5.10_

- [x] 6. Implement the safe Rust API (`src/lib.rs`)
  - [x] 6.1 Add module wiring, the `to_cstring` helper, and the `FreeGuard` RAII type
    - Declare `mod ffi; mod error;` and re-export `Error`; implement internal `fn to_cstring(&str) -> Result<CString, Error>` mapping interior NUL to `Error::Conversion` via `?`; implement `struct FreeGuard(*mut c_char)` whose `Drop` calls `ffi::free_string` exactly once so the C allocation is freed on every path (normal, early return, panic unwind)
    - _Requirements: 5.4, 5.5, 5.7, 5.11_

  - [x] 6.2 Implement the length and vowel-count wrappers
    - Implement `pub fn str_length(&str) -> Result<usize, Error>` and `pub fn count_vowels(&str) -> Result<usize, Error>`: convert via `to_cstring`, call the FFI function inside the crate's only-permitted internal `unsafe`, return the `usize` count directly (no allocation crosses back); no `unsafe` in the signatures; add doc comments with runnable doctests and note the ASCII limitation and interior-NUL behavior
    - _Requirements: 5.1, 5.2, 5.4, 9.1_

  - [x] 6.3 Implement the string-returning wrappers
    - Implement `pub fn str_reverse(&str) -> Result<String, Error>` and `pub fn to_uppercase(&str) -> Result<String, Error>`: convert via `to_cstring`, call the FFI function, return `Err(Error::NullResult)` on a null return without dereferencing, wrap the non-null pointer in `FreeGuard`, copy the bytes into an owned `String` via `CStr`, and let the guard free the C allocation exactly once as the function returns; no `unsafe` in the signatures; add doc comments with runnable doctests documenting the ASCII limitation and interior-NUL behavior
    - _Requirements: 5.1, 5.2, 5.6, 5.7, 5.8, 5.10, 5.11, 5.12, 9.1_

  - [x]* 6.4 Write unit tests in a `#[cfg(test)] mod tests` block
    - Cover representative valid inputs, empty string, single-character input, special-character/multi-byte Unicode input (e.g. `"héllo 世界!"`), and interior-NUL input returning `Error::Conversion` for each of the four functions; assert `str_length` equals the input byte length for representative inputs
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.8, 7.10_

- [x] 7. Implement the CLI demo binary (`src/main.rs`)
  - Parse arguments accepting exactly one string argument; on success invoke all four safe-API operations and print one labeled `key: value` line per operation, exiting with a zero status; print usage to stderr and exit non-zero when the argument count is wrong; on a `Conversion` error print the error to stderr and exit non-zero without printing any operation result; use only the safe API (no `unsafe`)
  - _Requirements: 6.1, 6.2, 6.3, 6.4_

- [x] 8. Checkpoint - verify build, bindings, and unit/doctest suite
  - Ensure the crate builds (C++ compiles, bindings generate and link), doctests and unit tests pass. Ensure all tests pass, ask the user if questions arise.

- [ ] 9. Implement property-based tests for the correctness properties
  - [ ]* 9.1 Add proptest generators and Property 1 (length)
    - Add a NUL-free string generator (including ASCII, empty, single-char, special, and multi-byte / >127 sequences) configured for at least 100 iterations; implement Property 1 as a single test
    - **Property 1: Length equals host byte length** — for any NUL-free string `s`, `str_length(s) == Ok(s.as_bytes().len())`
    - Tag: `// Feature: rust-cpp-ffi-wrapper, Property 1: Length equals host byte length`
    - **Validates: Requirements 1.1, 1.7, 7.10**

  - [ ]* 9.2 Add Property 2 (reverse involution)
    - **Property 2: Reverse is an involution** — for any NUL-free string `s`, `str_reverse(str_reverse(s)) == Ok(s)` and `str_reverse(s)` equals the host byte reversal
    - Tag: `// Feature: rust-cpp-ffi-wrapper, Property 2: Reverse is an involution`
    - **Validates: Requirements 1.2, 1.8**

  - [ ]* 9.3 Add Property 3 (vowel count vs host model)
    - **Property 3: Vowel count equals the host ASCII model** — for any NUL-free string `s`, `count_vowels(s)` equals an independent host-side ASCII vowel count, never counting bytes > 127
    - Tag: `// Feature: rust-cpp-ffi-wrapper, Property 3: Vowel count equals the host ASCII model`
    - **Validates: Requirements 1.3, 1.5**

  - [ ]* 9.4 Add Property 4 (uppercase vs host model)
    - **Property 4: Uppercase equals the host ASCII model** — for any NUL-free string `s`, `to_uppercase(s)` equals the host-side ASCII-uppercase transform, leaving bytes > 127 unchanged
    - Tag: `// Feature: rust-cpp-ffi-wrapper, Property 4: Uppercase equals the host ASCII model`
    - **Validates: Requirements 1.4, 1.5**

  - [ ]* 9.5 Add Property 5 (ASCII case round-trip)
    - **Property 5: ASCII case round-trip** — for any string `s` of only ASCII lowercase letters, `to_uppercase(s).map(|u| u.to_lowercase()) == Ok(s)`
    - Tag: `// Feature: rust-cpp-ffi-wrapper, Property 5: ASCII case round-trip`
    - **Validates: Requirements 7.9, 1.4**

  - [ ]* 9.6 Add Property 6 (interior NUL yields Conversion error)
    - **Property 6: Interior NUL always yields a Conversion error** — for any string with at least one interior NUL, every safe-API function returns `Err(Error::Conversion)` and does not call the FFI layer
    - Tag: `// Feature: rust-cpp-ffi-wrapper, Property 6: Interior NUL always yields a Conversion error`
    - **Validates: Requirements 5.5**

- [ ] 10. Implement integration, stress, and leak tests (`tests/integration.rs`)
  - [ ]* 10.1 Add the stress / no-panic test (Property 7)
    - Exercise each safe-API function over at least 10,000 generated NUL-free inputs and assert completion without panicking, confirming exactly-once frees under sustained execution
    - **Property 7: Repeated invocation never panics** — for any sequence of at least 10,000 NUL-free inputs, repeated invocation completes without panic and frees every C allocation exactly once
    - Tag: `// Feature: rust-cpp-ffi-wrapper, Property 7: Repeated invocation never panics`
    - **Validates: Requirements 7.6, 5.11**

  - [ ]* 10.2 Add the valgrind-runnable leak-detection test
    - Add an integration test that performs many `str_reverse`/`to_uppercase` calls (each allocating in C++ and freeing via `FreeGuard`) so it can be run under `valgrind --leak-check=full --error-exitcode=1` to confirm zero leaked bytes and zero invalid frees
    - _Requirements: 7.7, 5.11_

- [ ] 11. Checkpoint - verify the full test suite
  - Ensure `cargo test` runs the unit, doctest, integration, property, and stress tests successfully. Ensure all tests pass, ask the user if questions arise.

- [ ] 12. Create the Docker reproducible build/test environment
  - [ ] 12.1 Write the `Dockerfile` and `.dockerignore`
    - Base on a version-pinned Rust toolchain image; install pinned LLVM/Clang (`libclang`, for `bindgen`) and `g++` (for `cc`); set `LIBCLANG_PATH`; set the default command to compile the crate then run `cargo test` in a single chained invocation (`cargo build && cargo test`) so a compilation failure halts before tests with a non-zero exit and a green run exits zero; add `.dockerignore` excluding `target/`, `.git/`, and other local artifacts
    - _Requirements: 8.1, 8.2, 8.3, 8.5, 8.6_

  - [ ] 12.2 Write `docker-compose.yml`
    - Define a single service that builds the image and runs the build+test command so one command (`docker compose run --rm test`) compiles the crate and runs the suite without any host-installed toolchain, propagating the suite exit status
    - _Requirements: 8.4, 8.2, 8.3_

- [ ] 13. Write project documentation (`README.md`)
  - Document setup and the `build.rs`-driven build process; state build requirements (C++ compiler, bindgen / LLVM-Clang); describe how to run the CLI demo and the Docker environment; state the per-allocation ownership model (C++ allocates and frees returned strings via `free_string`; Rust copies into an owned `String`); state that uppercase and vowel counting are ASCII-defined with non-ASCII bytes passing through unchanged; add the interior-NUL / Conversion_Error usage note; include a 200–300 word design-choices explanation required by the evaluation
  - _Requirements: 9.2, 9.3, 9.4, 9.5, 9.6, 9.7_

- [ ] 14. Final checkpoint - verify the whole suite via cargo and Docker
  - Confirm `cargo build` and `cargo test` pass on the host, and that the Docker environment builds the crate and runs the full suite to a zero exit in a single invocation. Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional test sub-tasks and can be skipped for a faster MVP; core implementation tasks are never optional.
- Each task references specific requirement sub-clauses for traceability.
- Property tests use `proptest` (minimum 100 iterations each; Property 7 uses at least 10,000) and each is tagged with its `Feature: rust-cpp-ffi-wrapper, Property {n}` comment referencing the design's Correctness Properties.
- All `unsafe` is confined to `src/ffi.rs` and the `FreeGuard::drop` impl; no public signature contains `unsafe`.
- Checkpoints ensure incremental validation after the build/bindings stage, after the safe API and its tests, and after the full suite and Docker environment.

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1"] },
    { "id": 1, "tasks": ["2.1"] },
    { "id": 2, "tasks": ["2.2"] },
    { "id": 3, "tasks": ["3"] },
    { "id": 4, "tasks": ["4", "5"] },
    { "id": 5, "tasks": ["4.1", "6.1"] },
    { "id": 6, "tasks": ["6.2", "6.3"] },
    { "id": 7, "tasks": ["6.4", "7"] },
    { "id": 8, "tasks": ["9.1", "9.2", "9.3", "9.4", "9.5", "9.6"] },
    { "id": 9, "tasks": ["10.1", "10.2"] },
    { "id": 10, "tasks": ["12.1", "13"] },
    { "id": 11, "tasks": ["12.2"] }
  ]
}
```
