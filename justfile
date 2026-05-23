# thorvg-rs justfile
# Run `just --list` to see all available recipes.

set dotenv-load := false

# Host target triple (sanitizers require an explicit --target)
target := `rustc -vV | grep host | cut -d' ' -f2`

# ── Build ───────────────────────────────────────────────────────────

# Build the entire workspace
build:
    cargo build --workspace

# Build all examples
build-examples:
    cargo build --examples

# Build with no_std (no default features)
build-no-std:
    cargo check -p thorvg --no-default-features

# ── Test ────────────────────────────────────────────────────────────

# Run all tests
test:
    cargo test --workspace

# Run only thorvg-sys tests
test-sys:
    cargo test -p thorvg-sys

# Run only thorvg safe wrapper tests
test-lib:
    cargo test -p thorvg

# ── Lint ────────────────────────────────────────────────────────────

# Run clippy with pedantic warnings on everything
clippy:
    cargo clippy --workspace --all-targets -- -W clippy::pedantic

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Apply formatting
fmt:
    cargo fmt --all

# ── Sanitizers ──────────────────────────────────────────────────────

# Run tests under AddressSanitizer (use-after-free, double-free, buffer overflow)
test-asan:
    RUSTFLAGS="-Z sanitizer=address" ASAN_OPTIONS="detect_leaks=0" \
    CARGO_TARGET_DIR="target/asan" \
        cargo test --target {{ target }} -p thorvg
    RUSTFLAGS="-Z sanitizer=address" ASAN_OPTIONS="detect_leaks=0" \
    CARGO_TARGET_DIR="target/asan" \
        cargo test --target {{ target }} -p thorvg-sys

# Run tests under AddressSanitizer with leak detection
test-asan-leaks:
    RUSTFLAGS="-Z sanitizer=address" \
    CARGO_TARGET_DIR="target/asan" \
        cargo test --target {{ target }} -p thorvg -- --test-threads=1
    RUSTFLAGS="-Z sanitizer=address" \
    CARGO_TARGET_DIR="target/asan" \
        cargo test --target {{ target }} -p thorvg-sys -- --test-threads=1

# Run tests under ThreadSanitizer (data races)
# Note: TSan requires -Cunsafe-allow-abi-mismatch=sanitizer on some nightly toolchains
# where std is not built with TSan. For full TSan, use -Zbuild-std.
test-tsan:
    RUSTFLAGS="-Z sanitizer=thread -Cunsafe-allow-abi-mismatch=sanitizer" \
    CARGO_TARGET_DIR="target/tsan" \
        cargo test --target {{ target }} -p thorvg -- --test-threads=1
    RUSTFLAGS="-Z sanitizer=thread -Cunsafe-allow-abi-mismatch=sanitizer" \
    CARGO_TARGET_DIR="target/tsan" \
        cargo test --target {{ target }} -p thorvg-sys -- --test-threads=1

# Run all sanitizers (ASan + TSan)
test-sanitizers: test-asan test-tsan

# ── Examples ────────────────────────────────────────────────────────

# Run all examples and generate PNG output
examples:
    @for ex in shapes stroke gradient scene render_to_buffer \
               transforms blending opacity clipping scene_effects \
               paths picture_svg masking paint_order; do \
        echo "=== $ex ==="; \
        cargo run --example "$ex"; \
    done

# ── CI (full pipeline) ─────────────────────────────────────────────

# Full CI check: fmt, clippy, build, test, no_std, sanitizers
ci: fmt-check clippy build build-no-std test test-asan test-tsan

# Quick CI check: fmt, clippy, test (no sanitizers)
ci-quick: fmt-check clippy test build-no-std

# ── Utility ─────────────────────────────────────────────────────────

# Clean all build artifacts
clean:
    cargo clean

# Show coverage stats (which C API functions are wrapped)
coverage:
    #!/usr/bin/env bash
    set -euo pipefail
    header="thorvg-sys/thorvg/src/bindings/capi/thorvg_capi.h"
    grep -oP 'TVG_API\s+\w+\s+(\w+)\s*\(' "$header" \
        | grep -oP '\w+\s*\(' | sed 's/(//' | sort > /tmp/capi_all.txt
    grep -rhoP 'ffi::(\w+)' thorvg/src/*.rs \
        | sed 's/ffi:://' | sort -u > /tmp/capi_used.txt
    total=$(wc -l < /tmp/capi_all.txt)
    covered=$(comm -12 /tmp/capi_all.txt /tmp/capi_used.txt | wc -l)
    missing=$(comm -23 /tmp/capi_all.txt /tmp/capi_used.txt | wc -l)
    echo "C API coverage: $covered / $total (missing: $missing)"
    if [ "$missing" -gt 0 ]; then
        echo ""
        echo "Missing:"
        comm -23 /tmp/capi_all.txt /tmp/capi_used.txt | sed 's/^/  /'
    fi
