# thorvg fuzz harness

Fuzz targets that hit the `thorvg` safe wrapper's loader paths.

## Prerequisites

```sh
rustup install nightly
cargo install cargo-fuzz
```

## Run a target

```sh
cd fuzz
cargo +nightly fuzz run picture_load_data
cargo +nightly fuzz run picture_load_raw
cargo +nightly fuzz run shape_append_path
cargo +nightly fuzz run asset_resolver
cargo +nightly fuzz run lottie_load_and_play
cargo +nightly fuzz run text_glyph_metrics
```

Each target initializes one `Thorvg` engine per fuzz process via
`thread_local!` and exercises one entry point of the safe wrapper.
The fuzzer will report any abort, signal, or libfuzzer-detected
memory error.

Corpus seeds and crash reports live under `fuzz/corpus/<target>/` and
`fuzz/artifacts/<target>/` by default.

## Why these targets

| Target | Surface |
|--------|---------|
| `picture_load_data` | Loader-manager dispatch over all enabled MIME types — broadest surface in the crate. Regression target for C-2 / C-3. |
| `picture_load_raw` | Raw-pixel branch with arbitrary `(width, height, colorspace)` triples. |
| `shape_append_path` | C path builder reads `cmds` as opcodes and pulls 0/1/3 points from `pts`; arity mismatch / unknown opcodes are real OOB vectors. |
| `asset_resolver` | Trampoline + recursive `tvg_picture_load_data` from the C side. Optional panic in the closure exercises the `catch_unwind` guard (C-3). |
| `lottie_load_and_play` | Lottie parser + animation controller with arbitrary frames, segments, tweens, markers, and slots. |
| `text_glyph_metrics` | Font registration + UTF-8 text shaping + glyph metric query through the font registry. |

The `picture_load_*` targets use the always-copy variant; the
`*_static` borrowing variants take `&'static [u8]` and cannot be
fuzzed without manufactured leaks.
