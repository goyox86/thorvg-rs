# Changelog

All notable changes to the `thorvg-sys` crate are documented here. The
format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

**Versioning.** This crate uses its **own** SemVer, decoupled from the
vendored ThorVG C++ release. The bundled ThorVG version is recorded as
SemVer **build metadata** — e.g. `0.1.0+thorvg-1.0.5` is crate `0.1.0`
bundling ThorVG `1.0.5`. Because the crate is `0.x`, a **minor** bump is
breaking; the safe [`thorvg`](../thorvg/CHANGELOG.md) crate's dependency
moves in lockstep.

## [0.1.0+thorvg-1.0.5] - 2026-06-13

First release under the crate's own versioning. **Supersedes the yanked
`1.0.0` / `1.0.1` / `1.0.5`**, which mirrored the upstream ThorVG version
number 1:1 — a scheme abandoned because it left no room to publish
sys-crate-only changes (build system, bare-metal support) while upstream
stayed at 1.0.5. Bundles **ThorVG 1.0.5**.

### Changed

- **Versioning scheme** — the crate version is now independent of
  upstream; the bundled ThorVG release is carried as `+thorvg-X.Y.Z`
  build metadata. The previous upstream-mirroring `1.0.x` releases are
  yanked.

### Build system

- **Replaced the meson + ninja build with the `cc` crate** — ThorVG is
  compiled from source via Cargo's configured (cross-)compiler; no meson
  or ninja required.
- **Feature-gated loaders and capabilities** — `lottie`, `svg`, `png`,
  `fonts`, `expressions`, `threads`, `file-io` Cargo features select
  which ThorVG components compile (all enabled by default; embedded users
  disable defaults and pick what they need).
- bindgen now passes an explicit `--target=` to libclang on host builds.

### Bare-metal support (`target_os = "none"`)

- Toolchain-agnostic cross-compilation pipeline, split into bare-metal
  vs. SDK-runtime policy.
- **Vendors picolibc** (submodule pinned to 1.8.11) as the libc on
  bare-metal: compile-time `picolibc.h` configuration plus a compile-only
  validation phase in `build.rs`.
- Per-concern runtime stubs welded to picolibc declarations with weak
  linkage; bridges newlib's `__errno()` to picolibc's plain `errno`;
  stubs `_on_exit`.
- RISC-V canonical-multilib selection; `expressions` enabled on
  bare-metal ESP32-C6.

### Vendored ThorVG patches

- Local shims layered on ThorVG 1.0.5 to support bare-metal builds:
  a `bsearch` shim, a Lottie-loader shim extension, and runtime-stub
  welding across `tvgLock.h`, `tvgInitializer.cpp`, `tvgRender.cpp`,
  `tvgSwRenderer.cpp`, and `tvgSwMemPool.cpp`.

[0.1.0+thorvg-1.0.5]: https://github.com/goyox86/thorvg-rs/releases/tag/thorvg-sys-v0.1.0
