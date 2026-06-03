#![no_main]

//! Fuzz target for `LottieAnimation` load + playback controls.
//!
//! Loads arbitrary bytes as a Lottie document and then drives the
//! controller with arbitrary `f32`s (including subnormals, NaN, and
//! infinities) and arbitrary marker / slot strings.  Catches
//! regressions in the Lottie parser, the frame/segment setters, the
//! tween interpolator, and the slot-application path.

use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use thorvg::Thorvg;

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    json: &'a [u8],
    frame: f32,
    segment_begin: f32,
    segment_end: f32,
    tween_from: f32,
    tween_to: f32,
    tween_progress: f32,
    marker: &'a str,
    slot_json: &'a str,
    quality: u8,
}

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input<'_>| {
    ENGINE.with(|engine| {
        let Ok(mut lottie) = engine.lottie_animation() else {
            return;
        };
        // Load the document.  Errors are fine — the fuzzer is looking
        // for crashes inside the parser, not for valid output.
        if lottie.load_data(input.json).is_err() {
            return;
        }
        let _ = lottie.set_frame(input.frame);
        let _ = lottie.set_segment(input.segment_begin, input.segment_end);
        let _ = lottie.tween(input.tween_from, input.tween_to, input.tween_progress);
        let _ = lottie.set_marker(input.marker);
        if let Some(id) = lottie.gen_slot(input.slot_json) {
            let _ = lottie.apply_slot(id);
            let _ = lottie.del_slot(id);
        }
        let _ = lottie.set_quality(input.quality);
        // Read-side queries — should never crash regardless of state.
        let _ = lottie.frame();
        let _ = lottie.total_frame();
        let _ = lottie.duration();
        let _ = lottie.segment();
        let _ = lottie.markers();
    });
});
