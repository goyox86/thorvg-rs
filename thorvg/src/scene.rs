use crate::error::{Error, Result};
use crate::paint::Paint;
use thorvg_sys as ffi;

/// A scene that groups multiple paint objects.
pub struct Scene {
    raw: ffi::Tvg_Paint,
    owned: bool,
}

impl Scene {
    /// Creates a new Scene object.
    pub fn new() -> Self {
        let raw = unsafe { ffi::tvg_scene_new() };
        assert!(!raw.is_null(), "failed to create Scene");
        Self { raw, owned: true }
    }

    /// Adds a paint object to the scene (appended at the end).
    pub fn push<P: Paint>(&mut self, paint: P) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_scene_add(self.raw, paint.into_raw()) })
    }

    /// Inserts a paint object before another existing paint in the scene.
    pub fn insert<P: Paint, Q: Paint>(&mut self, target: P, at: &Q) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_scene_insert(self.raw, target.into_raw(), at.raw()) })
    }

    /// Removes a paint from the scene.
    pub fn remove<P: Paint>(&mut self, paint: &P) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_scene_remove(self.raw, paint.raw()) })
    }

    /// Removes all paints from the scene.
    pub fn clear(&mut self) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_scene_remove(self.raw, core::ptr::null_mut()) })
    }

    /// Clears all scene effects.
    pub fn clear_effects(&mut self) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_scene_clear_effects(self.raw) })
    }

    /// Adds a Gaussian blur effect.
    pub fn add_gaussian_blur(
        &mut self,
        sigma: f64,
        direction: i32,
        border: i32,
        quality: i32,
    ) -> Result<()> {
        Error::from_raw(unsafe {
            ffi::tvg_scene_add_effect_gaussian_blur(self.raw, sigma, direction, border, quality)
        })
    }

    /// Adds a drop shadow effect.
    #[allow(clippy::too_many_arguments)]
    pub fn add_drop_shadow(
        &mut self,
        r: i32,
        g: i32,
        b: i32,
        a: i32,
        angle: f64,
        distance: f64,
        sigma: f64,
        quality: i32,
    ) -> Result<()> {
        Error::from_raw(unsafe {
            ffi::tvg_scene_add_effect_drop_shadow(
                self.raw, r, g, b, a, angle, distance, sigma, quality,
            )
        })
    }

    /// Adds a fill color effect (overrides scene content color).
    pub fn add_fill_effect(&mut self, r: i32, g: i32, b: i32, a: i32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_scene_add_effect_fill(self.raw, r, g, b, a) })
    }

    /// Adds a tint effect.
    #[allow(clippy::too_many_arguments)]
    pub fn add_tint_effect(
        &mut self,
        black_r: i32,
        black_g: i32,
        black_b: i32,
        white_r: i32,
        white_g: i32,
        white_b: i32,
        intensity: f64,
    ) -> Result<()> {
        Error::from_raw(unsafe {
            ffi::tvg_scene_add_effect_tint(
                self.raw, black_r, black_g, black_b, white_r, white_g, white_b, intensity,
            )
        })
    }

    /// Adds a tritone color effect.
    #[allow(clippy::too_many_arguments)]
    pub fn add_tritone_effect(
        &mut self,
        shadow_r: i32,
        shadow_g: i32,
        shadow_b: i32,
        midtone_r: i32,
        midtone_g: i32,
        midtone_b: i32,
        highlight_r: i32,
        highlight_g: i32,
        highlight_b: i32,
        blend: i32,
    ) -> Result<()> {
        Error::from_raw(unsafe {
            ffi::tvg_scene_add_effect_tritone(
                self.raw,
                shadow_r,
                shadow_g,
                shadow_b,
                midtone_r,
                midtone_g,
                midtone_b,
                highlight_r,
                highlight_g,
                highlight_b,
                blend,
            )
        })
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Paint for Scene {
    fn raw(&self) -> ffi::Tvg_Paint {
        self.raw
    }

    fn into_raw(mut self) -> ffi::Tvg_Paint {
        self.owned = false;
        self.raw
    }

    unsafe fn from_raw_paint(raw: ffi::Tvg_Paint) -> Self {
        Self { raw, owned: true }
    }
}

impl Drop for Scene {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                ffi::tvg_paint_rel(self.raw);
            }
        }
    }
}
