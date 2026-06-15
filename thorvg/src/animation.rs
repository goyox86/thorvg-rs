//! Frame and playback control for animatable content such as Lottie.
//!
//! Wraps the [`ThorVG` C API](https://www.thorvg.org/c-native).

use crate::error::{Error, Result};
use crate::picture::Picture;
use thorvg_sys as sys;

/// Controller for animatable content such as Lottie.
///
/// Drives frame selection, duration queries, and playback segments over
/// the [`Picture`] the animation owns. Load animation data into that
/// picture (via [`picture_mut`](Self::picture_mut)), add it to a canvas,
/// then advance frames with [`set_frame`](Self::set_frame).
///
/// The lifetime `'eng` ties this animation to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create animations via [`Thorvg::animation()`](crate::Thorvg::animation).
///
/// # Thread Safety
///
/// `Animation` is [`Send`] but not [`Sync`]: you may move it to another
/// thread, but you must not share references across threads.
pub struct Animation<'eng> {
    raw: sys::Tvg_Animation,
    // Borrowed picture handle owned by the C-side animation.  We
    // build it once in `new` so [`Animation::picture`] can return a
    // stable `&mut Picture<'_>` rather than reconstructing a fresh
    // wrapper on every call.  The wrapper is `owned: false`, so
    // its Drop is a no-op — the C animation handle owns the actual
    // picture and releases it via `tvg_animation_del`.
    picture: Picture<'eng>,
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

impl<'eng> Animation<'eng> {
    /// Returns the raw `Tvg_Animation` handle.
    pub(crate) fn raw(&self) -> sys::Tvg_Animation {
        self.raw
    }

    /// Consumes the Animation and returns the raw `Tvg_Animation`.
    ///
    /// Suppresses `Drop`, so the caller is responsible for the C
    /// handle's lifetime.  Used by `Saver::save_animation_to_str`,
    /// where the C side takes ownership of the handle on every exit
    /// path under `refCnt <= 1` (see `tvgSaver.cpp:142-172`) —
    /// keeping the Rust `Drop` active would double-free.
    pub(crate) fn into_raw(self) -> sys::Tvg_Animation {
        let me = core::mem::ManuallyDrop::new(self);
        me.raw
    }

    /// Wraps an existing raw animation pointer.
    ///
    /// # Safety
    /// `raw` must be a valid, owned `Tvg_Animation`.
    pub(crate) unsafe fn from_raw(raw: sys::Tvg_Animation) -> Self {
        let pic_raw = unsafe { sys::tvg_animation_get_picture(raw) };
        assert!(!pic_raw.is_null(), "animation has no picture");
        Self {
            raw,
            picture: unsafe { Picture::from_raw(pic_raw, false) },
            _engine: core::marker::PhantomData,
        }
    }

    /// Creates a new Animation object.
    pub(crate) fn new() -> Result<Self> {
        let raw = unsafe { sys::tvg_animation_new() };
        if raw.is_null() {
            return Err(Error::FailedAllocation);
        }
        // SAFETY: `raw` was just returned non-null by tvg_animation_new.
        Ok(unsafe { Self::from_raw(raw) })
    }

    /// Returns the [`Picture`] this animation owns.
    ///
    /// The wrapper is built once at construction and re-borrowed on each
    /// call, so this performs no C call and no allocation. The picture is
    /// owned by the animation and is released when the animation is
    /// dropped.
    pub fn picture(&self) -> &Picture<'eng> {
        &self.picture
    }

    /// Returns a mutable borrow of the [`Picture`] this animation owns.
    ///
    /// Use this to load animation data (e.g. via
    /// [`Picture::load_data`](crate::Picture::load_data)) and to configure
    /// render size before playback.
    pub fn picture_mut(&mut self) -> &mut Picture<'eng> {
        &mut self.picture
    }

    /// Displays the frame at the given (possibly fractional) index.
    ///
    /// Frame numbering is zero-based; `frame` should be less than
    /// [`total_frame`](Self::total_frame). Fractional values are
    /// supported and interpolated by the loader.
    ///
    /// For efficiency, `ThorVG` ignores the update when `frame` differs
    /// from the current frame by less than `0.001`.
    ///
    /// # Errors
    ///
    /// - [`Error::InsufficientCondition`] if `frame` is the same as the
    ///   current frame (difference below `0.001`).
    /// - [`Error::NotSupported`] if the loaded picture data is not
    ///   animatable.
    pub fn set_frame(&mut self, frame: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_animation_set_frame(self.raw, frame) })
    }

    /// Returns the current frame index.
    ///
    /// The value lies between `0` and [`total_frame`](Self::total_frame)
    /// minus one, or `0` if the picture is not properly configured.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] only if `ThorVG` rejects the
    /// output pointer, which cannot occur through this wrapper.
    pub fn frame(&self) -> Result<f32> {
        let mut frame: f32 = 0.0;
        Error::from_raw(unsafe { sys::tvg_animation_get_frame(self.raw, &raw mut frame) })?;
        Ok(frame)
    }

    /// Returns the total number of frames in the animation.
    ///
    /// Returns `0.0` if the picture is not properly configured.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] only if `ThorVG` rejects the
    /// output pointer, which cannot occur through this wrapper.
    pub fn total_frame(&self) -> Result<f32> {
        let mut cnt: f32 = 0.0;
        Error::from_raw(unsafe { sys::tvg_animation_get_total_frame(self.raw, &raw mut cnt) })?;
        Ok(cnt)
    }

    /// Returns the animation duration in seconds.
    ///
    /// Returns `0.0` if the picture is not properly configured.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] only if `ThorVG` rejects the
    /// output pointer, which cannot occur through this wrapper.
    pub fn duration(&self) -> Result<f32> {
        let mut duration: f32 = 0.0;
        Error::from_raw(unsafe { sys::tvg_animation_get_duration(self.raw, &raw mut duration) })?;
        Ok(duration)
    }

    /// Restricts playback to the frame range `[begin, end]`.
    ///
    /// `begin` and `end` are frame indices, not normalized values, and
    /// must lie within `0.0` to [`total_frame`](Self::total_frame).
    /// After setting a segment, [`total_frame`](Self::total_frame) and
    /// [`duration`](Self::duration) are remapped so the segment spans
    /// the full range. A marker set via
    /// [`LottieAnimation::set_marker`](crate::LottieAnimation::set_marker)
    /// takes precedence and causes this range to be ignored.
    ///
    /// # Errors
    ///
    /// - [`Error::InsufficientCondition`] if no animation is loaded.
    /// - [`Error::InvalidArguments`] if `begin` is greater than `end`.
    /// - [`Error::NotSupported`] if the content is not animatable.
    pub fn set_segment(&mut self, begin: f32, end: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_animation_set_segment(self.raw, begin, end) })
    }

    /// Returns the current playback segment as `(begin, end)` frame indices.
    ///
    /// # Errors
    ///
    /// - [`Error::InsufficientCondition`] if no animation is loaded.
    /// - [`Error::NotSupported`] if the content is not animatable.
    pub fn segment(&self) -> Result<(f32, f32)> {
        let (mut begin, mut end) = (0.0f32, 0.0f32);
        Error::from_raw(unsafe {
            sys::tvg_animation_get_segment(self.raw, &raw mut begin, &raw mut end)
        })?;
        Ok((begin, end))
    }
}

impl Drop for Animation<'_> {
    fn drop(&mut self) {
        unsafe {
            sys::tvg_animation_del(self.raw);
        }
    }
}

impl core::fmt::Debug for Animation<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Animation").finish_non_exhaustive()
    }
}
