#![no_main]

//! Fuzz target for `Accessor::for_each`.
//!
//! Drives the scene-tree traversal trampoline on a `Picture`
//! loaded from arbitrary bytes (so the C side iterates a tree of
//! unknown shape and size), with a closure that:
//!
//!   * may return `false` at an arbitrary visit index to test
//!     early-termination from the Rust side;
//!   * may **panic** at an arbitrary visit index to exercise
//!     the `catch_unwind` guard inside `invoke_user` — unwinding
//!     across the `extern "C"` boundary into C++ would be UB.
//!
//! Inside the closure the read-side `BorrowedAccessor::get_name`
//! and `BorrowedPaint::{id, paint_type, opacity, bounds}` are
//! queried so any stale-pointer or wrong-lifetime bug surfaces as
//! a crash.

use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use thorvg::{MimeType, Thorvg};

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    /// Bytes loaded into the picture; the loader-manager picks a
    /// concrete decoder from `mime`.
    data: &'a [u8],
    /// Whether to load as SVG (the parser most likely to produce a
    /// non-empty tree for fuzz-generated inputs).
    as_svg: bool,
    /// 1-in-N chance of stopping iteration at each step (0 = never).
    stop_at: u8,
    /// 1-in-N chance of panicking at each step (0 = never panic).
    panic_at: u8,
    /// Whether to mark the picture accessible so `get_name` has a
    /// populated index.
    accessible: bool,
}

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

fuzz_target!(|input: Input<'_>| {
    ENGINE.with(|engine| {
        let Ok(mut pic) = engine.picture() else {
            return;
        };
        let mime = if input.as_svg {
            MimeType::Svg
        } else {
            MimeType::Lottie
        };
        if pic.load_data(input.data, mime, None).is_err() {
            return;
        }
        let _ = pic.set_accessible(input.accessible);
        let Ok(mut acc) = engine.accessor() else {
            return;
        };
        let stop_at = input.stop_at;
        let panic_at = input.panic_at;
        let mut visits: u32 = 0;
        let _ = acc.for_each(&pic, |a, p| {
            visits = visits.wrapping_add(1);
            // Touch every read-side method on both borrows.
            let _ = a.get_name(p.id());
            let _ = p.paint_type();
            let _ = p.opacity();
            let _ = p.bounds();
            if panic_at != 0 && visits.is_multiple_of(u32::from(panic_at)) {
                // Must be absorbed by `catch_unwind`; unwinding into
                // C++ would be UB.
                panic!("fuzz-induced accessor panic");
            }
            if stop_at != 0 && visits.is_multiple_of(u32::from(stop_at)) {
                return false;
            }
            true
        });
    });
});
