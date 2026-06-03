#![no_main]

//! Fuzz target for `Picture::load_raw`.
//!
//! Exercises the raw-pixel path with arbitrary dimensions and
//! payload sizes.  Catches off-by-one / overflow regressions in the
//! `w * h * sizeof(u32)` indexing on the C side and in the wrapper's
//! `as u32` length conversion.

use libfuzzer_sys::arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;
use thorvg::{ColorSpace, Thorvg};

#[derive(Debug)]
enum FuzzCs {
    Abgr8888,
    Argb8888,
    Abgr8888s,
    Argb8888s,
}

impl<'a> Arbitrary<'a> for FuzzCs {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(match u.int_in_range::<u8>(0..=3)? {
            0 => Self::Abgr8888,
            1 => Self::Argb8888,
            2 => Self::Abgr8888s,
            _ => Self::Argb8888s,
        })
    }
}

impl FuzzCs {
    fn to_cs(&self) -> ColorSpace {
        match self {
            Self::Abgr8888 => ColorSpace::ABGR8888,
            Self::Argb8888 => ColorSpace::ARGB8888,
            Self::Abgr8888s => ColorSpace::ABGR8888S,
            Self::Argb8888s => ColorSpace::ARGB8888S,
        }
    }
}

#[derive(Arbitrary, Debug)]
struct Input {
    // Keep dimensions modest so we don't OOM the fuzzer with the
    // copying loader; libfuzzer reports anything that goes wrong
    // inside thorvg's bounds.
    w: u16,
    h: u16,
    cs: FuzzCs,
    pixels: Vec<u32>,
}

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input| {
    ENGINE.with(|engine| {
        let Ok(mut pic) = engine.picture() else {
            return;
        };
        let _ = pic.load_raw(
            &input.pixels,
            u32::from(input.w),
            u32::from(input.h),
            input.cs.to_cs(),
        );
    });
});
