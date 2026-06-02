use alloc::ffi::CString;
use alloc::string::String;
use alloc::vec::Vec;

use crate::animation::Animation;
use crate::error::{Error, Result};
use thorvg_sys as sys;

/// Marker information for a Lottie animation segment.
#[derive(Debug, Clone)]
pub struct Marker {
    /// The marker name.
    pub name: String,
    /// The starting frame.
    pub begin: f32,
    /// The ending frame.
    pub end: f32,
}

/// A Lottie animation controller with Lottie-specific extensions.
///
/// This wraps [`Animation`] and adds slots, markers, tweening,
/// expression variables, and quality control.
///
/// All base [`Animation`] methods are available via `Deref`.
///
/// # Example
///
/// ```no_run
/// use thorvg::{Thorvg, ColorSpace};
///
/// // `Thorvg::init` takes a thread count under the default `threads` feature
/// // and takes no arguments when that feature is disabled.
/// let engine = Thorvg::init(0).unwrap();
/// let mut canvas = engine.sw_canvas(Default::default()).unwrap();
/// let mut buffer = vec![0u32; 800 * 600];
/// unsafe {
///     canvas
///         .set_target(&mut buffer, 800, 800, 600, ColorSpace::ABGR8888)
///         .unwrap()
/// };
///
/// let mut lottie = engine.lottie_animation();
/// let mut pic = lottie.picture();
/// pic.load_from_str("animation.json").ok();
/// ```
///
/// The lifetime `'eng` ties this animation to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create Lottie animations via
/// [`Thorvg::lottie_animation()`](crate::Thorvg::lottie_animation).
pub struct LottieAnimation<'eng> {
    inner: Animation<'eng>,
}

impl LottieAnimation<'_> {
    /// Creates a new Lottie animation object.
    pub(crate) fn new() -> Self {
        let raw = unsafe { sys::tvg_lottie_animation_new() };
        assert!(!raw.is_null(), "failed to create LottieAnimation");
        Self {
            inner: unsafe { Animation::from_raw(raw) },
        }
    }

    // ── Convenience loaders ──────────────────────────────────────────

    /// Load a Lottie animation from a JSON byte slice.
    ///
    /// Convenience wrapper around [`Picture::load_data`](crate::Picture::load_data) that uses the
    /// `"lottie"` mimetype and copies the data. For advanced use cases
    /// (e.g. external asset `resource_path`), access the picture
    /// directly via [`Animation::picture`].
    pub fn load_data(&mut self, data: &[u8]) -> Result<()> {
        let mut pic = self.picture();
        pic.load_data(data, crate::picture::MimeType::Lottie, None, true)
    }

    /// Load a Lottie animation from a file path string.
    ///
    /// Convenience wrapper around [`Picture::load_from_str`](crate::Picture::load_from_str).
    #[cfg(feature = "std")]
    pub fn load_file(&mut self, path: &str) -> Result<()> {
        let mut pic = self.picture();
        pic.load_from_str(path)
    }

    /// Set the render size of the animation picture.
    ///
    /// Convenience wrapper around [`Picture::set_size`](crate::Picture::set_size).
    pub fn set_size(&mut self, w: f32, h: f32) -> Result<()> {
        let mut pic = self.picture();
        pic.set_size(w, h)
    }

    // ── Slots ──────────────────────────────────────────────────────

    /// Generates a new slot from the given Lottie slot data (JSON format).
    ///
    /// Returns the generated slot ID, or `None` on failure.
    pub fn gen_slot(&mut self, slot_json: &str) -> Option<u32> {
        let c_slot = CString::new(slot_json).ok()?;
        let id = unsafe { sys::tvg_lottie_animation_gen_slot(self.inner.raw(), c_slot.as_ptr()) };
        if id == 0 {
            None
        } else {
            Some(id)
        }
    }

    /// Applies a previously generated slot to the animation.
    ///
    /// Pass `0` to reset all slots.
    pub fn apply_slot(&mut self, id: u32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_lottie_animation_apply_slot(self.inner.raw(), id) })
    }

    /// Deletes a previously generated slot.
    pub fn del_slot(&mut self, id: u32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_lottie_animation_del_slot(self.inner.raw(), id) })
    }

    // ── Markers ────────────────────────────────────────────────────

    /// Gets the number of markers in the animation.
    pub fn markers_count(&self) -> Result<u32> {
        let mut cnt: u32 = 0;
        Error::from_raw(unsafe {
            sys::tvg_lottie_animation_get_markers_cnt(self.inner.raw(), &raw mut cnt)
        })?;
        Ok(cnt)
    }

    /// Gets the marker name at the given index.
    pub fn marker_name(&self, idx: u32) -> Result<String> {
        let mut name_ptr: *const core::ffi::c_char = core::ptr::null();
        Error::from_raw(unsafe {
            sys::tvg_lottie_animation_get_marker(self.inner.raw(), idx, &raw mut name_ptr)
        })?;
        if name_ptr.is_null() {
            return Ok(String::new());
        }
        Ok(unsafe { core::ffi::CStr::from_ptr(name_ptr) }
            .to_string_lossy()
            .into_owned())
    }

    /// Gets full marker information (name, begin frame, end frame) at the given index.
    pub fn marker_info(&self, idx: u32) -> Result<Marker> {
        let mut name_ptr: *const core::ffi::c_char = core::ptr::null();
        let mut begin: f32 = 0.0;
        let mut end: f32 = 0.0;
        Error::from_raw(unsafe {
            sys::tvg_lottie_animation_get_marker_info(
                self.inner.raw(),
                idx,
                &raw mut name_ptr,
                &raw mut begin,
                &raw mut end,
            )
        })?;
        let name = if name_ptr.is_null() {
            String::new()
        } else {
            unsafe { core::ffi::CStr::from_ptr(name_ptr) }
                .to_string_lossy()
                .into_owned()
        };
        Ok(Marker { name, begin, end })
    }

    /// Returns all markers in the animation.
    pub fn markers(&self) -> Result<Vec<Marker>> {
        let count = self.markers_count()?;
        let mut markers = Vec::with_capacity(count as usize);
        for i in 0..count {
            markers.push(self.marker_info(i)?);
        }
        Ok(markers)
    }

    /// Specifies a playback segment by marker name.
    pub fn set_marker(&mut self, marker: &str) -> Result<()> {
        let c_marker = CString::new(marker).map_err(|_| Error::InvalidArguments)?;
        Error::from_raw(unsafe {
            sys::tvg_lottie_animation_set_marker(self.inner.raw(), c_marker.as_ptr())
        })
    }

    // ── Tweening ───────────────────────────────────────────────────

    /// Interpolates between two frames based on a progress value (0.0–1.0).
    pub fn tween(&mut self, from: f32, to: f32, progress: f32) -> Result<()> {
        Error::from_raw(unsafe {
            sys::tvg_lottie_animation_tween(self.inner.raw(), from, to, progress)
        })
    }

    // ── Expressions ────────────────────────────────────────────────

    /// Updates the value of an expression variable for a specific layer.
    pub fn assign(&mut self, layer: &str, ix: u32, var: &str, val: f32) -> Result<()> {
        let c_layer = CString::new(layer).map_err(|_| Error::InvalidArguments)?;
        let c_var = CString::new(var).map_err(|_| Error::InvalidArguments)?;
        Error::from_raw(unsafe {
            sys::tvg_lottie_animation_assign(
                self.inner.raw(),
                c_layer.as_ptr(),
                ix,
                c_var.as_ptr(),
                val,
            )
        })
    }

    // ── Quality ────────────────────────────────────────────────────

    /// Sets the quality level for Lottie effects (0–100).
    ///
    /// Lower values prioritize performance, higher values prioritize quality.
    pub fn set_quality(&mut self, value: u8) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_lottie_animation_set_quality(self.inner.raw(), value) })
    }
}

impl<'eng> core::ops::Deref for LottieAnimation<'eng> {
    type Target = Animation<'eng>;

    fn deref(&self) -> &Animation<'eng> {
        &self.inner
    }
}

impl<'eng> core::ops::DerefMut for LottieAnimation<'eng> {
    fn deref_mut(&mut self) -> &mut Animation<'eng> {
        &mut self.inner
    }
}

impl core::fmt::Debug for LottieAnimation<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LottieAnimation").finish_non_exhaustive()
    }
}
