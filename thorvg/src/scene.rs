use crate::color::{Rgb, Rgba};
use crate::error::{Error, Result};
use crate::paint::Paint;
use thorvg_sys as sys;

/// Axis along which a [`Scene::add_gaussian_blur_effect`] blur is applied.
///
/// Maps to the raw `int direction` parameter of the underlying C call
/// `tvg_scene_add_effect_gaussian_blur`, whose documented values are:
///
/// | C value | Variant           |
/// |---------|-------------------|
/// | `0`     | [`Both`](Self::Both)             |
/// | `1`     | [`Horizontal`](Self::Horizontal) |
/// | `2`     | [`Vertical`](Self::Vertical)     |
///
/// thorvg's C API takes a bare `int` here — there is no `Tvg_*`
/// typedef — so the wrapper carries the encoding rather than
/// re-exporting a C enum.
///
/// Exhaustive: the C header documents the full set
/// (`tvg_scene_add_effect_gaussian_blur`'s `direction` parameter) and
/// has not grown since the function was introduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum BlurDirection {
    /// Blur on both axes (the default in upstream).
    Both = 0,
    /// Blur along the horizontal axis only.
    Horizontal = 1,
    /// Blur along the vertical axis only.
    Vertical = 2,
}

impl BlurDirection {
    fn to_raw(self) -> core::ffi::c_int {
        self as core::ffi::c_int
    }
}

/// Edge-sampling behaviour for [`Scene::add_gaussian_blur_effect`].
///
/// Maps to the raw `int border` parameter of the underlying C call
/// `tvg_scene_add_effect_gaussian_blur`:
///
/// | C value | Variant                          |
/// |---------|----------------------------------|
/// | `0`     | [`Duplicate`](Self::Duplicate)   |
/// | `1`     | [`Wrap`](Self::Wrap)             |
///
/// Exhaustive: the C header documents both values and has not grown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum BlurBorder {
    /// Replicate the edge pixel when the kernel reaches outside the
    /// scene bounds.
    Duplicate = 0,
    /// Wrap the sampling window around to the opposite edge.
    Wrap = 1,
}

impl BlurBorder {
    fn to_raw(self) -> core::ffi::c_int {
        self as core::ffi::c_int
    }
}

/// Parameters for [`Scene::add_drop_shadow_effect`].
///
/// Mirrors the layout of
/// `tvg_scene_add_effect_drop_shadow(scene, r, g, b, a, angle, distance, sigma, quality)`,
/// grouping the four RGBA ints into a single [`Rgba`] so the call
/// site no longer needs eight positional arguments.
///
/// Three construction styles are supported:
///
/// ```ignore
/// // 1. Struct literal (all fields explicit):
/// DropShadow {
///     color: Rgba::new(0, 0, 0, 150),
///     angle: 135.0,
///     distance: 8.0,
///     sigma: 4.0,
///     quality: 80,
/// }
///
/// // 2. Default + field override:
/// DropShadow { angle: 135.0, ..Default::default() }
///
/// // 3. Builder:
/// DropShadow::new().angle(135.0).distance(8.0)
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DropShadow {
    /// Shadow color (RGBA, 0..=255 per channel).
    pub color: Rgba,
    /// Shadow direction in degrees, `[0, 360]`.
    pub angle: f64,
    /// Distance of the shadow from the original object.
    pub distance: f64,
    /// Gaussian blur sigma for the shadow.  Must be `> 0`.
    pub sigma: f64,
    /// Blur quality level, `[0, 100]`.
    pub quality: u8,
}

impl DropShadow {
    /// Returns a shadow with sensible defaults that the engine
    /// accepts (opaque black, angled downward, modest blur).
    ///
    /// | Field      | Value                  |
    /// |------------|------------------------|
    /// | `color`    | `Rgba::new(0, 0, 0, 255)` (opaque black) |
    /// | `angle`    | `0.0` (downward)       |
    /// | `distance` | `4.0`                  |
    /// | `sigma`    | `2.0`                  |
    /// | `quality`  | `50`                   |
    ///
    /// All defaults are non-zero so the effect actually renders
    /// (the engine rejects `sigma <= 0`).
    #[must_use]
    pub const fn new() -> Self {
        Self {
            color: Rgba::new(0, 0, 0, 255),
            angle: 0.0,
            distance: 4.0,
            sigma: 2.0,
            quality: 50,
        }
    }

    /// Sets the shadow color.
    #[must_use]
    pub const fn color(mut self, color: Rgba) -> Self {
        self.color = color;
        self
    }

    /// Sets the shadow direction in degrees, `[0, 360]`.
    #[must_use]
    pub const fn angle(mut self, angle: f64) -> Self {
        self.angle = angle;
        self
    }

    /// Sets the distance of the shadow from the source object.
    #[must_use]
    pub const fn distance(mut self, distance: f64) -> Self {
        self.distance = distance;
        self
    }

    /// Sets the Gaussian blur sigma (must be `> 0`).
    #[must_use]
    pub const fn sigma(mut self, sigma: f64) -> Self {
        self.sigma = sigma;
        self
    }

    /// Sets the blur quality level, `[0, 100]`.
    #[must_use]
    pub const fn quality(mut self, quality: u8) -> Self {
        self.quality = quality;
        self
    }
}

impl Default for DropShadow {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameters for [`Scene::add_tritone_effect`].
///
/// Mirrors the layout of
/// `tvg_scene_add_effect_tritone(scene, shadow_r, shadow_g, shadow_b, midtone_r, midtone_g, midtone_b, highlight_r, highlight_g, highlight_b, blend)`,
/// grouping the three RGB triplets into named [`Rgb`] fields.
///
/// Same three construction styles as [`DropShadow`] are supported
/// (struct literal, `..Default::default()`, builder).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tritone {
    /// Shadow color.
    pub shadow: Rgb,
    /// Midtone color.
    pub midtone: Rgb,
    /// Highlight color.
    pub highlight: Rgb,
    /// Blend factor between the original color and the tritone
    /// palette, `[0, 255]`.
    pub blend: u8,
}

impl Tritone {
    /// Returns a neutral tritone palette:
    ///
    /// | Field       | Value                            |
    /// |-------------|----------------------------------|
    /// | `shadow`    | `Rgb::new(0, 0, 0)` (black)      |
    /// | `midtone`   | `Rgb::new(128, 128, 128)` (gray) |
    /// | `highlight` | `Rgb::new(255, 255, 255)` (white)|
    /// | `blend`     | `128`                            |
    #[must_use]
    pub const fn new() -> Self {
        Self {
            shadow: Rgb::new(0, 0, 0),
            midtone: Rgb::new(128, 128, 128),
            highlight: Rgb::new(255, 255, 255),
            blend: 128,
        }
    }

    /// Sets the shadow tone.
    #[must_use]
    pub const fn shadow(mut self, shadow: Rgb) -> Self {
        self.shadow = shadow;
        self
    }

    /// Sets the midtone.
    #[must_use]
    pub const fn midtone(mut self, midtone: Rgb) -> Self {
        self.midtone = midtone;
        self
    }

    /// Sets the highlight tone.
    #[must_use]
    pub const fn highlight(mut self, highlight: Rgb) -> Self {
        self.highlight = highlight;
        self
    }

    /// Sets the blend factor between the original color and the
    /// tritone palette, `[0, 255]`.
    #[must_use]
    pub const fn blend(mut self, blend: u8) -> Self {
        self.blend = blend;
        self
    }
}

impl Default for Tritone {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameters for [`Scene::add_tint_effect`].
///
/// Mirrors the layout of
/// `tvg_scene_add_effect_tint(scene, black_r, black_g, black_b, white_r, white_g, white_b, intensity)`,
/// grouping the two RGB endpoints into named [`Rgb`] fields.
///
/// Same three construction styles as [`DropShadow`] are supported
/// (struct literal, `..Default::default()`, builder).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tint {
    /// Color the darkest scene pixels map to.
    pub black: Rgb,
    /// Color the brightest scene pixels map to.
    pub white: Rgb,
    /// Tint intensity, `[0, 100]`.  `0` leaves the original
    /// colors untouched; `100` is full tint.
    pub intensity: f64,
}

impl Tint {
    /// Returns a neutral grayscale tint:
    ///
    /// | Field       | Value                            |
    /// |-------------|----------------------------------|
    /// | `black`     | `Rgb::new(0, 0, 0)` (black)      |
    /// | `white`     | `Rgb::new(255, 255, 255)` (white)|
    /// | `intensity` | `50.0`                           |
    #[must_use]
    pub const fn new() -> Self {
        Self {
            black: Rgb::new(0, 0, 0),
            white: Rgb::new(255, 255, 255),
            intensity: 50.0,
        }
    }

    /// Sets the color the darkest scene pixels map to.
    #[must_use]
    pub const fn black(mut self, black: Rgb) -> Self {
        self.black = black;
        self
    }

    /// Sets the color the brightest scene pixels map to.
    #[must_use]
    pub const fn white(mut self, white: Rgb) -> Self {
        self.white = white;
        self
    }

    /// Sets the tint intensity, `[0, 100]`.
    #[must_use]
    pub const fn intensity(mut self, intensity: f64) -> Self {
        self.intensity = intensity;
        self
    }
}

impl Default for Tint {
    fn default() -> Self {
        Self::new()
    }
}

/// A scene that groups multiple paint objects.
///
/// The lifetime `'eng` ties this scene to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create scenes via [`Thorvg::scene()`](crate::Thorvg::scene).
pub struct Scene<'eng> {
    raw: sys::Tvg_Paint,
    owned: bool,
    _engine: core::marker::PhantomData<&'eng ()>,
}

// SAFETY: Same rationale as other ThorVG handle types — exclusive
// ownership of a C heap object; global state is mutex-protected.
unsafe impl Send for Scene<'_> {}

impl Scene<'_> {
    /// Creates a new Scene object.
    pub(crate) fn new() -> Result<Self> {
        let raw = unsafe { sys::tvg_scene_new() };
        if raw.is_null() {
            return Err(Error::FailedAllocation);
        }
        Ok(Self {
            raw,
            owned: true,
            _engine: core::marker::PhantomData,
        })
    }

    /// Adds a paint object to the scene (appended at the end).
    pub fn add<P: Paint>(&mut self, paint: P) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_scene_add(self.raw, paint.into_raw()) })
    }

    /// Inserts a paint object before another existing paint in the scene.
    pub fn insert<P: Paint, Q: Paint>(&mut self, target: P, at: &Q) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_scene_insert(self.raw, target.into_raw(), at.raw()) })
    }

    /// Removes a paint from the scene.
    pub fn remove<P: Paint>(&mut self, paint: &P) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_scene_remove(self.raw, paint.raw()) })
    }

    /// Removes all paints from the scene.
    pub fn clear(&mut self) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_scene_remove(self.raw, core::ptr::null_mut()) })
    }

    /// Clears all scene effects.
    pub fn clear_effects(&mut self) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_scene_clear_effects(self.raw) })
    }

    /// Adds a Gaussian blur effect.
    ///
    /// `sigma` is the blur radius (must be `> 0`); `direction`
    /// selects which axis (or both) the kernel sweeps; `border`
    /// controls how samples outside the scene bounds are handled;
    /// `quality` is in `[0, 100]` (clamped by the engine).
    pub fn add_gaussian_blur_effect(
        &mut self,
        sigma: f64,
        direction: BlurDirection,
        border: BlurBorder,
        quality: i32,
    ) -> Result<()> {
        Error::from_raw(unsafe {
            sys::tvg_scene_add_effect_gaussian_blur(
                self.raw,
                sigma,
                direction.to_raw(),
                border.to_raw(),
                quality,
            )
        })
    }

    /// Adds a drop shadow effect.
    ///
    /// See [`DropShadow`] for the parameter layout.
    pub fn add_drop_shadow_effect(&mut self, params: DropShadow) -> Result<()> {
        let DropShadow {
            color: Rgba { r, g, b, a },
            angle,
            distance,
            sigma,
            quality,
        } = params;
        Error::from_raw(unsafe {
            sys::tvg_scene_add_effect_drop_shadow(
                self.raw,
                i32::from(r),
                i32::from(g),
                i32::from(b),
                i32::from(a),
                angle,
                distance,
                sigma,
                i32::from(quality),
            )
        })
    }

    /// Adds a fill color effect (overrides scene content color).
    pub fn add_fill_effect(&mut self, color: Rgba) -> Result<()> {
        let Rgba { r, g, b, a } = color;
        Error::from_raw(unsafe {
            sys::tvg_scene_add_effect_fill(
                self.raw,
                i32::from(r),
                i32::from(g),
                i32::from(b),
                i32::from(a),
            )
        })
    }

    /// Adds a tint effect.
    ///
    /// See [`Tint`] for the parameter layout.
    pub fn add_tint_effect(&mut self, params: Tint) -> Result<()> {
        let Tint {
            black,
            white,
            intensity,
        } = params;
        Error::from_raw(unsafe {
            sys::tvg_scene_add_effect_tint(
                self.raw,
                i32::from(black.r),
                i32::from(black.g),
                i32::from(black.b),
                i32::from(white.r),
                i32::from(white.g),
                i32::from(white.b),
                intensity,
            )
        })
    }

    /// Adds a tritone color effect.
    ///
    /// See [`Tritone`] for the parameter layout.
    pub fn add_tritone_effect(&mut self, params: Tritone) -> Result<()> {
        let Tritone {
            shadow,
            midtone,
            highlight,
            blend,
        } = params;
        Error::from_raw(unsafe {
            sys::tvg_scene_add_effect_tritone(
                self.raw,
                i32::from(shadow.r),
                i32::from(shadow.g),
                i32::from(shadow.b),
                i32::from(midtone.r),
                i32::from(midtone.g),
                i32::from(midtone.b),
                i32::from(highlight.r),
                i32::from(highlight.g),
                i32::from(highlight.b),
                i32::from(blend),
            )
        })
    }
}

impl crate::paint::sealed::Sealed for Scene<'_> {}

impl Paint for Scene<'_> {
    fn raw(&self) -> sys::Tvg_Paint {
        self.raw
    }

    fn into_raw(mut self) -> sys::Tvg_Paint {
        self.owned = false;
        self.raw
    }

    unsafe fn from_raw_paint(raw: sys::Tvg_Paint) -> Self {
        Self {
            raw,
            owned: true,
            _engine: core::marker::PhantomData,
        }
    }
}

impl Drop for Scene<'_> {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                sys::tvg_paint_rel(self.raw);
            }
        }
    }
}

impl core::fmt::Debug for Scene<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Scene").finish_non_exhaustive()
    }
}
