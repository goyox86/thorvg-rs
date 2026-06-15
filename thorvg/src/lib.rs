//! Safe, idiomatic Rust bindings to the [ThorVG](https://github.com/thorvg/thorvg) vector graphics library.
//!
//! `ThorVG` is a production-ready vector graphics engine supporting SVG, Lottie animations,
//! shapes, text, gradients, effects, and more.
//!
//! These bindings wrap the [`ThorVG` C API](https://www.thorvg.org/c-native);
//! consult it for the authoritative engine semantics. The bindings themselves
//! are a work in progress: not every `ThorVG` API is wrapped yet, and the
//! surface here may change between releases.
//!
//! # `no_std` Support
//!
//! This crate is `no_std` compatible (requires `alloc`). The `std` feature (enabled by default)
//! adds file I/O APIs that accept [`std::path::Path`] (e.g., [`Picture::load`], [`Thorvg::load_font`]).
//!
//! To use in `no_std`, disable default features:
//! ```toml
//! [dependencies]
//! thorvg = { version = "0.4", default-features = false }
//! ```
//!
//! **`no_std` panic policy:** the crate executes user-supplied closures
//! (asset resolvers, accessor visitors) from inside `extern "C"`
//! trampolines invoked by the C++ engine.  In `std` builds the
//! trampolines wrap each closure call in [`std::panic::catch_unwind`]
//! and convert panics to a "failure" return; in `no_std` builds there
//! is no `catch_unwind`.
//!
//! This is sound regardless: a panic in your closure reaches the
//! mandatory `#[panic_handler]`, which diverges (`-> !`) and so cannot
//! unwind back across the trampoline into the C++ frame.  As a second
//! backstop, the `extern "C"` trampolines are `nounwind` (Rust ≥ 1.81),
//! so any forced unwind out of them aborts rather than invoking UB.
//! Building with `panic = "abort"` is nonetheless **recommended** for
//! `no_std`: it makes a panic terminate deterministically instead of
//! depending on `#[panic_handler]` behaviour, and bare-metal targets
//! usually require it to link anyway (no `eh_personality`).
//!
//! # Cargo Features
//!
//! Each feature is forwarded to `thorvg-sys`, controlling which
//! `ThorVG` loaders and capabilities are compiled into the static
//! library. All listed features are enabled by default.
//!
//! | Feature       | Default | Effect                                                              |
//! |---------------|---------|---------------------------------------------------------------------|
//! | `vendored`    | yes     | Builds the bundled `ThorVG` source instead of linking a system one. |
//! | `std`         | yes     | Enables `std` and the `Path`-based file APIs (implies `file-io`).   |
//! | `file-io`     | yes     | Enables `ThorVG`'s file-based loaders and savers.                   |
//! | `threads`     | yes     | Multi-threaded rendering; changes [`Thorvg::init`] to take a thread count. |
//! | `svg`         | yes     | SVG loader.                                                         |
//! | `lottie`      | yes     | Lottie animation loader.                                            |
//! | `png`         | yes     | PNG image loader.                                                   |
//! | `fonts`       | yes     | Scalable font (TTF) support for text.                              |
//! | `expressions` | yes     | Lottie expression evaluation.                                       |
//!
//! # Quick Start
//!
//! ```no_run
//! use thorvg::{Thorvg, ColorSpace};
//!
//! // Initialize the engine — all objects borrow from this guard.
//! // With the `threads` feature (default): `Thorvg::init(threads: u32)`.
//! // Without it (bare-metal builds): `Thorvg::init()` — single-threaded only.
//! let engine = Thorvg::init(0).expect("Failed to initialize ThorVG");
//!
//! // Create a canvas with a buffer
//! let mut canvas = engine.sw_canvas(Default::default()).expect("Failed to create canvas");
//! let mut buffer = vec![0u32; 800 * 600];
//! // Safety: buffer outlives the canvas.
//! unsafe {
//!     canvas
//!         .set_target(&mut buffer, 800, 800, 600, ColorSpace::ABGR8888)
//!         .expect("Failed to set target");
//! }
//!
//! // Draw a red rectangle
//! let mut shape = engine.shape().unwrap();
//! shape.append_rect(thorvg::Rect::new(0.0, 0.0, 200.0, 200.0)).unwrap();
//! shape.set_fill_color(thorvg::Rgba::new(255, 0, 0, 255)).unwrap();
//! canvas.add(shape).unwrap();
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

mod accessor;
mod animation;
mod canvas;
mod color;
mod error;
mod gradient;
mod lottie;
mod paint;
mod path;
mod picture;
mod saver;
mod scene;
mod shape;
mod text;

pub use accessor::{Accessor, BorrowedAccessor};
pub use animation::Animation;
pub use canvas::{
    Canvas, EngineOption, GlCanvas, GlTarget, SwCanvas, WgCanvas, WgTarget, WgTargetType,
};
pub use error::{Error, Result};
pub use gradient::{
    BorrowedGradient, BorrowedLinearGradient, BorrowedRadialGradient, ColorStop, FillSpread,
    LinearGradient, RadialGradient,
};
pub use lottie::{AudioInfo, LottieAnimation, Marker};
pub use paint::{BlendMethod, BorrowedPaint, MaskMethod, Matrix, Paint, PaintType, Point};
pub use path::{Path, PathCommand, Segment, Segments};
pub use picture::{FilterMethod, MimeType, Picture};
pub use saver::Saver;
pub use color::{ColorSpace, Rgb, Rgba};
pub use scene::{BlurBorder, BlurDirection, DropShadow, GaussianBlur, Scene, Tint, Tritone};
pub use shape::{Circle, FillRule, PaintOrder, Rect, Shape, StrokeCap, StrokeJoin};
pub use text::{GlyphMetrics, Text, TextMetrics, TextWrap};

use thorvg_sys as sys;

// Public-API ("black-box") tests live in `tests/integration.rs` — they
// touch no crate-private items, so they belong in `tests/` rather than
// an in-crate `#[cfg(test)]` module.  The one exception is below: the
// no-`threads` `init()` signature can only be exercised under
// `not(feature = "threads")`, which the threads-gated integration suite
// cannot cover, so it stays inline here.
#[cfg(all(test, not(feature = "threads")))]
mod tests_no_threads {
    use super::Thorvg;

    #[test]
    fn init_no_arg_signature() {
        let _engine = Thorvg::init().expect("init() should succeed");
        let (major, _minor, _micro, version_str) =
            Thorvg::version().expect("Failed to get version");
        assert!(major >= 1);
        assert!(!version_str.is_empty());
    }
}

/// RAII guard owning the `ThorVG` engine lifetime.
///
/// Created by [`Thorvg::init`]; the engine is terminated when the
/// guard is dropped. Not `Send` or `Sync` — initialize, use, and drop
/// the engine on the same thread.
///
/// All `ThorVG` objects ([`Shape`], [`SwCanvas`], [`Scene`], etc.) borrow from this
/// guard via a lifetime parameter, ensuring the engine cannot be terminated while
/// any object is alive.
///
/// # Example
///
/// ```no_run
/// use thorvg::{Thorvg, ColorSpace};
///
/// // `init` takes a thread count with the `threads` feature (default)
/// // and takes no arguments when that feature is disabled.
/// let engine = Thorvg::init(0).unwrap();
/// let mut canvas = engine.sw_canvas(Default::default()).unwrap();
/// let mut shape = engine.shape().unwrap();
/// shape.set_fill_color(thorvg::Rgba::new(255, 0, 0, 255)).unwrap();
/// canvas.add(shape).unwrap();
/// ```
pub struct Thorvg {
    _not_send_sync: core::marker::PhantomData<*const ()>,
}

impl Thorvg {
    /// Initializes the `ThorVG` engine and returns an owning guard.
    ///
    /// Available when the `threads` feature is enabled (the default).
    /// `threads` is the number of worker threads to spawn; `0` uses
    /// only the calling thread. When the `threads` feature is
    /// disabled (e.g. bare-metal builds), this function takes no
    /// arguments — see the no-arg variant below.
    ///
    /// The returned [`Thorvg`] guard terminates the engine when
    /// dropped; create all `ThorVG` objects via methods on it.
    ///
    /// The engine uses internal reference counting, so nested
    /// initializations are safe; however, the thread count is fixed
    /// on the first initialization and ignored thereafter.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unknown`] if `ThorVG` fails to build its
    /// version info or initialize its loader manager.
    #[cfg(feature = "threads")]
    pub fn init(threads: u32) -> Result<Self> {
        let result = unsafe { sys::tvg_engine_init(threads) };
        Error::from_raw(result)?;
        Ok(Self {
            _not_send_sync: core::marker::PhantomData,
        })
    }

    /// Initializes the `ThorVG` engine in single-threaded mode and
    /// returns an owning guard.
    ///
    /// Available when the `threads` feature is disabled (e.g.
    /// bare-metal builds). All work runs synchronously on the calling
    /// thread. The returned [`Thorvg`] guard terminates the engine
    /// when dropped; create all `ThorVG` objects via methods on it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unknown`] if `ThorVG` fails to build its
    /// version info or initialize its loader manager.
    #[cfg(not(feature = "threads"))]
    pub fn init() -> Result<Self> {
        let result = unsafe { sys::tvg_engine_init(0) };
        Error::from_raw(result)?;
        Ok(Self {
            _not_send_sync: core::marker::PhantomData,
        })
    }

    /// Returns the `ThorVG` engine version as
    /// `(major, minor, micro, version_string)`.
    ///
    /// The string is formatted `"major.minor.micro"`, or empty if
    /// `ThorVG` reports no version. Unlike the other engine APIs, this
    /// is an associated function and does not require an initialized
    /// engine.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] only if the underlying `tvg_engine_version`
    /// call fails; in practice it always succeeds.
    pub fn version() -> Result<(u32, u32, u32, alloc::string::String)> {
        let mut major: u32 = 0;
        let mut minor: u32 = 0;
        let mut micro: u32 = 0;
        let mut version: *const core::ffi::c_char = core::ptr::null();

        let result = unsafe {
            sys::tvg_engine_version(
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

    // ── Paint factories ────────────────────────────────────────────

    /// Creates a new [`Shape`] bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FailedAllocation`] if `ThorVG` cannot allocate
    /// the shape.
    pub fn shape(&self) -> Result<Shape<'_>> {
        Shape::new()
    }

    /// Creates a new [`Scene`] bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FailedAllocation`] if `ThorVG` cannot allocate
    /// the scene.
    pub fn scene(&self) -> Result<Scene<'_>> {
        Scene::new()
    }

    /// Creates a new [`Picture`] bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FailedAllocation`] if `ThorVG` cannot allocate
    /// the picture.
    pub fn picture(&self) -> Result<Picture<'_>> {
        Picture::new()
    }

    /// Creates a new [`Text`] object bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FailedAllocation`] if `ThorVG` cannot allocate
    /// the text object.
    pub fn text(&self) -> Result<Text<'_>> {
        Text::new()
    }

    // ── Gradient factories ─────────────────────────────────────────

    /// Creates a new [`LinearGradient`] bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FailedAllocation`] if `ThorVG` cannot allocate
    /// the gradient.
    pub fn linear_gradient(&self) -> Result<LinearGradient<'_>> {
        LinearGradient::new()
    }

    /// Creates a new [`RadialGradient`] bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FailedAllocation`] if `ThorVG` cannot allocate
    /// the gradient.
    pub fn radial_gradient(&self) -> Result<RadialGradient<'_>> {
        RadialGradient::new()
    }

    // ── Canvas factories ───────────────────────────────────────────

    /// Creates a new software-rendered [`SwCanvas`] bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unknown`] if `ThorVG` cannot create the canvas.
    pub fn sw_canvas(&self, option: EngineOption) -> Result<SwCanvas<'_>> {
        SwCanvas::new(option)
    }

    /// Creates a new OpenGL-rendered [`GlCanvas`] bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unknown`] if `ThorVG` cannot create the canvas.
    pub fn gl_canvas(&self, option: EngineOption) -> Result<GlCanvas<'_>> {
        GlCanvas::new(option)
    }

    /// Creates a new WebGPU-rendered [`WgCanvas`] bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Unknown`] if `ThorVG` cannot create the canvas.
    pub fn wg_canvas(&self, option: EngineOption) -> Result<WgCanvas<'_>> {
        WgCanvas::new(option)
    }

    // ── Animation factories ────────────────────────────────────────

    /// Creates a new [`Animation`] controller bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FailedAllocation`] if `ThorVG` cannot allocate
    /// the animation.
    pub fn animation(&self) -> Result<Animation<'_>> {
        Animation::new()
    }

    /// Creates a new [`LottieAnimation`] controller bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FailedAllocation`] if `ThorVG` cannot allocate
    /// the animation.
    pub fn lottie_animation(&self) -> Result<LottieAnimation<'_>> {
        LottieAnimation::new()
    }

    // ── Utility factories ──────────────────────────────────────────

    /// Creates a new [`Saver`] bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FailedAllocation`] if `ThorVG` cannot allocate
    /// the saver.
    pub fn saver(&self) -> Result<Saver<'_>> {
        Saver::new()
    }

    // ── Font registry (engine-global) ──────────────────────────────

    /// Loads a font from a file path into the engine's font registry,
    /// keyed by the path.
    ///
    /// `ThorVG` caches the loaded data under `path`, so loading the
    /// same file again reuses it. Fonts persist for the engine's
    /// lifetime or until [`unload_font_from_str`](Self::unload_font_from_str).
    ///
    /// # Runtime requirements
    ///
    /// thorvg reads the file with the C runtime (`fopen`/`fread`), so
    /// this requires a working filesystem at runtime even though it
    /// compiles under `no_std`.  On bare-metal targets with no libc
    /// filesystem it returns an error; embed the font and use
    /// [`load_font_data_static`](Self::load_font_data_static) instead.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidArguments`] if `path` is empty, invalid, or
    ///   contains an interior NUL byte.
    /// - [`Error::NotSupported`] if the file extension is not a
    ///   recognized font format.
    pub fn load_font_from_str(&self, path: &str) -> Result<()> {
        let c_path = alloc::ffi::CString::new(path)?;
        Error::from_raw(unsafe { sys::tvg_font_load(c_path.as_ptr()) })
    }

    /// Loads a font from a filesystem [`Path`](std::path::Path).
    ///
    /// Convenience wrapper over
    /// [`load_font_from_str`](Self::load_font_from_str); the path is
    /// lossily converted to UTF-8.
    ///
    /// # Errors
    ///
    /// Same conditions as
    /// [`load_font_from_str`](Self::load_font_from_str).
    #[cfg(feature = "std")]
    pub fn load_font<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        self.load_font_from_str(&path.as_ref().to_string_lossy())
    }

    /// Unloads a previously loaded font by path, releasing its
    /// registry entry.
    ///
    /// As with [`load_font_from_str`](Self::load_font_from_str), the
    /// path keys into thorvg's registry; this needs the same working
    /// filesystem at runtime under `no_std`. If the font is still in
    /// use it is not freed immediately.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidArguments`] if `path` contains an interior
    ///   NUL byte.
    /// - [`Error::InsufficientCondition`] if the font loader is not
    ///   initialized (no font has been loaded).
    pub fn unload_font_from_str(&self, path: &str) -> Result<()> {
        let c_path = alloc::ffi::CString::new(path)?;
        Error::from_raw(unsafe { sys::tvg_font_unload(c_path.as_ptr()) })
    }

    /// Unloads a previously loaded font by
    /// [`Path`](std::path::Path).
    ///
    /// Convenience wrapper over
    /// [`unload_font_from_str`](Self::unload_font_from_str); the path
    /// is lossily converted to UTF-8.
    ///
    /// # Errors
    ///
    /// Same conditions as
    /// [`unload_font_from_str`](Self::unload_font_from_str).
    #[cfg(feature = "std")]
    pub fn unload_font<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        self.unload_font_from_str(&path.as_ref().to_string_lossy())
    }

    /// Loads a font from memory, copying `data` into thorvg's
    /// internal registry.
    ///
    /// The font is registered under `name` and remains usable for the
    /// engine's lifetime. `mimetype` (e.g. `"ttf"`) selects the
    /// loader; `None` lets thorvg detect it automatically. Use this
    /// variant for owned or non-`'static` buffers; for zero-copy
    /// registration of `'static` data (e.g. `include_bytes!(...)`),
    /// use [`load_font_data_static`](Self::load_font_data_static).
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidArguments`] if `name` is empty, `data` is
    ///   empty, or `name`/`mimetype` contains an interior NUL byte.
    /// - [`Error::NotSupported`] if the font format cannot be loaded.
    pub fn load_font_data(&self, name: &str, data: &[u8], mimetype: Option<&str>) -> Result<()> {
        load_font_data_inner(name, data, mimetype, /* copy = */ true)
    }

    /// Loads a font from `'static` memory without copying.
    ///
    /// thorvg stores the pointer to `data` in its global font
    /// registry and dereferences it on every subsequent text-render
    /// call, so the buffer must outlive the engine — the `'static`
    /// bound enforces this at compile time.  Typical use:
    /// `engine.load_font_data_static("Roboto", include_bytes!("Roboto.ttf"), None)`.
    ///
    /// # Errors
    ///
    /// Same conditions as
    /// [`load_font_data`](Self::load_font_data).
    ///
    /// # Compile-time safety
    ///
    /// The `'static` bound rejects local buffers at the type level:
    ///
    /// ```compile_fail,E0597
    /// let engine = thorvg::Thorvg::init(0).unwrap();
    /// let local: Vec<u8> = vec![0; 32];
    /// engine.load_font_data_static("nope", &local, None).unwrap();
    /// // error[E0597]: `local` does not live long enough
    /// ```
    pub fn load_font_data_static(
        &self,
        name: &str,
        data: &'static [u8],
        mimetype: Option<&str>,
    ) -> Result<()> {
        load_font_data_inner(name, data, mimetype, /* copy = */ false)
    }

    /// Creates a new [`Accessor`] bound to this engine.
    ///
    /// # Errors
    ///
    /// Returns [`Error::FailedAllocation`] if `ThorVG` cannot allocate
    /// the accessor.
    pub fn accessor(&self) -> Result<Accessor<'_>> {
        Accessor::new()
    }
}

impl Drop for Thorvg {
    fn drop(&mut self) {
        unsafe {
            sys::tvg_engine_term();
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn load_font_data_inner(name: &str, data: &[u8], mimetype: Option<&str>, copy: bool) -> Result<()> {
    let c_name = alloc::ffi::CString::new(name)?;
    let c_mime = mimetype.map(alloc::ffi::CString::new).transpose()?;
    let mime_ptr = c_mime.as_ref().map_or(core::ptr::null(), |c| c.as_ptr());
    Error::from_raw(unsafe {
        sys::tvg_font_load_data(
            c_name.as_ptr(),
            data.as_ptr().cast::<core::ffi::c_char>(),
            data.len() as u32,
            mime_ptr,
            copy,
        )
    })
}
