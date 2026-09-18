# syntax=docker/dockerfile:1

# Pinned Rust toolchain on Debian bookworm (Req 8.1: version-pinned toolchain; 1.90 supports edition 2024 deps).
# bookworm ships clang/libclang (for bindgen) and g++ (for the cc crate) via apt.
FROM rust:1.90.0-bookworm

# Install pinned LLVM/Clang (libclang is required by bindgen) and g++ (used by the
# cc crate to compile the C++ sources), plus valgrind for the leak-detection test.
# Every apt package is pinned to an exact version for reproducibility (Req 8.1).
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        clang=1:14.0-55.7~deb12u1 \
        libclang-14-dev=1:14.0.6-12 \
        llvm-14-dev=1:14.0.6-12 \
        g++=4:12.2.0-3 \
        valgrind=1:3.19.0-1 \
    && rm -rf /var/lib/apt/lists/*

# bindgen needs to locate libclang.
ENV LIBCLANG_PATH=/usr/lib/llvm-14/lib

WORKDIR /workspace

# Default: compile the crate, then run the full test suite in a single invocation
# (Req 8.2). A compile failure halts before tests (Req 8.5); a green run exits 0
# (Req 8.3); a test failure exits non-zero (Req 8.6).
CMD ["bash", "-c", "cargo build --locked && cargo test --locked"]
