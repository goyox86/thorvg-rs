#![no_main]

//! Fuzz target for `Saver::save_to_str` and
//! `Saver::save_animation_to_str`.
//!
//! Exercises:
//!
//!   * the path → `CString` conversion (rejects embedded NUL,
//!     enforces UTF-8 → C-string boundary);
//!   * the C-side file extension dispatcher (.tvg / .png / .gif …
//!     branch in `tvgSaver.cpp`) on arbitrary path strings;
//!   * the `into_raw` ownership transfer on
//!     `save_animation_to_str` — the C side `delete`s the animation
//!     on failure and hands it off to the save module on success;
//!     a refcount-handling regression here would double-free.
//!
//! The actual file write is redirected to a per-iteration scratch
//! file under `std::env::temp_dir()`; the fuzz-supplied "path" is
//! appended as an extension hint so the dispatcher still picks a
//! branch based on user input, without letting the fuzzer escape
//! the tempdir.

use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use std::sync::atomic::{AtomicU64, Ordering};
use thorvg::Thorvg;

#[derive(Arbitrary, Debug)]
struct Input<'a> {
    /// User-controlled extension hint — drives the C-side format
    /// dispatcher.  Sanitised to a filename-safe shape before use.
    ext_hint: &'a str,
    quality: u32,
    fps: u32,
    /// Which save flavour to exercise.
    save_animation: bool,
    /// Whether to include the animation's load_data step (only
    /// relevant when `save_animation`).
    load_anim_data: bool,
    anim_data: &'a [u8],
}

thread_local! {
    static ENGINE: Thorvg = Thorvg::init(0).expect("thorvg init");
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Build a per-iteration tempfile path whose **basename** comes from
/// the fuzzer (so the C extension dispatcher sees user-controlled
/// input) but whose **directory** is fixed at the process tempdir
/// (so the fuzzer cannot escape it).  Path separators in the hint
/// are stripped.
fn temp_path(hint: &str) -> String {
    let cleaned: String = hint
        .chars()
        .filter(|c| !matches!(c, '/' | '\\' | '\0'))
        .take(32)
        .collect();
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir();
    format!("{}/tvg-fuzz-{n}-{cleaned}", dir.display())
}

fuzz_target!(|input: Input<'_>| {
    ENGINE.with(|engine| {
        let Ok(mut saver) = engine.saver() else {
            return;
        };
        let path = temp_path(input.ext_hint);
        if input.save_animation {
            let Ok(mut anim) = engine.animation() else {
                return;
            };
            if input.load_anim_data {
                let _ = anim
                    .picture_mut()
                    .load_data(input.anim_data, thorvg::MimeType::Lottie, None);
            }
            let _ = saver.save_animation_to_str(anim, &path, input.quality, input.fps);
        } else {
            let Ok(mut shape) = engine.shape() else {
                return;
            };
            let _ = shape.append_rect(0.0, 0.0, 10.0, 10.0, 0.0, 0.0, true);
            let _ = shape.set_fill_color(255, 0, 0, 255);
            let _ = saver.save_to_str(shape, &path, input.quality);
        }
        let _ = saver.sync();
        // Clean up immediately so the tempdir doesn't fill the
        // disk over millions of iterations.
        let _ = std::fs::remove_file(&path);
    });
});
