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
cargo +nightly fuzz run picture_load_data        # arbitrary mime + bytes
cargo +nightly fuzz run picture_load_raw         # arbitrary pixel buffer
```

Each target initializes one `Thorvg` engine per fuzz process and feeds
arbitrary input to the corresponding `Picture::load_*` API. The fuzzer
will report any abort, signal, or libfuzzer-detected memory error.

Corpus seeds and crash reports live under `fuzz/corpus/<target>/` and
`fuzz/artifacts/<target>/` by default.

## Why these targets

* `picture_load_data` — broadest surface in the crate (drives the full
  loader-manager dispatch); regression target for C-2 (`load_data`
  buffer lifetime contract) and C-3 (panic safety at the FFI edge).
* `picture_load_raw` — exercises the raw-pixel branch with arbitrary
  `(width, height, colorspace)` triples.

Both targets use the always-copy variant; the `*_static` borrowing
variants take `&'static [u8]` and cannot be fuzzed without manufactured
leaks.
