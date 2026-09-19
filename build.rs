use std::env;
use std::path::PathBuf;

fn main() {
    // Rebuild triggers (Req 4.4, 4.5): one directive per C++ source file plus
    // one for the C header, so any edit to the native layer re-runs this script
    // and recompiles the affected sources on the next `cargo build`.
    println!("cargo:rerun-if-changed=cpp/lib.cpp");
    println!("cargo:rerun-if-changed=cpp/wrapper.h");

    // 1) Compile the C++ into a static lib and link it (Req 4.1, 4.2, 4.3, 4.6).
    //    .cpp(true) selects the C++ compiler and links the C++ standard library
    //    (Req 4.3); .include("cpp") supplies the include path so wrapper.h
    //    resolves without unresolved-include errors (Req 4.2);
    //    .compile("stringutils") emits libstringutils.a and the link directives,
    //    so Cargo links it automatically (Req 4.1, 4.6). A cc failure aborts the
    //    build with a non-zero status and surfaces the diagnostics (Req 4.7).
    cc::Build::new()
        .cpp(true)
        .file("cpp/lib.cpp")
        .include("cpp")
        .compile("stringutils");

    // 2) Generate Rust FFI bindings from the C header (Req 3.1, 3.2, 3.3, 3.4,
    //    3.5, 3.6). One declaration is produced per allowlisted function (Req
    //    3.1); the `-x c++` clang args make the C++ header parse cleanly;
    //    allowlisting the five functions keeps the bindings tight so no stray
    //    system types leak in (supports Req 3.2). CargoCallbacks re-emits
    //    rerun-if-changed for headers bindgen pulls in.
    let bindings = bindgen::Builder::default()
        .header("cpp/wrapper.h")
        .clang_arg("-x")
        .clang_arg("c++")
        .allowlist_function("str_length")
        .allowlist_function("count_vowels")
        .allowlist_function("str_reverse")
        .allowlist_function("to_uppercase")
        .allowlist_function("free_string")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        // On a parse/type-mapping failure this panics, which Cargo reports as a
        // non-zero build exit with a diagnostic naming the failing header
        // (Req 3.4, 3.5, 3.6).
        .expect("bindgen failed to parse cpp/wrapper.h");

    // Write to $OUT_DIR/bindings.rs so the crate compiles the bindings into the
    // FFI layer during the same build invocation (Req 3.3). The write only runs
    // after a successful generate(), so no bindings.rs is written on parse
    // failure (Req 3.4).
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out.join("bindings.rs"))
        .expect("failed to write $OUT_DIR/bindings.rs");
}
