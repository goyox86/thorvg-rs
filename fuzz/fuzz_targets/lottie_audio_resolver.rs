#![no_main]

//! Fuzz target for `LottieAnimation::set_audio_resolver` (ThorVG 1.0.6).
//!
//! Loads arbitrary bytes as a Lottie document, installs an audio
//! resolver, then advances the timeline so that any audio layer the
//! parser produced drives `updateAudio` — which calls back into Rust
//! through the monomorphized `audio_trampoline`. The resolver reads
//! every `AudioInfo` accessor (including the pointer-backed `source`,
//! `embedded_data`, and `mime_type`). This exercises:
//!
//!   * the `AudioInfo` borrowed view — `source` / `embedded_data` /
//!     `mime_type` must safely handle null and arbitrary C strings the
//!     parser may hand back;
//!   * the heap-boxed closure storage living inside `LottieAnimation`,
//!     and its `Drop`-time unregister (use-after-free guard).
//!
//! If either regresses, libfuzzer surfaces it as a crash.
//!
//! The trampoline's `catch_unwind` guard is *not* exercised here: under
//! libfuzzer-sys a panic hook aborts the process before unwinding, so a
//! deliberate panic could never be caught regardless. That guard is
//! covered by `test_audio_resolver_panic_is_caught` in the integration
//! suite, which runs in a normal unwinding build.

use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use thorvg::{AudioInfo, Thorvg};

// Field order matters for corpus seeding: `frames` is consumed from the
// front and `json` is the trailing `arbitrary_take_rest` slice, so a seed
// file is simply `[16 frame bytes][json...]`.
#[derive(Arbitrary, Debug)]
struct Input<'a> {
    /// Frames to step through, toggling audio-layer activation
    /// (subnormals, NaN, and infinities included).
    frames: [f32; 4],
    /// Lottie document bytes (the rest of the input).
    json: &'a [u8],
}

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input<'_>| {
    ENGINE.with(|engine| {
        let Ok(mut lottie) = engine.lottie_animation() else {
            return;
        };
        // The resolver can only be installed once a loader exists, so
        // load first. Invalid documents are expected — the fuzzer wants
        // crashes in the parser / trampoline, not valid output.
        if lottie.load_data(input.json).is_err() {
            return;
        }

        let _ = lottie.set_audio_resolver(move |info: &AudioInfo| {
            // Touch every accessor so the pointer-backed ones run on
            // whatever the parser produced.
            let _ = info.source();
            let _ = info.embedded_data();
            let _ = info.mime_type();
            let _ = info.size();
            let _ = info.offset();
            let _ = info.volume();
            let _ = info.is_active();
            let _ = info.is_embedded();
        });

        // Step the timeline; activation transitions fire the resolver.
        for f in input.frames {
            let _ = lottie.set_frame(f);
        }

        // Clearing then dropping must run the unregister path cleanly.
        let _ = lottie.clear_audio_resolver();
    });
});
