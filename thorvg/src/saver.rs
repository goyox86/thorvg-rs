use alloc::ffi::CString;

use crate::animation::Animation;
use crate::error::{Error, Result};
use crate::paint::Paint;
use thorvg_sys as sys;

/// Exports paint objects or animations to files.
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

    /// Saves a paint object to a file path string.
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

    /// Saves a paint object to a file.
    #[cfg(feature = "std")]
    pub fn save<P: Paint, Q: AsRef<std::path::Path>>(
        &mut self,
        paint: P,
        path: Q,
        quality: u32,
    ) -> Result<()> {
        self.save_to_str(paint, &path.as_ref().to_string_lossy(), quality)
    }

    /// Saves an animation to a file path string.
    ///
    /// Consumes the `Animation` — the C side takes ownership of the
    /// handle on every exit path where `picture()->refCnt() <= 1`
    /// (which is the case for a freshly-constructed `Animation`).
    /// See `tvgSaver.cpp:142-172`: the animation is `delete`d on
    /// failure, and handed off to the save module on success.
    /// Keeping the Rust `Drop` active would double-free.  Same
    /// shape as `Paint::set_mask` / `Paint::set_clip`.
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

    /// Saves an animation to a file.  Consumes the `Animation` — see
    /// [`save_animation_to_str`](Self::save_animation_to_str).
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

    /// Waits for the saving task to finish.
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
