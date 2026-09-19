#!/usr/bin/env bash
#
# Run the FFI leak-detection integration test under valgrind.
#
# IMPORTANT: This must be run INSIDE the Docker dev/test container — the host has
# no Rust/Cargo toolchain and no valgrind. From the workspace root run:
#
#     docker compose run --rm dev bash scripts/valgrind.sh
#
# It builds the integration test binary, then runs the `leak_detection_many_allocations`
# test under valgrind with the scoped suppression file (valgrind.supp). Valgrind
# exits non-zero (via --error-exitcode=1) if any real, unsuppressed leak is found;
# the benign libtest/std-runtime blocks are suppressed. The real guarantee we care
# about is "definitely lost: 0 bytes" from our FFI allocation path.
set -euo pipefail

# Build only the integration test binary without running it.
cargo test --test integration --no-run --locked

# Locate the compiled integration test binary robustly (skip .d dep files).
BIN=$(find target/debug/deps -maxdepth 1 -type f -name 'integration-*' ! -name '*.d' -print | sort | tail -1)

if [[ -z "${BIN}" ]]; then
    echo "error: could not locate the integration test binary under target/debug/deps" >&2
    exit 1
fi

echo "Running valgrind on: ${BIN}"
valgrind --leak-check=full --show-leak-kinds=all --error-exitcode=1 \
    --suppressions=valgrind.supp \
    "${BIN}" leak_detection_many_allocations --test-threads=1
