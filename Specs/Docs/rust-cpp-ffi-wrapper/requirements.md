# Requirements Document

## Introduction

This feature delivers a Rust wrapper around a minimal C++ string-utilities library, demonstrating safe cross-language interoperability (FFI). The C++ layer exposes four string operations plus an explicit deallocation function through a C-compatible interface. A build script compiles the C++ sources with the `cc` crate and generates Rust bindings with `bindgen`. A safe, idiomatic Rust API wraps the generated unsafe bindings so that no `unsafe` appears in any public signature, ownership of every allocation is explicit, and errors are surfaced through `Result`. The deliverable also includes a CLI demo binary, integration tests that verify both correctness and memory safety, a Docker-based reproducible build/test environment, and documentation covering setup, usage, and memory-management decisions.

The primary quality goal is a clean separation between the `unsafe` FFI layer and the safe public API, with sound resource management (no leaks, no undefined behavior) and thoughtful error handling.

## Glossary

- **C_Library**: The minimal C++ library that implements the four string-utility functions and the deallocation function, compiled from C++ source into a static library.
- **C_Header**: The C-compatible header file that declares the `C_Library` functions with C linkage for `bindgen` to parse.
- **Bindgen_Generator**: The `bindgen` tooling invoked during the build to translate `C_Header` into Rust FFI declarations.
- **Build_Script**: The `build.rs` program that orchestrates C++ compilation, include-path and flag configuration, rebuild triggers, linking, and binding generation.
- **CC_Compiler**: The `cc` crate used by the `Build_Script` to compile the C++ source files.
- **FFI_Layer**: The generated Rust bindings plus the internal `unsafe` code that calls into the `C_Library`.
- **Safe_API**: The public, idiomatic Rust module whose function signatures contain no `unsafe` keyword.
- **CLI_Demo**: The runnable binary (`src/main.rs`) that exercises the `Safe_API` and prints results.
- **Test_Suite**: The collection of integration and unit tests executed by `cargo test`.
- **Docker_Environment**: The container image and `docker-compose.yml` providing a reproducible build and test toolchain (Rust, LLVM/Clang, C++ compiler).
- **Owned_String**: A Rust `String` that owns its memory, produced by copying data returned from the `C_Library`.
- **Deallocation_Function**: The C++ `free_string` function that releases memory allocated by the `C_Library`.
- **Interior_NUL**: A NUL (`\0`) byte occurring within the body of an input string, which is invalid for C string conversion.
- **Conversion_Error**: The error value returned by the `Safe_API` when an input string cannot be converted to a C-compatible string.

## Requirements

### Requirement 1: C++ String-Utility Library

**User Story:** As a systems developer, I want a minimal C++ library that provides core string operations, so that the Rust wrapper has native functionality to bridge.

#### Acceptance Criteria

1. WHEN provided a NUL-terminated C string, THE C_Library SHALL return the length of the string as a non-negative count of bytes preceding the terminating NUL, excluding the terminating NUL.
2. WHEN provided a NUL-terminated C string, THE C_Library SHALL return a newly allocated NUL-terminated C string containing the input bytes preceding the terminating NUL in reverse byte order, excluding the terminating NUL from the reversal.
3. WHEN provided a NUL-terminated C string, THE C_Library SHALL return the non-negative count of vowel characters (`a`, `e`, `i`, `o`, `u`, case-insensitive, ASCII) among the input bytes preceding the terminating NUL.
4. WHEN provided a NUL-terminated C string, THE C_Library SHALL return a newly allocated NUL-terminated C string containing the ASCII-uppercase form of the input bytes preceding the terminating NUL.
5. WHERE a function of the C_Library converts characters to uppercase or counts vowels, THE C_Library SHALL leave input bytes whose value is greater than 127 unchanged.
6. WHERE a function of the C_Library returns a newly allocated string, THE C_Library SHALL allocate that string on the heap and transfer deallocation responsibility to the caller.
7. WHEN provided an empty NUL-terminated C string, THE C_Library length function SHALL return 0.
8. WHEN provided an empty NUL-terminated C string, THE C_Library SHALL return, for each string-returning function, a newly allocated empty string consisting of only a terminating NUL.
9. IF allocation for a returned string fails, THEN THE C_Library SHALL return a null pointer and leave the input unmodified.
10. THE C_Library SHALL provide a Deallocation_Function that releases memory previously allocated by the C_Library using the matching allocator.
11. WHEN given a null pointer, THE Deallocation_Function SHALL perform no operation.

### Requirement 2: C-Compatible Header and Linkage

**User Story:** As a developer configuring FFI, I want a C-compatible header with unmangled symbols, so that bindgen can parse the interface and Rust can link against it.

#### Acceptance Criteria

1. THE C_Header SHALL declare each public function of the C_Library using C linkage so that name mangling is prevented.
2. THE C_Header SHALL declare function parameters and return values using C types compatible with FFI, and THE C_Header SHALL parse cleanly for binding generation.
3. THE C_Library SHALL define each public function with C linkage matching the corresponding declaration in the C_Header.
4. THE C_Header and the C_Library SHALL correspond such that every function declared in the C_Header is defined in the C_Library and every public function defined in the C_Library is declared in the C_Header.
5. THE C_Header SHALL declare the Deallocation_Function with a signature that accepts a pointer previously returned by any C_Library string-producing function.

### Requirement 3: Binding Generation

**User Story:** As a Rust developer, I want bindings generated automatically from the C header, so that I can call the C++ functions from Rust without hand-writing declarations.

#### Acceptance Criteria

1. WHEN the Build_Script runs, THE Bindgen_Generator SHALL parse the C_Header and produce one Rust FFI declaration for each function declared in the C_Header.
2. THE Bindgen_Generator SHALL map each C type declared in the C_Header to its corresponding Rust FFI type such that no C type in the C_Header is left unmapped.
3. WHEN the Build_Script runs and the Bindgen_Generator completes parsing successfully, THE Build_Script SHALL write the generated bindings to a location that the crate compiles into the FFI_Layer during the same build invocation.
4. IF the Bindgen_Generator cannot parse the C_Header, THEN THE Build_Script SHALL terminate with a non-zero exit status without writing generated bindings.
5. IF the Bindgen_Generator cannot parse the C_Header, THEN THE Build_Script SHALL emit a diagnostic message indicating the parse failure and the C_Header that failed.
6. IF the C_Header declares a C type that the Bindgen_Generator cannot map to a Rust FFI type, THEN THE Build_Script SHALL terminate with a non-zero exit status and emit a diagnostic message identifying the unmapped C type.

### Requirement 4: Build System Integration

**User Story:** As a developer building the project, I want the build script to compile and link the C++ library automatically, so that `cargo build` produces a working artifact without manual steps.

#### Acceptance Criteria

1. WHEN the Build_Script runs during a `cargo build` invocation, THE CC_Compiler SHALL compile all C++ source files of the C_Library without requiring any manual step beyond the single `cargo build` command.
2. THE Build_Script SHALL configure the include paths such that every C++ source file of the C_Library compiles without unresolved-include errors and the C_Header is parsed without unresolved-include errors.
3. THE Build_Script SHALL configure the compiler flags such that the C++ sources are compiled in C++ mode without language-mode compilation errors.
4. THE Build_Script SHALL emit one `cargo:rerun-if-changed` directive for each C++ source file of the C_Library and one `cargo:rerun-if-changed` directive for the C_Header.
5. WHEN any C++ source file or the C_Header referenced by a `cargo:rerun-if-changed` directive is modified between builds, THE Build_Script SHALL re-run and recompile the affected C++ sources on the next `cargo build`.
6. WHEN all C++ sources compile successfully, THE Build_Script SHALL link the resulting compiled library into the Rust crate and complete with a zero exit status, producing the linked build artifact.
7. IF C++ compilation or linking fails, THEN THE Build_Script SHALL terminate with a non-zero exit status, surface the compiler or linker diagnostic output to the caller, and produce no linked build artifact.

### Requirement 5: Safe Rust Wrapper API

**User Story:** As a Rust consumer of this crate, I want a safe idiomatic API, so that I can use the string utilities without writing unsafe code or managing C memory.

#### Acceptance Criteria

1. THE Safe_API SHALL expose one public function for each of the four string operations (length, reverse, vowel count, uppercase).
2. THE Safe_API SHALL declare every public function signature without the `unsafe` keyword.
3. THE FFI_Layer SHALL confine every `unsafe` block to internal implementation code that is not part of a public signature.
4. WHEN the Safe_API receives a `&str` input, THE Safe_API SHALL convert the input to a C-compatible string before calling the FFI_Layer.
5. IF an input `&str` contains an Interior_NUL, THEN THE Safe_API SHALL return a `Result` Err containing a Conversion_Error and SHALL NOT call the FFI_Layer.
6. WHEN a C_Library function returns a non-null pointer, THE Safe_API SHALL copy the returned data into an Owned_String.
7. WHEN the Safe_API has copied the data returned by a C_Library function into an Owned_String, THE Safe_API SHALL release the C++ allocation using the Deallocation_Function after copying and before returning.
8. THE Safe_API SHALL return string-producing operations as an Owned_String owned by the caller.
9. THE Safe_API SHALL return fallible operations as a `Result` whose error variant indicates the failure cause.
10. IF a C_Library string-producing function returns a null pointer, THEN THE Safe_API SHALL return a `Result` Err indicating the failure and SHALL NOT dereference the null pointer.
11. THE Safe_API SHALL release every allocation obtained from the C_Library exactly once.
12. WHERE the Safe_API returns borrowed data, THE Safe_API SHALL bound the borrow to the lifetime of the source data.

### Requirement 6: CLI Demo Binary

**User Story:** As an evaluator, I want a runnable command-line demo, so that I can exercise the safe API and see results without writing my own driver.

#### Acceptance Criteria

1. WHEN the CLI_Demo is run with exactly one input string argument, THE CLI_Demo SHALL invoke each Safe_API string operation on that argument.
2. WHEN the Safe_API string operations complete successfully for the provided input, THE CLI_Demo SHALL print the result produced by each invoked operation and exit with a zero status.
3. IF the CLI_Demo is run with no input string argument, THEN THE CLI_Demo SHALL print usage guidance and exit with a non-zero status.
4. IF the Safe_API returns a Conversion_Error for the provided input, THEN THE CLI_Demo SHALL print an error message identifying the conversion failure and exit with a non-zero status, without printing any operation result.

### Requirement 7: Testing

**User Story:** As a maintainer, I want comprehensive tests that verify correctness and safety, so that I can trust the wrapper does not leak memory or exhibit undefined behavior.

#### Acceptance Criteria

1. THE Test_Suite SHALL verify that each Safe_API function produces the expected output for representative valid inputs.
2. THE Test_Suite SHALL verify Safe_API behavior for empty-string input.
3. THE Test_Suite SHALL verify Safe_API behavior for single-character input.
4. THE Test_Suite SHALL verify Safe_API behavior for input containing special characters and multi-byte Unicode.
5. THE Test_Suite SHALL verify that an input containing an Interior_NUL produces a Conversion_Error.
6. THE Test_Suite SHALL verify that at least 10,000 repeated invocations of each Safe_API function complete without panicking.
7. THE Test_Suite SHALL include a test exercising repeated string-producing calls that can be run under a leak detector (e.g. valgrind) to confirm no memory is leaked.
8. THE Test_Suite SHALL organize unit tests within a `#[cfg(test)] mod tests` block.
9. FOR ALL inputs consisting only of ASCII lowercase letters, THE Test_Suite SHALL verify that applying uppercase then host-side lowercase reproduces the original input (round-trip property for ASCII case).
10. THE Test_Suite SHALL verify that the length reported by the length function equals the byte length of the input string for representative inputs.

### Requirement 8: Reproducible Docker Environment

**User Story:** As an evaluator on any machine, I want a containerized build and test environment, so that I can reproduce results without installing the toolchain locally.

#### Acceptance Criteria

1. THE Docker_Environment image SHALL contain a Rust toolchain, LLVM/Clang for the Bindgen_Generator, and a C++ compiler, each pinned to an exact version.
2. WHEN the Docker_Environment build is executed, THE Docker_Environment SHALL compile the crate and then run the Test_Suite within a single build invocation.
3. WHEN the crate compiles and all tests pass, THE Docker_Environment SHALL complete with a zero exit status.
4. THE Docker_Environment SHALL provide a `docker-compose.yml` that, with a single command, compiles the crate and runs the Test_Suite without requiring any host-installed toolchain.
5. IF C++ or crate compilation fails inside the Docker_Environment, THEN THE Docker_Environment SHALL halt before running tests and report a non-zero exit status indicating compilation failure.
6. IF the Test_Suite fails inside the Docker_Environment, THEN THE Docker_Environment SHALL report a non-zero exit status and surface which cases failed.

### Requirement 9: Documentation

**User Story:** As a new developer, I want clear documentation, so that I can set up, build, and understand the memory-management decisions of the project.

#### Acceptance Criteria

1. THE Safe_API SHALL provide function-level documentation with a usage example for each of the four public Safe_API functions.
2. THE project SHALL provide a README describing the setup and the Build_Script-driven build process.
3. THE README SHALL state the build requirements, including the required C++ compiler (CC_Compiler) and binding-generation tooling (Bindgen_Generator / LLVM-Clang).
4. THE documentation SHALL state, for each allocation, which side (C++ or Rust) owns the memory and which side releases it (Owned_String copied by Rust, released via the Deallocation_Function).
5. THE documentation SHALL state that uppercase conversion and vowel counting are ASCII-defined and that non-ASCII bytes pass through unchanged.
6. THE documentation SHALL describe how to run the CLI_Demo and the Docker_Environment.
7. THE documentation SHALL describe the Interior_NUL / Conversion_Error behavior as a usage note.
