//! Safe, idiomatic Rust bindings to the [ThorVG](https://github.com/thorvg/thorvg) vector graphics library.
//!
//! `ThorVG` is a production-ready vector graphics engine supporting SVG, Lottie animations,
//! shapes, text, gradients, effects, and more.
//!
//! # `no_std` Support
//!
//! This crate is `no_std` compatible (requires `alloc`). The `std` feature (enabled by default)
//! adds file I/O APIs that accept [`std::path::Path`] (e.g., [`Picture::load`], [`Text::load_font`]).
//!
//! To use in `no_std`, disable default features:
//! ```toml
//! [dependencies]
//! thorvg = { version = "0.1", default-features = false }
//! ```
//!
//! # Quick Start
//!
//! ```no_run
//! use thorvg::{Thorvg, SwCanvas, Shape, ColorSpace};
//!
//! // Initialize the engine
//! let _guard = Thorvg::init(0).expect("Failed to initialize ThorVG");
//!
//! // Create a canvas with a buffer
//! let mut canvas = SwCanvas::new(Default::default()).expect("Failed to create canvas");
//! let mut buffer = vec![0u32; 800 * 600];
//! canvas
//!     .set_target(&mut buffer, 800, 800, 600, ColorSpace::ABGR8888)
//!     .expect("Failed to set target");
//!
//! // Draw a red rectangle
//! let mut shape = Shape::new();
//! shape.append_rect(0.0, 0.0, 200.0, 200.0, 0.0, 0.0, true);
//! shape.set_fill_color(255, 0, 0, 255).unwrap();
//! canvas.push(shape).unwrap();
//!
//! // Render
//! canvas.draw(true).unwrap();
//! canvas.sync().unwrap();
//! ```

#![no_std]
#![warn(clippy::pedantic)]
// These pedantic lints are too noisy for an FFI wrapper crate
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::match_wildcard_for_single_variants)]

extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

mod error;
mod canvas;
mod paint;
mod shape;
mod gradient;
mod picture;
mod scene;
mod text;
mod animation;
mod lottie;
mod saver;
mod accessor;

pub use error::{Error, Result};
pub use canvas::{SwCanvas, GlCanvas, WgCanvas, WgTargetType, EngineOption, ColorSpace};
pub use paint::{Paint, BlendMethod, MaskMethod, PaintType, Matrix, Point};
pub use shape::{Shape, FillRule, StrokeCap, StrokeJoin};
pub use gradient::{LinearGradient, RadialGradient, ColorStop, FillSpread};
pub use picture::{Picture, FilterMethod};
pub use scene::Scene;
pub use text::{Text, TextWrap, TextMetrics, GlyphMetrics};
pub use animation::Animation;
pub use lottie::{LottieAnimation, Marker};
pub use saver::Saver;
pub use accessor::Accessor;

use thorvg_sys as ffi;

#[cfg(test)]
mod tests;

/// RAII guard for the `ThorVG` engine lifetime.
///
/// The engine is terminated when this guard is dropped.
pub struct Thorvg {
    _private: (),
}

impl Thorvg {
    /// Initialize the `ThorVG` engine.
    ///
    /// `threads` specifies the number of worker threads. Use `0` for single-threaded mode.
    ///
    /// Returns a guard that will terminate the engine when dropped.
    pub fn init(threads: u32) -> Result<Self> {
        let result = unsafe { ffi::tvg_engine_init(threads) };
        Error::from_raw(result)?;
        Ok(Self { _private: () })
    }

    /// Get the `ThorVG` engine version.
    pub fn version() -> Result<(u32, u32, u32, alloc::string::String)> {
        let mut major: u32 = 0;
        let mut minor: u32 = 0;
        let mut micro: u32 = 0;
        let mut version: *const core::ffi::c_char = core::ptr::null();

        let result = unsafe {
            ffi::tvg_engine_version(
                &raw mut major,
                &raw mut minor,
                &raw mut micro,
                &raw mut version,
            )
        };
        Error::from_raw(result)?;

        let version_str = if version.is_null() {
            alloc::string::String::new()
        } else {
            unsafe { core::ffi::CStr::from_ptr(version) }
                .to_string_lossy()
                .into_owned()
        };

        Ok((major, minor, micro, version_str))
    }
}

impl Drop for Thorvg {
    fn drop(&mut self) {
        unsafe {
            ffi::tvg_engine_term();
        }
    }
}
