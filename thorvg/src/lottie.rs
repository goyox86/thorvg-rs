use alloc::ffi::CString;
use alloc::string::String;
use alloc::vec::Vec;

use crate::error::{Error, Result};
use crate::picture::Picture;
use thorvg_sys as ffi;

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
/// This extends the base [`Animation`](crate::Animation) functionality with
/// slots, markers, tweening, expression variables, and quality control.
///
/// # Example
///
/// ```no_run
/// use thorvg::{Thorvg, SwCanvas, ColorSpace, EngineOption, LottieAnimation, Paint};
///
/// let _engine = Thorvg::init(0).unwrap();
/// let mut canvas = SwCanvas::new(EngineOption::Default).unwrap();
/// let mut buffer = vec![0u32; 800 * 600];
/// canvas.set_target(&mut buffer, 800, 800, 600, ColorSpace::ABGR8888).unwrap();
///
/// let mut lottie = LottieAnimation::new();
/// let mut pic = lottie.picture();
/// pic.load_from_str("animation.json").ok();
/// // canvas.push(pic) would transfer the picture to the canvas
/// ```
pub struct LottieAnimation {
    raw: ffi::Tvg_Animation,
}

impl LottieAnimation {
    /// Creates a new Lottie animation object.
    pub fn new() -> Self {
        let raw = unsafe { ffi::tvg_lottie_animation_new() };
        assert!(!raw.is_null(), "failed to create LottieAnimation");
        Self { raw }
    }

    // ── Base Animation methods ─────────────────────────────────────

    /// Returns the associated Picture object.
    ///
    /// The returned Picture is **not owned** — it is managed by the animation.
    pub fn picture(&self) -> Picture {
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
        let mut d: f32 = 0.0;
        Error::from_raw(unsafe { ffi::tvg_animation_get_duration(self.raw, &raw mut d) })?;
        Ok(d)
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

    // ── Slots ──────────────────────────────────────────────────────

    /// Generates a new slot from the given Lottie slot data (JSON format).
    ///
    /// Returns the generated slot ID, or `None` on failure.
    pub fn gen_slot(&mut self, slot_json: &str) -> Option<u32> {
        let c_slot = CString::new(slot_json).ok()?;
        let id = unsafe { ffi::tvg_lottie_animation_gen_slot(self.raw, c_slot.as_ptr()) };
        if id == 0 { None } else { Some(id) }
    }

    /// Applies a previously generated slot to the animation.
    ///
    /// Pass `0` to reset all slots.
    pub fn apply_slot(&mut self, id: u32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_lottie_animation_apply_slot(self.raw, id) })
    }

    /// Deletes a previously generated slot.
    pub fn del_slot(&mut self, id: u32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_lottie_animation_del_slot(self.raw, id) })
    }

    // ── Markers ────────────────────────────────────────────────────

    /// Gets the number of markers in the animation.
    pub fn markers_count(&self) -> Result<u32> {
        let mut cnt: u32 = 0;
        Error::from_raw(unsafe {
            ffi::tvg_lottie_animation_get_markers_cnt(self.raw, &raw mut cnt)
        })?;
        Ok(cnt)
    }

    /// Gets the marker name at the given index.
    pub fn marker_name(&self, idx: u32) -> Result<String> {
        let mut name_ptr: *const core::ffi::c_char = core::ptr::null();
        Error::from_raw(unsafe {
            ffi::tvg_lottie_animation_get_marker(self.raw, idx, &raw mut name_ptr)
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
            ffi::tvg_lottie_animation_get_marker_info(
                self.raw,
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
            ffi::tvg_lottie_animation_set_marker(self.raw, c_marker.as_ptr())
        })
    }

    // ── Tweening ───────────────────────────────────────────────────

    /// Interpolates between two frames based on a progress value (0.0–1.0).
    pub fn tween(&mut self, from: f32, to: f32, progress: f32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_lottie_animation_tween(self.raw, from, to, progress) })
    }

    // ── Expressions ────────────────────────────────────────────────

    /// Updates the value of an expression variable for a specific layer.
    pub fn assign(&mut self, layer: &str, ix: u32, var: &str, val: f32) -> Result<()> {
        let c_layer = CString::new(layer).map_err(|_| Error::InvalidArguments)?;
        let c_var = CString::new(var).map_err(|_| Error::InvalidArguments)?;
        Error::from_raw(unsafe {
            ffi::tvg_lottie_animation_assign(self.raw, c_layer.as_ptr(), ix, c_var.as_ptr(), val)
        })
    }

    // ── Quality ────────────────────────────────────────────────────

    /// Sets the quality level for Lottie effects (0–100).
    ///
    /// Lower values prioritize performance, higher values prioritize quality.
    pub fn set_quality(&mut self, value: u8) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_lottie_animation_set_quality(self.raw, value) })
    }
}

impl Default for LottieAnimation {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LottieAnimation {
    fn drop(&mut self) {
        unsafe {
            ffi::tvg_animation_del(self.raw);
        }
    }
}
