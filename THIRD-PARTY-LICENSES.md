# Third-Party Licenses

This crate vendors [ThorVG](https://github.com/thorvg/thorvg) as a git
submodule. ThorVG itself bundles several third-party components. Their
licenses are listed below.

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
