# Third-Party Licenses

This crate vendors [ThorVG](https://github.com/thorvg/thorvg) and
[picolibc](https://github.com/picolibc/picolibc) as git submodules.
ThorVG itself bundles several third-party components. The licenses
of all vendored code are listed below.

## ThorVG

- **License:** MIT
- **Copyright:** 2020–2026 ThorVG Project
- **Source:** <https://github.com/thorvg/thorvg>

## RapidJSON

Bundled in ThorVG for Lottie JSON parsing.

- **License:** MIT
- **Copyright:** 2015 THL A29 Limited (Tencent), Milo Yip
- **File:** `thorvg-sys/thorvg/src/loaders/lottie/rapidjson/LICENSE`

## JerryScript

Bundled in ThorVG for Lottie expression evaluation.

- **License:** Apache-2.0
- **Copyright:** JS Foundation and other contributors
- **File:** `thorvg-sys/thorvg/src/loaders/lottie/jerryscript/jerry-core/LICENSE`
- **Note:** Apache-2.0 is compatible with MIT but NOT with GPLv2.
  See the license file for details.

## WebP (libwebp)

Bundled in ThorVG for WebP image decoding.

- **License:** BSD-3-Clause with patent grant
- **Copyright:** 2012 Google Inc.
- **File:** `thorvg-sys/thorvg/src/loaders/webp/LICENSE`

## LodePNG

Bundled in ThorVG for PNG decoding.

- **License:** zlib
- **Copyright:** 2005–2020 Lode Vandevenne
- **File:** Embedded in `thorvg-sys/thorvg/src/loaders/png/tvgLodePng.h`

## picolibc

Vendored as a git submodule at `thorvg-sys/picolibc/`, pinned to tag
`1.8.11`. Compiled and linked only on bare-metal targets
(`target_os == "none"`) to provide the C library symbols thorvg's C++
TUs reference (ctype, string, stdlib parsers, stdio, setjmp/longjmp,
bsearch, …) without depending on the cross toolchain's libc.a.

- **License:** BSD-style (mixture of 2-clause and 3-clause BSD plus a
  handful of older permissive licenses inherited from newlib and
  AVR libc). All MIT-compatible.
- **Copyright:** 2018–2026 Keith Packard and contributors; portions
  © newlib, AVR libc, and NetBSD contributors.
- **Files:** `thorvg-sys/picolibc/COPYING.picolibc` (full machine-
  readable manifest in Debian DEP-5 format).
- **Upstream:** <https://github.com/picolibc/picolibc>
