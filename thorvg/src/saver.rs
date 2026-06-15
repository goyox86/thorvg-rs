//! Exporting paints and animations to files (`.tvg`, `.gif`, …).
//!
//! Wraps the [`ThorVG` C API](https://www.thorvg.org/c-native).

use alloc::ffi::CString;

use crate::animation::Animation;
use crate::error::{Error, Result};
use crate::paint::Paint;
use thorvg_sys as sys;

/// Exports paint objects or animations to files.
///
/// The output format is chosen from the file extension (e.g. `.tvg`,
/// `.gif`); the supported set depends on how `ThorVG` was packaged. A
/// saved file can later be reloaded with the [`Picture`](crate::Picture)
/// module.
///
/// # Asynchronous saving
///
/// Saving runs asynchronously when the engine was initialized with more
/// than one thread, so a successful return from a `save*` method does not
/// guarantee the file is fully written. Call [`sync`](Self::sync) to block
/// until the task completes. With a single-threaded engine the save is
/// synchronous, but calling [`sync`](Self::sync) afterward is still
/// correct.
///
/// The lifetime `'eng` ties this saver to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create savers via [`Thorvg::saver()`](crate::Thorvg::saver).
pub struct Saver<'eng> {
    raw: sys::Tvg_Saver,
    _engine: core::marker::PhantomData<&'eng ()>,
}

// SAFETY: Same rationale as other ThorVG handle types — exclusive
// ownership of a C heap object; global state is mutex-protected.
unsafe impl Send for Saver<'_> {}

impl Saver<'_> {
    /// Creates a new Saver object.
    pub(crate) fn new() -> Result<Self> {
        let raw = unsafe { sys::tvg_saver_new() };
        if raw.is_null() {
            return Err(Error::FailedAllocation);
        }
        Ok(Self {
            raw,
            _engine: core::marker::PhantomData,
        })
    }

    /// Saves a paint object to the file at `path`.
    ///
    /// `quality` is the encoder quality level, `0` (minimum) to `100`
    /// (maximum, recommended). The save may complete asynchronously; call
    /// [`sync`](Self::sync) to ensure the file is flushed (see the
    /// [type-level docs](Self)).
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidArguments`] if `path` contains an interior NUL
    ///   byte.
    /// - [`Error::InsufficientCondition`] if another save is already in
    ///   progress.
    /// - [`Error::NotSupported`] if the extension is unknown or the format
    ///   is unsupported.
    /// - [`Error::Unknown`] if the paint is empty.
    ///
    /// # Runtime requirements
    ///
    /// thorvg writes the file with the C runtime (`fopen`/`fwrite`),
    /// so this requires a working filesystem at runtime even though
    /// it compiles under `no_std`.  On bare-metal targets with no
    /// libc filesystem it returns an error.  There is no in-memory
    /// alternative on the C API side; embedded targets that need
    /// serialised output must provide a libc-style file backend
    /// (newlib / picolibc / a custom syscall layer).
    ///
    /// Requires the `file-io` feature on `thorvg-sys` (enabled by
    /// default; pulled in transitively by the `std` feature on this
    /// crate).
    pub fn save_to_str<P: Paint>(&mut self, paint: P, path: &str, quality: u32) -> Result<()> {
        let c_path = CString::new(path)?;
        Error::from_raw(unsafe {
            sys::tvg_saver_save_paint(self.raw, paint.into_raw(), c_path.as_ptr(), quality)
        })
    }

    /// Saves a paint object to a [`Path`](std::path::Path).
    ///
    /// Convenience wrapper over [`save_to_str`](Self::save_to_str); see it
    /// for `quality`, async behavior, and errors.
    #[cfg(feature = "std")]
    pub fn save<P: Paint, Q: AsRef<std::path::Path>>(
        &mut self,
        paint: P,
        path: Q,
        quality: u32,
    ) -> Result<()> {
        self.save_to_str(paint, &path.as_ref().to_string_lossy(), quality)
    }

    /// Saves an animation to the file at `path`.
    ///
    /// `quality` is the encoder quality level, `0` to `100`; `fps` is the
    /// target frame rate, or `0` to keep the source frame rate. The save
    /// may complete asynchronously; call [`sync`](Self::sync) to ensure
    /// the file is flushed (see the [type-level docs](Self)).
    ///
    /// Consumes the [`Animation`]: the C side takes ownership of the
    /// handle on every exit path where `picture()->refCnt() <= 1`
    /// (the case for a freshly-constructed [`Animation`]) — it is
    /// `delete`d on failure and handed off to the save module on success,
    /// so keeping the Rust `Drop` active would double-free. Same shape as
    /// `Paint::set_mask` / `Paint::set_clip`.
    ///
    /// # Errors
    ///
    /// - [`Error::InvalidArguments`] if `path` contains an interior NUL
    ///   byte.
    /// - [`Error::InsufficientCondition`] if another save is in progress
    ///   or the animation has no frames.
    /// - [`Error::NotSupported`] if the extension is unknown or the format
    ///   is unsupported.
    /// - [`Error::Unknown`] if the animation's paint is empty.
    ///
    /// # Runtime requirements
    ///
    /// Same filesystem caveat as
    /// [`save_to_str`](Self::save_to_str): thorvg serialises through
    /// the C runtime (`fopen`/`fwrite`), so a libc-backed filesystem
    /// must exist at runtime even though the function compiles under
    /// `no_std`.  On bare-metal targets without one this returns an
    /// error.  Requires the `file-io` feature on `thorvg-sys`
    /// (enabled by default; pulled in transitively by `std`).
    pub fn save_animation_to_str(
        &mut self,
        animation: Animation<'_>,
        path: &str,
        quality: u32,
        fps: u32,
    ) -> Result<()> {
        let c_path = CString::new(path)?;
        Error::from_raw(unsafe {
            sys::tvg_saver_save_animation(
                self.raw,
                animation.into_raw(),
                c_path.as_ptr(),
                quality,
                fps,
            )
        })
    }

    /// Saves an animation to a [`Path`](std::path::Path).
    ///
    /// Convenience wrapper over
    /// [`save_animation_to_str`](Self::save_animation_to_str); see it for
    /// `quality`/`fps`, ownership, async behavior, and errors. Consumes
    /// the [`Animation`].
    #[cfg(feature = "std")]
    pub fn save_animation<P: AsRef<std::path::Path>>(
        &mut self,
        animation: Animation<'_>,
        path: P,
        quality: u32,
        fps: u32,
    ) -> Result<()> {
        self.save_animation_to_str(animation, &path.as_ref().to_string_lossy(), quality, fps)
    }

    /// Blocks until the pending save task has finished.
    ///
    /// Call this after a `save*` method to guarantee the output file is
    /// fully written when saving runs asynchronously (multi-threaded
    /// engine). It is safe to call when the save was synchronous.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InsufficientCondition`] if no save task is
    /// running.
    pub fn sync(&mut self) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_saver_sync(self.raw) })
    }
}

impl Drop for Saver<'_> {
    fn drop(&mut self) {
        unsafe {
            sys::tvg_saver_del(self.raw);
        }
    }
}

impl core::fmt::Debug for Saver<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Saver").finish_non_exhaustive()
    }
}
