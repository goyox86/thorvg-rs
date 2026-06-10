# Test assets

Tiny fixtures used by the asset-resolver unit tests in
`thorvg/src/tests.rs`.

## License

The files in this directory (`logo.png`, `logo.svg`) are released
into the **public domain** under
[CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/).

They are hand-authored test fixtures and are **not** affiliated with
the official ThorVG brand assets (which live in the proprietary
[`thorvg.site`](https://github.com/thorvg/thorvg.site) repository and
are not redistributable).

## Contents

| File       | Size   | Format     | Purpose                                                    |
|------------|--------|------------|------------------------------------------------------------|
| `logo.png` | 32×32  | PNG (RGBA) | Returned from a Rust asset resolver to verify the success branch of the FFI trampoline. |
| `logo.svg` | 32×32  | SVG        | SVG counterpart for completeness; not currently exercised in tests but kept alongside the PNG. |
