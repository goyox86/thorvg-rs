# picolibc-config

Hand-authored configuration headers for the vendored
[picolibc](../picolibc/) submodule.

Upstream picolibc generates these from `.in` templates via meson at
configure time.  We don't run meson; instead the headers here are
checked into the repo and placed *first* on the compile-time include
path by `thorvg-sys/build.rs` (phase 3 of the picolibc landing,
landing next), so that every picolibc translation unit and every
thorvg translation unit sees the same static configuration.

## Files

| File | Upstream counterpart | Purpose |
|---|---|---|
| `picolibc.h` | `picolibc.h.in` (resolved by meson at build time) | All ~50 picolibc compile-time knobs: errno shape, single-threaded mode, stdio family, math errno policy, locale support, version macros, … |

## Editing

Knobs are documented inline in `picolibc.h` next to each
`#define` / `#undef` site.  When upgrading the picolibc submodule
across a feature window, re-read `picolibc/picolibc.h.in` for any
new `#cmakedefine` lines we don't yet handle and add them here with
an explicit choice (set or unset — *do not* leave them implicitly
absent without a comment, the build relies on every knob being
covered intentionally).

## Why hand-authored vs. generated

* `build.rs` already does substantial work probing the cross
  toolchain (sysroot, multilib, runtime libs).  Wiring it to *also*
  spawn meson or replicate meson's logic for resolving knobs (which
  involves cross-compile-test programs, target introspection, and
  option-file parsing) would more than double its complexity.
* The full set of knobs we care about is small (~30 we set
  intentionally, ~20 deliberately off).  Hand-authoring is faster
  to review and gives us a single auditable artifact.
* Submodule bumps stay clean: the picolibc tree is untouched on
  disk, no generated files leak into the working copy.
