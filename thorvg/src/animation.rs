use crate::error::{Error, Result};
use crate::picture::Picture;
use thorvg_sys as ffi;

/// An animation controller for animated content (e.g., Lottie).
///
/// The lifetime `'eng` ties this animation to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create animations via [`Thorvg::animation()`](crate::Thorvg::animation).
///
/// # Thread Safety
///
/// `Animation` is [`Send`] but not [`Sync`]: you may move it to another
/// thread, but you must not share references across threads.
pub struct Animation<'eng> {
    raw: ffi::Tvg_Animation,
    _engine: core::marker::PhantomData<&'eng ()>,
}

// SAFETY: `Animation` exclusively owns a heap-allocated ThorVG animation
// handle (`Tvg_Animation`).  The C++ implementation guards shared global
// state (loader registry, memory pool) with internal mutexes.  Per-object
// state is only accessed through `&mut self`.  Transferring sole ownership
// to another thread is safe.
//
// `Animation` is intentionally `!Sync`: the raw pointer field prevents the
// auto-`Sync` impl, which is correct — concurrent `&`-access to the same
// C handle would be a data race.
unsafe impl Send for Animation<'_> {}

impl Animation<'_> {
    /// Returns the raw `Tvg_Animation` handle.
    pub(crate) fn raw(&self) -> ffi::Tvg_Animation {
        self.raw
    }

    /// Wraps an existing raw animation pointer.
    ///
    /// # Safety
    /// `raw` must be a valid, owned `Tvg_Animation`.
    pub(crate) unsafe fn from_raw(raw: ffi::Tvg_Animation) -> Self {
        Self {
            raw,
            _engine: core::marker::PhantomData,
        }
    }

    /// Creates a new Animation object.
    pub(crate) fn new() -> Self {
        let raw = unsafe { ffi::tvg_animation_new() };
        assert!(!raw.is_null(), "failed to create Animation");
        Self {
            raw,
            _engine: core::marker::PhantomData,
        }
    }

    /// Returns the associated Picture object.
    ///
    /// The returned Picture is **not owned** — it is managed by the Animation.
    pub fn picture(&self) -> Picture<'_> {
        let raw = unsafe { ffi::tvg_animation_get_picture(self.raw) };
        assert!(!raw.is_null(), "animation has no picture");
        unsafe { Picture::from_raw(raw, false) }
    }

    /// Sets the current animation frame.
    pub fn set_frame(&mut self, frame: f32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_animation_set_frame(self.raw, frame) })
    }

    /// Gets the current animation frame.
    pub fn frame(&self) -> Result<f32> {
        let mut frame: f32 = 0.0;
        Error::from_raw(unsafe { ffi::tvg_animation_get_frame(self.raw, &raw mut frame) })?;
        Ok(frame)
    }

    /// Gets the total number of frames.
    pub fn total_frame(&self) -> Result<f32> {
        let mut cnt: f32 = 0.0;
        Error::from_raw(unsafe { ffi::tvg_animation_get_total_frame(self.raw, &raw mut cnt) })?;
        Ok(cnt)
    }

    /// Gets the animation duration in seconds.
    pub fn duration(&self) -> Result<f32> {
        let mut duration: f32 = 0.0;
        Error::from_raw(unsafe { ffi::tvg_animation_get_duration(self.raw, &raw mut duration) })?;
        Ok(duration)
    }

    /// Sets the playback segment.
    pub fn set_segment(&mut self, begin: f32, end: f32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_animation_set_segment(self.raw, begin, end) })
    }

    /// Gets the current playback segment.
    pub fn segment(&self) -> Result<(f32, f32)> {
        let (mut begin, mut end) = (0.0f32, 0.0f32);
        Error::from_raw(unsafe {
            ffi::tvg_animation_get_segment(self.raw, &raw mut begin, &raw mut end)
        })?;
        Ok((begin, end))
    }
}

impl Drop for Animation<'_> {
    fn drop(&mut self) {
        unsafe {
            ffi::tvg_animation_del(self.raw);
        }
    }
}

impl core::fmt::Debug for Animation<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Animation").finish_non_exhaustive()
    }
}
