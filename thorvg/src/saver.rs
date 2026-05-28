use alloc::ffi::CString;

use crate::animation::Animation;
use crate::error::{Error, Result};
use crate::paint::Paint;
use thorvg_sys as ffi;

/// Exports paint objects or animations to files.
///
/// The lifetime `'eng` ties this saver to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create savers via [`Thorvg::saver()`](crate::Thorvg::saver).
pub struct Saver<'eng> {
    raw: ffi::Tvg_Saver,
    _engine: core::marker::PhantomData<&'eng ()>,
}

// SAFETY: Same rationale as other ThorVG handle types — exclusive
// ownership of a C heap object; global state is mutex-protected.
unsafe impl Send for Saver<'_> {}

impl Saver<'_> {
    /// Creates a new Saver object.
    pub(crate) fn new() -> Self {
        let raw = unsafe { ffi::tvg_saver_new() };
        assert!(!raw.is_null(), "failed to create Saver");
        Self {
            raw,
            _engine: core::marker::PhantomData,
        }
    }

    /// Saves a paint object to a file path string.
    pub fn save_to_str<P: Paint>(&mut self, paint: P, path: &str, quality: u32) -> Result<()> {
        let c_path = CString::new(path).map_err(|_| Error::InvalidArguments)?;
        Error::from_raw(unsafe {
            ffi::tvg_saver_save_paint(self.raw, paint.into_raw(), c_path.as_ptr(), quality)
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
    pub fn save_animation_to_str(
        &mut self,
        animation: &Animation<'_>,
        path: &str,
        quality: u32,
        fps: u32,
    ) -> Result<()> {
        let c_path = CString::new(path).map_err(|_| Error::InvalidArguments)?;
        Error::from_raw(unsafe {
            ffi::tvg_saver_save_animation(self.raw, animation.raw(), c_path.as_ptr(), quality, fps)
        })
    }

    /// Saves an animation to a file.
    #[cfg(feature = "std")]
    pub fn save_animation<P: AsRef<std::path::Path>>(
        &mut self,
        animation: &Animation<'_>,
        path: P,
        quality: u32,
        fps: u32,
    ) -> Result<()> {
        self.save_animation_to_str(animation, &path.as_ref().to_string_lossy(), quality, fps)
    }

    /// Waits for the saving task to finish.
    pub fn sync(&mut self) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_saver_sync(self.raw) })
    }
}

impl Drop for Saver<'_> {
    fn drop(&mut self) {
        unsafe {
            ffi::tvg_saver_del(self.raw);
        }
    }
}

impl core::fmt::Debug for Saver<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Saver").finish_non_exhaustive()
    }
}
