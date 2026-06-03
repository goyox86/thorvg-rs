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
# Loader / parser surface
cargo +nightly fuzz run picture_load_data
cargo +nightly fuzz run picture_load_raw
cargo +nightly fuzz run lottie_load_and_play
cargo +nightly fuzz run text_glyph_metrics
# Builder / mutator surface
cargo +nightly fuzz run shape_append_path
cargo +nightly fuzz run shape_primitives
cargo +nightly fuzz run paint_transforms
cargo +nightly fuzz run scene_effects
cargo +nightly fuzz run gradient_build
# Canvas / animation / IO
cargo +nightly fuzz run canvas_set_target
cargo +nightly fuzz run animation_picture_play
cargo +nightly fuzz run saver_save
# Closure / trampoline edges
cargo +nightly fuzz run asset_resolver
cargo +nightly fuzz run accessor_for_each
cargo +nightly fuzz run lottie_slots_assign
```

Each target initializes one `Thorvg` engine per fuzz process via
`thread_local!` and exercises one entry point of the safe wrapper.
The fuzzer will report any abort, signal, or libfuzzer-detected
memory error.

Corpus seeds and crash reports live under `fuzz/corpus/<target>/` and
`fuzz/artifacts/<target>/` by default.

## Why these targets

Coverage map for the `thorvg` safe public API.

| Target                   | Surface                                                                                                                                 |
| ------------------------ | --------------------------------------------------------------------------------------------------------------------------------------- |
| `picture_load_data`      | Loader-manager dispatch over all enabled MIME types — broadest surface in the crate. Regression target for C-2 / C-3.                   |
| `picture_load_raw`       | Raw-pixel branch with arbitrary `(width, height, colorspace)` triples.                                                                  |
| `shape_append_path`      | C path builder reads `cmds` as opcodes and pulls 0/1/3 points from `pts`; arity mismatch / unknown opcodes are real OOB vectors.        |
| `shape_primitives`       | Arbitrary sequences of `Shape` path/style builders (`move_to`/`cubic_to`/`append_rect`/dash/trim/stroke) with pathological floats.      |
| `paint_transforms`       | `Paint` trait surface: `scale`/`rotate`/`translate`/`set_transform`/opacity/blend/mask/clip/intersects/bounds, including ownership transfers via `set_mask`/`set_clip`. |
| `scene_effects`          | `Scene` post-processing chain: `add_gaussian_blur`/`add_drop_shadow`/`add_fill_effect`/`add_tint_effect`/`add_tritone_effect`/`clear_effects`. |
| `gradient_build`         | `LinearGradient` / `RadialGradient` with arbitrary color stops, bounds/radial, spread, and transform; attached to a shape via `set_*_gradient`. |
| `canvas_set_target`      | `SwCanvas::set_target` overflow-check (`stride * height` in u64) + `push`/`draw`/`sync`/`set_viewport`.                                 |
| `animation_picture_play` | `Animation` controller backed by a `Picture` loaded from arbitrary bytes; drives `set_frame`/`set_segment` with pathological floats.    |
| `asset_resolver`         | Trampoline + recursive `tvg_picture_load_data` from the C side. Optional panic in the closure exercises the `catch_unwind` guard (C-3). |
| `accessor_for_each`      | `Accessor::for_each` trampoline + closure that may early-exit or panic; verifies `BorrowedAccessor::get_name` / `BorrowedPaint::*` view safety. |
| `lottie_load_and_play`   | Lottie parser + animation controller with arbitrary frames, segments, tweens, markers, and slots.                                       |
| `lottie_slots_assign`    | Lottie dynamic-content API: `gen_slot`/`apply_slot`/`del_slot`/`assign`/`set_marker` with arbitrary IDs and strings.                    |
| `text_glyph_metrics`     | Font registration + UTF-8 text shaping + glyph metric query through the font registry.                                                  |
| `saver_save`             | `Saver::save_to_str` / `save_animation_to_str` — path → CString boundary + C-side format dispatcher; writes are sandboxed to `$TMPDIR`. |

The `picture_load_*` targets use the always-copy variant; the `*_static` borrowing variants take `&'static [u8]` and cannot be
fuzzed without manufactured leaks.

The `saver_save` target writes to a per-iteration file under `std::env::temp_dir()` and removes it immediately; the fuzzer-supplied path string is sanitised to a filename and used as an extension hint for the C-side format dispatcher.
