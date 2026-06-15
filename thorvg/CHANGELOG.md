# Changelog

All notable changes to the `thorvg` crate are documented here. The
format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

While the crate is pre-`1.0`, breaking changes bump the **minor**
version (`0.y`), per SemVer's `0.x` rule.

The companion `thorvg-sys` FFI crate is versioned independently — see
[`thorvg-sys/CHANGELOG.md`](../thorvg-sys/CHANGELOG.md).

## [0.4.1] - 2026-06-15

Portability and documentation fixes. No API change — a patch bump.

### Fixed

- **Windows (MSVC) build.** Path-command decoding no longer depends on
  bindgen's enum repr: the `TVG_PATH_COMMAND_*` constants are `c_uint` on
  most targets but `c_int` on MSVC, which broke the `match` (E0308).
- **Three broken rustdoc intra-doc links** (`Thorvg::load_font`,
  `BorrowedPaint`, `BorrowedPaint::paint_type`).

### Changed

- **Depends on `thorvg-sys` 0.2.1**, which carries the MSVC and
  system-library (pkg-config) build fixes.

## [0.4.0] - 2026-06-15

Tracks **ThorVG 1.0.6** (via `thorvg-sys` 0.2). Picks up the upstream
API changes that affect the safe surface.

### Changed (breaking)

- **Depends on `thorvg-sys` 0.2** (bundling ThorVG 1.0.6).

### Added

- **`LottieAnimation::set_audio_resolver` / `clear_audio_resolver`** and the
  **`AudioInfo`** borrowed view — safe wrappers over ThorVG 1.0.6's new
  Lottie audio-resolver callback, for synchronizing external audio
  playback with the animation timeline. The closure is heap-stored and
  unregistered on drop (same ownership model as the asset resolver).
  Experimental upstream.
- **`EngineOption::Aliased`** — re-introduced upstream in ThorVG 1.0.6
  (disables anti-aliased rendering). Marked experimental by upstream.
  The `EngineOption` enum is `#[non_exhaustive]`, so this is additive.

### Removed (breaking)

- **`LottieAnimation::assign`** — the underlying experimental
  `tvg_lottie_animation_assign` C API was removed in ThorVG 1.0.6.

## [0.3.0] - 2026-06-13

A large API-hardening and ergonomics release. The headline themes are
type-safety (enums and parameter structs replacing primitives), a
shared color module, sealed traits, lifetime-correct borrowed views,
and a sweep of memory-safety fixes around FFI ownership.

### Changed (breaking)

- **Colors are now typed.** `Text::set_color` / `set_outline` take
  `Rgb`; `Shape` fill/stroke and gradient `ColorStop` take `Rgba`.
  Raw `u8` channel arguments are gone. `Rgb`/`Rgba` live in a shared
  `color` module, which now also hosts `ColorSpace`.
- **`EngineOption` is a plain enum** (`None` / `Default` / `SmartRender`)
  matching the C++ engine's exact-equality semantics, instead of a
  bitflags newtype.
- **`Shape` paint order** is a `PaintOrder` enum (`FillThenStroke` /
  `StrokeThenFill`) instead of a `bool`.
- **Shape primitives take parameter structs** — `Rect` and `Circle`
  (with builders and `Default`) replace long positional argument lists.
- **Scene effects take parameter structs** — `GaussianBlur`,
  `DropShadow`, `Tint`, and `Tritone` (builders + `Default`); blur
  direction and border are now enums rather than ints. Effect methods
  were renamed for consistency.
- **Factory methods return `Result`** across paint/scene/picture/text/
  animation/saver/accessor/gradient, instead of panicking or returning
  bare handles on allocation failure.
- **`set_mask` / `set_clip` consume the paint by value**, modelling the
  ownership transfer to the C engine (prevents double-free / leaks).
- **`Canvas::push` and `Scene::push` renamed to `add`** for C API parity.
- **`Picture::load_data` takes a typed `MimeType` enum** instead of a
  string.
- **Font-loading API moved from `Text` to `Thorvg`** (it is engine-global
  state, not per-text).
- **`Thorvg::init` signature is gated on the `threads` feature.**
- **`Paint` and `Canvas` are now sealed traits** — they cannot be
  implemented downstream.
- **`from_raw` conversions are exhaustive** for `PaintType` and
  `MaskMethod`, and `Shape` enums (`FillRule`, `StrokeCap`, `StrokeJoin`)
  are exhaustive.

### Added

- Typed `Path` / `PathCommand` / `Segment` model; `Shape::path` returns
  it and `Shape::append_path` closes the round-trip. `Segments` now
  implements `size_hint`.
- `Text::text` getter (C parity).
- `Shape::set_stroke_radial_gradient` (C parity).
- Chainable `Matrix` transform combinators.
- `Point::new` plus rounded-out ergonomics and enum-variant docs.
- `GlCanvas` / `WgCanvas` GPU target builders for API parity (the
  vendored build still strips the GPU engine).
- Safe, closure-based `Picture::set_asset_resolver`; `BorrowedAccessor`
  and `BorrowedPaint` are passed into `for_each` visitors.
- `impl From<NulError> for Error`, so `CString` construction works with
  `?`.
- Gradient kinds are enumerated in `PaintType`.

### Fixed

- Memory leaks in `Paint::set_clip` / `set_mask` and a leaked gradient
  handle in `Shape` (now exposed as a `BorrowedGradient` view). Leaks are
  memory-safe; the related *double-free* is listed under Security below.
- `Paint`'s mask getter called the wrong C API; replaced with the correct
  one.
- `Animation::picture` and `Picture::get_paint` now return stable borrows
  (`&Picture` / `BorrowedPaint`) instead of a rebuilt wrapper / raw
  handle — clearer ownership. (Neither was unsound in prior releases.)
- Hardened the newly safe, closure-based `Picture::set_asset_resolver`
  (new in this release, see Added) so its trampoline pointer stays stable
  across moves of the picture.

### Security

Several FFI-ownership and bounds bugs that were **unsound — reachable
from safe code without `unsafe`** — are fixed in this release. All of
them affect published versions **≤ 0.2.0** (the APIs date back to the
initial release) and are fixed in `0.3.0`; users should upgrade.
`0.1.0` and `0.1.1` were already yanked.

- **Use-after-free in `Saver::save_animation`.**
- **Dangling input buffers** — `Picture::load_data` / `Picture::load_raw`
  and `Text::load_font_data` did not tie the borrowed slice's lifetime to
  the consuming object, so the engine could read freed memory after the
  slice was dropped. The buffer lifetime is now enforced by the type
  system.
- **Out-of-bounds read in `Picture::load_raw`** — the pixel-buffer length
  was not validated against the declared `width × height`; now
  bounds-checked.
- **Out-of-bounds write in `SwCanvas::set_target`** — the target buffer
  size computation could overflow and accept an undersized buffer; now
  overflow-checked.
- **Double-free via `Paint::set_clip` / `set_mask`** — these took the
  clipper/mask by reference while the engine took ownership, so the Rust
  wrapper and the engine could both free it. They now consume the paint
  by value.
- **Undefined behaviour on panic across FFI** — a panic in a user closure
  passed to `Accessor::for_each` unwound through an `extern "C"`
  trampoline (UB). Such closures are now wrapped in `catch_unwind` under
  `std`; the `no_std` panic policy is documented.

### Removed

- The unused, unsafe `Accessor::set` raw-FFI escape hatch.

### Deprecated

- `Gradient::get_transform` — renamed to `transform` for consistency
  with `Paint::transform`.

## [0.2.0] - 2026-05-28

- Renamed the internal `thorvg-sys` re-export from `ffi` to `sys`,
  changing public API signatures.

## [0.1.2] - 2026-05-28

- README / metadata fixes for crates.io rendering.

## [0.1.0] - 2026-05-23

Initial release of the safe `thorvg` bindings.

- **`Send` for all handle types** — `SwCanvas`, `GlCanvas`, `WgCanvas`,
  `Animation`, `Picture`, `Shape`, `Scene`, `Text`, `Saver`, and
  `Accessor`. Each exclusively owns its heap-allocated ThorVG handle, and
  the C++ library guards shared global state with internal mutexes. All
  types remain `!Sync`; the `Thorvg` engine guard is `!Send + !Sync`.
- **`LottieAnimation::load_data(data)`** — load from a JSON byte slice.
- **`LottieAnimation::load_file(path)`** *(requires `std`)* — load from a
  file path.
- **`LottieAnimation::set_size(w, h)`** — delegate to
  `picture().set_size()`.
- **`SwCanvas::render()` / `GlCanvas::render()` / `WgCanvas::render()`** —
  `update()` + `draw(true)` + `sync()` in one call.
- **Feature-gated loaders** — `lottie`, `svg`, `png`, `fonts`,
  `expressions`, `threads`, `file-io` pass through to `thorvg-sys`.
- Removed `EngineOption::Aliased` (dropped upstream in ThorVG 1.0.5).

[0.4.0]: https://github.com/goyox86/thorvg-rs/releases/tag/thorvg-v0.4.0
[0.3.0]: https://github.com/goyox86/thorvg-rs/releases/tag/thorvg-v0.3.0
[0.2.0]: https://github.com/goyox86/thorvg-rs/releases/tag/thorvg-v0.2.0
[0.1.2]: https://github.com/goyox86/thorvg-rs/releases/tag/thorvg-v0.1.2
[0.1.0]: https://github.com/goyox86/thorvg-rs/releases/tag/thorvg-v0.1.0
