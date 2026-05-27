# Changelog

All notable changes to the `thorvg` crate will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0]

Initial release (previous 0.1.x versions were yanked and restarted at 0.1.0
with the changes below folded in).

### thorvg (safe wrapper)

- **`Send` impls for all handle types** — `SwCanvas`, `GlCanvas`, `WgCanvas`,
  `Animation`, `Picture`, `Shape`, `Scene`, `Text`, `Saver`, and `Accessor` are
  now `Send`.  Each type exclusively owns its heap-allocated ThorVG handle, and
  the C++ library guards shared global state with internal mutexes.  All types
  remain `!Sync`.  The `Thorvg` engine guard is intentionally `!Send + !Sync`.
- **`LottieAnimation::load_data(data)`** — convenience loader from a JSON byte
  slice.
- **`LottieAnimation::load_file(path)`** *(requires `std`)* — convenience
  loader from a file path.
- **`LottieAnimation::set_size(w, h)`** — convenience delegate to
  `picture().set_size()`.
- **`SwCanvas::render()`**, **`GlCanvas::render()`**, **`WgCanvas::render()`**
  — `update() + draw(true) + sync()` in one call.
- **Feature-gated loaders** — `lottie`, `svg`, `png`, `fonts`, `expressions`,
  `threads`, `file-io` features pass through to `thorvg-sys`.
- Removed `EngineOption::Aliased` (dropped upstream in ThorVG 1.0.5).

### thorvg-sys (FFI bindings)

- **Vendored ThorVG updated to v1.0.5.**
- **Replaced meson+ninja build with `cc` crate** — cross-compiles to embedded
  targets (ESP32, RP2350, etc.) out of the box.  No meson or ninja required.
- **Feature-gated loaders and capabilities** — `lottie`, `svg`, `png`, `fonts`,
  `expressions`, `threads`, `file-io` cargo features control which ThorVG
  components are compiled.  All enabled by default.  Embedded users disable
  defaults and pick only what they need.

[0.1.0]: https://github.com/goyox86/thorvg-rs/releases/tag/v0.1.0
