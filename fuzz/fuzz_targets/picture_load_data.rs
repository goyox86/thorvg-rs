#![no_main]

//! Fuzz target for `Picture::load_data`.
//!
//! Hardens C-2 (buffer lifetime contract) and C-3 (panic guards):
//! arbitrary mime + resource_path + byte payloads are fed to every
//! loader thorvg compiles in (svg/png/jpg/webp/lottie/raw).  Any
//! soundness regression in the loader plumbing — wild reads, double
//! frees, panics across the FFI edge — surfaces as a libfuzzer crash.

use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use thorvg::{MimeType, Thorvg};

#[derive(Debug)]
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
struct Input<'a> {
    mime: FuzzMime,
    resource_path: Option<&'a str>,
    data: &'a [u8],
}

// One engine per fuzz process — `Thorvg` is `!Send + !Sync` so it
// cannot live in a `static`; `thread_local!` works because libfuzzer
// drives the target on a single thread.
thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input<'_>| {
    ENGINE.with(|engine| {
        let Ok(mut pic) = engine.picture() else {
            return;
        };
        // load_data always copies under the new contract; if the buffer
        // bookkeeping or any loader mis-handles the input, we either
        // get a typed `Err` or crash here.
        let _ = pic.load_data(input.data, input.mime.to_mime(), input.resource_path);
    });
});
