# rust-cpp-ffi-wrapper

A Rust safe wrapper around a minimal C++ string-utilities library, demonstrating
cross-language FFI. This project implements **Option A** (string utilities): four small
operations (`str_length`, `count_vowels`, `str_reverse`, `to_uppercase`, plus a
`free_string` deallocator) are implemented in C++ behind a C-compatible header, compiled and
bound at build time, and exposed through an idiomatic Rust API. Its defining property is a
clean separation between a single `unsafe` FFI layer and a public API that is 100% safe: no
public signature contains the `unsafe` keyword, every allocation has an explicit owner, and
every fallible operation returns a `Result`. String operations were chosen deliberately —
two of the four return freshly heap-allocated C strings, which forces an honest demonstration
of manual heap-memory management across a language boundary rather than trivially copyable
scalars.

## Features

The safe public API (crate `stringutils`) exposes exactly four functions:

```rust
pub fn str_length(input: &str)  -> Result<usize, Error>;
pub fn count_vowels(input: &str) -> Result<usize, Error>;
pub fn str_reverse(input: &str)  -> Result<String, Error>;
pub fn to_uppercase(input: &str) -> Result<String, Error>;
```

- `str_length` — byte length of the input, excluding the terminating NUL (this is the UTF-8
  byte length, matching `input.as_bytes().len()`, not the `char` count).
- `count_vowels` — count of ASCII vowels (`a e i o u`, case-insensitive) as a byte count.
- `str_reverse` — the input bytes in reverse byte order, returned as an owned `String`.
- `to_uppercase` — the ASCII-uppercase form of the input, returned as an owned `String`.

`str_length` and `count_vowels` return byte counts (`usize`); `str_reverse` and
`to_uppercase` return owned `String`s that the caller owns and drops.

## Project structure

```text
rust-c-bridge/
├── Cargo.toml            # [lib] stringutils + [[bin]] stringutils-cli; build-deps bindgen & cc; dev-dep proptest
├── build.rs              # cc compiles cpp/lib.cpp; bindgen generates bindings; rerun-if-changed directives
├── cpp/
│   ├── wrapper.h         # extern "C" declarations of the five functions (the C header)
│   └── lib.cpp           # C++ implementations with C linkage (the C library)
├── src/
│   ├── ffi.rs            # include!s the generated bindings — the ONLY unsafe boundary
│   ├── error.rs          # Error enum (Conversion, NullResult) + Display / Error / From<NulError>
│   ├── lib.rs            # safe API, to_cstring helper, FreeGuard RAII type, unit tests
│   └── main.rs           # stringutils-cli demo binary (safe API only)
├── tests/
│   ├── properties.rs     # proptest property tests (Properties 1–6, ≥100 iterations each)
│   └── integration.rs    # stress no-panic test (Property 7) + valgrind leak-detection test
├── Dockerfile            # pinned Rust + clang/libclang + g++ + valgrind; default build+test command
├── docker-compose.yml    # `test` (build+test) and `dev` (interactive shell) services
├── .dockerignore         # keeps the build context small (target/, .git/, editor artifacts)
├── valgrind.supp         # scoped suppressions for benign libtest/std-runtime blocks only
└── scripts/valgrind.sh   # runs the leak-detection test under valgrind inside the container
```

## Build & test

There are two first-class paths. The original evaluation asks for the test suite to pass via
`cargo test`; Docker is provided as the zero-setup, reproducible way to run that same suite
without installing anything but Docker.

### A) Docker (recommended — zero host setup)

No host Rust, clang/libclang, or g++ needed. Only Docker is required. From the workspace
root:

```bash
# Compile the crate and run the full test suite in one invocation.
# Exits 0 on success, non-zero if compilation or any test fails.
docker compose run --rm test

# Interactive shell for iterative work (source is bind-mounted).
docker compose run --rm dev

# Run the exact command the evaluation names (`cargo test`) inside the container.
docker compose run --rm dev bash -c 'cargo test'
```

The image bakes in a pinned toolchain for reproducibility (Debian bookworm base):

| Tool             | Pinned version | Why                                  |
|------------------|----------------|--------------------------------------|
| Rust             | 1.90.0         | crate toolchain (edition 2021)       |
| clang / libclang | 14             | required by `bindgen`                |
| g++              | 12             | required by the `cc` crate for C++   |
| valgrind         | 3.19           | leak-detection test                  |

### B) Native cargo

This is the command the evaluation names. It requires a host toolchain:

- A Rust toolchain (edition 2021; tested with 1.90).
- A C++ compiler (`g++` or `clang`) for the `cc` crate to compile `cpp/lib.cpp`.
- `libclang` for `bindgen` to parse `cpp/wrapper.h`. On some systems you may need to point
  `bindgen` at it, e.g. `export LIBCLANG_PATH=/usr/lib/llvm-14/lib`.

These must be installed on the host — there is no way around it for the native path.

```bash
cargo build
cargo test
cargo run -- "hello world"   # run the CLI demo
```

This native path was validated inside the Linux container; a host with the tools above
reproduces it.

## CLI demo

The `stringutils-cli` binary runs all four operations on a single argument and prints one
labeled line per operation:

```bash
cargo run -- "hello world"
# or in Docker:
docker compose run --rm dev bash -c 'cargo run -- "hello world"'
```

Output:

```text
length: 11
vowels: 3
reversed: dlrow olleh
uppercase: HELLO WORLD
```

Error / usage behavior:

- Running with no argument, or more than one argument, prints usage to stderr and exits with
  a non-zero status.
- An input containing an interior NUL byte prints an error to stderr and exits non-zero
  without printing any operation results (all values are computed before anything is
  printed, so the error path prints nothing to stdout).

## How the build works

`build.rs` does two jobs during `cargo build`, and fails the whole build if either fails:

1. **Compile + link the C++.** The `cc` crate compiles `cpp/lib.cpp` in C++ mode
   (`.cpp(true)`) into a static library `libstringutils.a`, with `cpp/` on the include path,
   and emits the link directives so Cargo links it automatically.
2. **Generate bindings.** `bindgen` parses `cpp/wrapper.h` as C++ (`-x c++`), allowlisting
   the five functions, and writes Rust FFI declarations to `$OUT_DIR/bindings.rs`, which
   `src/ffi.rs` pulls in with `include!`.

`build.rs` also emits `cargo:rerun-if-changed=cpp/lib.cpp` and
`cargo:rerun-if-changed=cpp/wrapper.h`, so editing the C++ sources triggers a rebuild on the
next `cargo build`.

## Memory management / ownership model

The hard part of FFI is deciding, for every allocation, which side owns it and which side
frees it. The rule here is simple: **C++ allocates and C++ frees** every returned string
(matching `new[]`/`delete[]`); **Rust copies** the bytes into an owned `String`. Rust never
frees a C++ allocation with `libc::free` or Rust's allocator.

| Allocation                          | Allocated by    | Owned by (until freed)        | Freed by                                          |
|-------------------------------------|-----------------|-------------------------------|---------------------------------------------------|
| Input `&str` → `CString`            | Rust            | Rust                          | Rust (dropped at scope end)                       |
| `str_reverse` / `to_uppercase` return `char*` | C++ heap (`new[]`) | C++ heap, held via `FreeGuard` | C++ `free_string` (`delete[]`) via `FreeGuard::drop` |
| Returned owned `String`             | Rust (copy)     | Caller                        | Rust (caller drops)                               |
| `usize` count returns               | n/a (by value)  | n/a                           | n/a                                               |

For the two string-returning functions, Rust receives a raw `*mut c_char` from C++, copies
the bytes into an owned `String` (via `CStr`), and then frees the original C++ allocation
through the C-provided `free_string`. The copy happens *before* the free, so the returned
`String` never borrows freed memory.

The **exactly-once free** is guaranteed by a `FreeGuard` RAII type: a non-null returned
pointer is immediately moved into a `FreeGuard`, whose `Drop` impl calls `free_string` on
every return path — normal return, early `?` return, and unwinding during a panic.
`free_string` is a no-op on null, so the guard is sound regardless.

## ASCII semantics

`to_uppercase` and `count_vowels` are **ASCII-defined**. Any input byte greater than 127
passes through unchanged, and non-ASCII "vowels" (such as `é`) are never counted. For
example:

```rust
assert_eq!(stringutils::to_uppercase("héllo").unwrap(), "HéLLO");
assert_eq!(stringutils::count_vowels("héllo").unwrap(), 1); // only the ASCII 'o'
```

## Interior NUL / error behavior

Every function first converts its `&str` to a C string. An input containing an **interior
NUL byte** cannot be represented as a C string, so the conversion fails and the function
returns `Err(Error::Conversion(_))` — the FFI layer is not called at all. Separately, if a
string-producing C function returns a null pointer (allocation failure), the safe API returns
`Err(Error::NullResult)` without ever dereferencing the pointer.

The `Error` type has exactly two honest failure modes:

```rust
pub enum Error {
    Conversion(NulError), // input had an interior NUL; FFI not called
    NullResult,           // C library returned null (allocation failure); not dereferenced
}
```

```rust
assert!(stringutils::str_length("a\0b").is_err()); // Error::Conversion
```

## Safety design

All `unsafe` in the crate is confined to two places: `src/ffi.rs` (the generated bindings and
their call sites) and `FreeGuard::drop`. No public signature contains the `unsafe` keyword —
the public API is 100% safe. The `&str` → C-string conversion, the null check, and the
copy-into-`String` all happen in safe code around the small internal `unsafe` blocks.

## Testing

The suite (run by `cargo test`) has four layers:

- **Unit tests** (`src/lib.rs`, `src/error.rs`, `src/ffi.rs`): correctness for representative
  inputs plus edge cases — empty string, single character, multibyte/high-byte Unicode
  (`"héllo 世界!"`), and interior-NUL inputs returning `Error::Conversion`. Includes an FFI
  binding smoke check that proves the C++ ↔ bindgen ↔ link chain works end-to-end.
- **Doctests**: the runnable usage example on each of the four public functions.
- **Property tests** (`tests/properties.rs`): six `proptest` properties (Properties 1–6),
  each running at least 100 iterations — length equals host byte length, reverse is an
  involution, vowel count and uppercase match independent host-side ASCII models, ASCII case
  round-trip, and interior-NUL always yields a `Conversion` error.
- **Integration safety tests** (`tests/integration.rs`): a ≥10,000-invocation no-panic stress
  test (the executable form of Property 7) that exercises the allocate/copy/free path under
  load, and a valgrind-runnable leak-detection test.

### Leak check (valgrind — Docker / Linux)

The leak check runs under valgrind inside the container:

```bash
docker compose run --rm dev bash scripts/valgrind.sh
```

Our FFI allocation path reports **"definitely lost: 0 bytes"**. `valgrind.supp` contains
scoped suppressions for benign libtest/std-runtime blocks only (never our FFI code), so a
clean run exits 0. Valgrind support on macOS/Apple Silicon is limited, so on a native Mac use
the Docker path for the leak check.

## Design Choices

I chose **Option A (string utilities)** over a numeric alternative precisely because two of
its operations return heap-allocated C strings. That forces a real demonstration of
cross-language memory management — the genuinely hard part of FFI — instead of copying
scalars that carry no ownership.

The architecture is a strict one-directional layering: C++ implementation → `build.rs`
(compile + bind) → `ffi.rs` (the only `unsafe`) → safe API → CLI. Confining every raw
declaration and call to `ffi.rs`, plus `FreeGuard::drop`, is what lets the crate guarantee no
public signature is `unsafe`. Each module has a single responsibility, which keeps that
boundary enforceable.

For memory, the model is copy-then-free: Rust copies the C bytes into an owned `String`, then
frees the original C++ buffer via the C-provided `free_string`, matching C++'s
`new[]`/`delete[]` so allocators never mismatch. A `FreeGuard` RAII type owns the raw pointer
and frees it exactly once on every path — normal return, early return, and panic unwind —
which is the crux of leak-free FFI.

Errors are surfaced through `Result` with two honest failure modes: `Conversion` for interior
NULs (caught before the FFI layer) and `NullResult` for allocation failure (caught without
dereferencing). ASCII-defined uppercase and vowel counting are a documented limitation:
bytes above 127 pass through unchanged, which keeps the C++ simple and the behavior
predictable. Finally, Docker with a fully pinned toolchain (Rust, clang, g++, valgrind) makes
the build and the whole test suite reproducible on any machine with zero host setup.
