#![no_main]

//! Fuzz target for `Picture::set_asset_resolver`.
//!
//! Drives a fixed SVG that references one external asset, so the C
//! loader invokes the installed resolver during `load_data`.  The
//! resolver returns arbitrary `(bytes, mime)` (or `None`), and on a
//! configurable branch it panics — exercising both:
//!
//!   * **C-1**: the C-side data pointer must survive the closure
//!     storage that lives inside `Picture`;
//!   * **C-3**: a panic in the user closure must be caught at the
//!     `extern "C"` boundary rather than unwinding into C++.
//!
//! If either regression returns, libfuzzer surfaces it as a crash.

use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use thorvg::{MimeType, Thorvg};

#[derive(Debug, Clone)]
enum FuzzMime {
    Svg,
    Png,
    Jpg,
    Webp,
    Lottie,
    Raw,
}

impl<'a> Arbitrary<'a> for FuzzMime {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(match u.int_in_range::<u8>(0..=5)? {
            0 => Self::Svg,
            1 => Self::Png,
            2 => Self::Jpg,
            3 => Self::Webp,
            4 => Self::Lottie,
            _ => Self::Raw,
        })
    }
}

impl FuzzMime {
    fn to_mime(&self) -> MimeType {
        match self {
            Self::Svg => MimeType::Svg,
            Self::Png => MimeType::Png,
            Self::Jpg => MimeType::Jpg,
            Self::Webp => MimeType::Webp,
            Self::Lottie => MimeType::Lottie,
            Self::Raw => MimeType::Raw,
        }
    }
}

#[derive(Arbitrary, Debug)]
struct Input {
    /// What the resolver returns on lookup.  `None` means "asset not found".
    payload: Option<(Vec<u8>, FuzzMime)>,
    /// If true, the resolver panics instead of returning — exercises C-3.
    panic_in_resolver: bool,
}

// Minimal SVG that references one external asset; the loader will
// invoke the resolver with `src = "asset"` while parsing the document.
const SVG_WITH_ASSET: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" width="100" height="100"><image href="asset" x="0" y="0" width="100" height="100"/></svg>"#;

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input| {
    ENGINE.with(|engine| {
        let Ok(mut pic) = engine.picture() else {
            return;
        };
        let Input {
            payload,
            panic_in_resolver,
        } = input;
        let _ = pic.set_asset_resolver(move |_src| {
            if panic_in_resolver {
                // catch_unwind in the trampoline must absorb this;
                // unwinding into C++ would be UB.
                panic!("fuzz-induced resolver panic");
            }
            payload.clone().map(|(bytes, mime)| (bytes, mime.to_mime()))
        });
        let _ = pic.load_data(SVG_WITH_ASSET, MimeType::Svg, None);
    });
});
