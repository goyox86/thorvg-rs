#![no_main]

//! Fuzz target for the `LottieAnimation` slot/marker surface.
//!
//! Complements [`lottie_load_and_play`]: where that target stresses
//! the parser and playback controllers, this one focuses on the
//! dynamic-content APIs that take arbitrary strings and IDs after
//! a successful (or unsuccessful) load:
//!
//!   * `gen_slot` (parses a small slot JSON snippet),
//!   * `apply_slot` / `del_slot` (arbitrary u32 IDs, including
//!     ones the engine never produced),
//!   * `set_marker` (UTF-8 marker name lookup),
//!   * `set_size` (arbitrary f32 dimensions).
//!
//! (The expression `assign` API this target once covered was removed
//! upstream in ThorVG 1.0.6.)

use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use thorvg::Thorvg;

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    json: &'a [u8],
    slot_json: &'a str,
    /// Use the wrapper-returned slot id, or an attacker-chosen id?
    use_returned_slot_id: bool,
    raw_slot_id: u32,
    marker: &'a str,
    /// Index used for the marker-table lookups below.
    var_ix: u32,
    width: f32,
    height: f32,
}

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input<'_>| {
    ENGINE.with(|engine| {
        let Ok(mut lottie) = engine.lottie_animation() else {
            return;
        };
        // Loading may fail; the slot/marker APIs should still be
        // defined to operate (return Err) without crashing.
        let _ = lottie.load_data(input.json);
        let _ = lottie.set_size(input.width, input.height);

        let returned = lottie.gen_slot(input.slot_json);
        let id = if input.use_returned_slot_id {
            returned.unwrap_or(input.raw_slot_id)
        } else {
            input.raw_slot_id
        };
        let _ = lottie.apply_slot(id);
        let _ = lottie.set_marker(input.marker);

        // Marker query surface — markers_count / marker_name /
        // marker_info exercise an integer index lookup into a
        // possibly-empty marker table.
        let _ = lottie.markers_count();
        let _ = lottie.marker_name(input.var_ix);
        let _ = lottie.marker_info(input.var_ix);
        let _ = lottie.markers();

        // Clean the slot up so the registry doesn't grow across
        // iterations.  `del_slot` on an invalid id is a no-op.
        let _ = lottie.del_slot(id);
    });
});
