#![no_main]

//! Fuzz target for the `Animation` controller backed by a `Picture`
//! loaded from arbitrary bytes.
//!
//! `Animation::picture_mut` returns the C-owned picture; the
//! animation refcounts it, so a `load_data` here exercises both the
//! loader plumbing and the animation's frame/segment counters when
//! they're computed from a possibly-malformed document.  Setters
//! and getters are then driven with arbitrary `f32`s.

use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use thorvg::{MimeType, Thorvg};

#[derive(Debug)]
enum FuzzMime {
    Svg,
    Lottie,
    Png,
    Jpg,
    Webp,
}

impl<'a> Arbitrary<'a> for FuzzMime {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(match u.int_in_range::<u8>(0..=4)? {
            0 => Self::Svg,
            1 => Self::Lottie,
            2 => Self::Png,
            3 => Self::Jpg,
            _ => Self::Webp,
        })
    }
}

impl FuzzMime {
    fn to_mime(&self) -> MimeType {
        match self {
            Self::Svg => MimeType::Svg,
            Self::Lottie => MimeType::Lottie,
            Self::Png => MimeType::Png,
            Self::Jpg => MimeType::Jpg,
            Self::Webp => MimeType::Webp,
        }
    }
}

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    data: &'a [u8],
    mime: FuzzMime,
    frame: f32,
    seg_begin: f32,
    seg_end: f32,
}

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input<'_>| {
    ENGINE.with(|engine| {
        let Ok(mut anim) = engine.animation() else {
            return;
        };
        // Load arbitrary bytes through the animation's borrowed
        // picture.  The animation will recompute total_frame /
        // duration from whatever the loader produced.
        let _ = anim
            .picture_mut()
            .load_data(input.data, input.mime.to_mime(), None);
        let _ = anim.set_frame(input.frame);
        let _ = anim.set_segment(input.seg_begin, input.seg_end);
        // Read-side queries — should never crash regardless of
        // load state or pathological setter inputs.
        let _ = anim.frame();
        let _ = anim.total_frame();
        let _ = anim.duration();
        let _ = anim.segment();
    });
});
