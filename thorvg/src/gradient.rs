use alloc::vec::Vec;
use core::mem;

use crate::error::{Error, Result};
use crate::paint::{Matrix, PaintType};
use thorvg_sys as sys;

/// A color stop in a gradient.
#[derive(Debug, Clone, Copy)]
pub struct ColorStop {
    pub offset: f32,
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// How to fill the area outside the gradient bounds.
///
/// # Naming
///
/// This maps to thorvg's C enum `Tvg_Stroke_Fill`. The C name is
/// generic because thorvg reuses one enum for both stroke dash/fill
/// behavior and gradient spread; here it is only ever used for the
/// latter, so the Rust binding is named `FillSpread` to reflect the
/// gradient-spread role. The variants correspond one-to-one:
/// [`Pad`](Self::Pad) → `TVG_STROKE_FILL_PAD`,
/// [`Reflect`](Self::Reflect) → `TVG_STROKE_FILL_REFLECT`,
/// [`Repeat`](Self::Repeat) → `TVG_STROKE_FILL_REPEAT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FillSpread {
    Pad,
    Reflect,
    Repeat,
}

impl FillSpread {
    fn to_raw(self) -> sys::Tvg_Stroke_Fill {
        match self {
            FillSpread::Pad => sys::Tvg_Stroke_Fill::TVG_STROKE_FILL_PAD,
            FillSpread::Reflect => sys::Tvg_Stroke_Fill::TVG_STROKE_FILL_REFLECT,
            FillSpread::Repeat => sys::Tvg_Stroke_Fill::TVG_STROKE_FILL_REPEAT,
        }
    }

    fn from_raw(s: sys::Tvg_Stroke_Fill) -> Self {
        match s {
            sys::Tvg_Stroke_Fill::TVG_STROKE_FILL_PAD => FillSpread::Pad,
            sys::Tvg_Stroke_Fill::TVG_STROKE_FILL_REFLECT => FillSpread::Reflect,
            sys::Tvg_Stroke_Fill::TVG_STROKE_FILL_REPEAT => FillSpread::Repeat,
        }
    }
}

// ── LinearGradient ─────────────────────────────────────────────────

/// A linear gradient fill.
///
/// The lifetime `'eng` ties this gradient to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create gradients via [`Thorvg::linear_gradient()`](crate::Thorvg::linear_gradient).
pub struct LinearGradient<'eng> {
    raw: sys::Tvg_Gradient,
    _engine: core::marker::PhantomData<&'eng ()>,
}

impl LinearGradient<'_> {
    /// Creates a new linear gradient.
    pub(crate) fn new() -> Result<Self> {
        let raw = unsafe { sys::tvg_linear_gradient_new() };
        if raw.is_null() {
            return Err(Error::FailedAllocation);
        }
        Ok(Self {
            raw,
            _engine: core::marker::PhantomData,
        })
    }

    /// Sets the gradient bounds.
    pub fn set_bounds(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_linear_gradient_set(self.raw, x1, y1, x2, y2) })
    }

    /// Gets the gradient bounds.
    pub fn bounds(&self) -> Result<(f32, f32, f32, f32)> {
        let (mut x1, mut y1, mut x2, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        Error::from_raw(unsafe {
            sys::tvg_linear_gradient_get(
                self.raw,
                &raw mut x1,
                &raw mut y1,
                &raw mut x2,
                &raw mut y2,
            )
        })?;
        Ok((x1, y1, x2, y2))
    }

    /// Sets the color stops.
    pub fn set_color_stops(&mut self, stops: &[ColorStop]) -> Result<()> {
        set_color_stops_raw(self.raw, stops)
    }

    /// Gets the color stops.
    pub fn color_stops(&self) -> Result<Vec<ColorStop>> {
        get_color_stops_raw(self.raw)
    }

    /// Sets the fill spread method.
    pub fn set_spread(&mut self, spread: FillSpread) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_gradient_set_spread(self.raw, spread.to_raw()) })
    }

    /// Gets the fill spread method.
    pub fn spread(&self) -> Result<FillSpread> {
        get_spread_raw(self.raw)
    }

    /// Sets the affine transformation matrix.
    pub fn set_transform(&mut self, m: &Matrix) -> Result<()> {
        set_transform_raw(self.raw, m)
    }

    /// Gets the affine transformation matrix.
    pub fn transform(&self) -> Result<Matrix> {
        get_transform_raw(self.raw)
    }

    /// Gets the affine transformation matrix.
    #[deprecated(since = "0.2.0", note = "renamed to `transform` for consistency with `Paint::transform`")]
    pub fn get_transform(&self) -> Result<Matrix> {
        self.transform()
    }

    /// Gets the gradient type.
    pub fn gradient_type(&self) -> Result<PaintType> {
        get_type_raw(self.raw)
    }

    /// Duplicates this gradient.
    pub fn duplicate(&self) -> Option<Self> {
        let raw = unsafe { sys::tvg_gradient_duplicate(self.raw) };
        if raw.is_null() {
            None
        } else {
            Some(Self {
                raw,
                _engine: core::marker::PhantomData,
            })
        }
    }

    /// Consumes self and returns the raw pointer (ownership transferred).
    pub(crate) fn into_raw(self) -> sys::Tvg_Gradient {
        let raw = self.raw;
        mem::forget(self);
        raw
    }
}

impl Drop for LinearGradient<'_> {
    fn drop(&mut self) {
        unsafe {
            sys::tvg_gradient_del(self.raw);
        }
    }
}

// ── RadialGradient ─────────────────────────────────────────────────

/// A radial gradient fill.
pub struct RadialGradient<'eng> {
    raw: sys::Tvg_Gradient,
    _engine: core::marker::PhantomData<&'eng ()>,
}

impl RadialGradient<'_> {
    /// Creates a new radial gradient.
    pub(crate) fn new() -> Result<Self> {
        let raw = unsafe { sys::tvg_radial_gradient_new() };
        if raw.is_null() {
            return Err(Error::FailedAllocation);
        }
        Ok(Self {
            raw,
            _engine: core::marker::PhantomData,
        })
    }

    /// Sets the radial gradient attributes.
    pub fn set_radial(
        &mut self,
        cx: f32,
        cy: f32,
        r: f32,
        fx: f32,
        fy: f32,
        fr: f32,
    ) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_radial_gradient_set(self.raw, cx, cy, r, fx, fy, fr) })
    }

    /// Gets the radial gradient attributes: `(cx, cy, r, fx, fy, fr)`.
    pub fn radial(&self) -> Result<(f32, f32, f32, f32, f32, f32)> {
        let (mut cx, mut cy, mut r, mut fx, mut fy, mut fr) =
            (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
        Error::from_raw(unsafe {
            sys::tvg_radial_gradient_get(
                self.raw,
                &raw mut cx,
                &raw mut cy,
                &raw mut r,
                &raw mut fx,
                &raw mut fy,
                &raw mut fr,
            )
        })?;
        Ok((cx, cy, r, fx, fy, fr))
    }

    /// Sets the color stops.
    pub fn set_color_stops(&mut self, stops: &[ColorStop]) -> Result<()> {
        set_color_stops_raw(self.raw, stops)
    }

    /// Gets the color stops.
    pub fn color_stops(&self) -> Result<Vec<ColorStop>> {
        get_color_stops_raw(self.raw)
    }

    /// Sets the fill spread method.
    pub fn set_spread(&mut self, spread: FillSpread) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_gradient_set_spread(self.raw, spread.to_raw()) })
    }

    /// Gets the fill spread method.
    pub fn spread(&self) -> Result<FillSpread> {
        get_spread_raw(self.raw)
    }

    /// Sets the affine transformation matrix.
    pub fn set_transform(&mut self, m: &Matrix) -> Result<()> {
        set_transform_raw(self.raw, m)
    }

    /// Gets the affine transformation matrix.
    pub fn transform(&self) -> Result<Matrix> {
        get_transform_raw(self.raw)
    }

    /// Gets the affine transformation matrix.
    #[deprecated(since = "0.2.0", note = "renamed to `transform` for consistency with `Paint::transform`")]
    pub fn get_transform(&self) -> Result<Matrix> {
        self.transform()
    }

    /// Gets the gradient type.
    pub fn gradient_type(&self) -> Result<PaintType> {
        get_type_raw(self.raw)
    }

    /// Duplicates this gradient.
    pub fn duplicate(&self) -> Option<Self> {
        let raw = unsafe { sys::tvg_gradient_duplicate(self.raw) };
        if raw.is_null() {
            None
        } else {
            Some(Self {
                raw,
                _engine: core::marker::PhantomData,
            })
        }
    }

    /// Consumes self and returns the raw pointer (ownership transferred).
    pub(crate) fn into_raw(self) -> sys::Tvg_Gradient {
        let raw = self.raw;
        mem::forget(self);
        raw
    }
}

impl Drop for RadialGradient<'_> {
    fn drop(&mut self) {
        unsafe {
            sys::tvg_gradient_del(self.raw);
        }
    }
}

// ── Borrowed views ────────────────────────────────────────────
//
// Returned from [`Shape::gradient`] / [`Shape::stroke_gradient`].
// The shape owns the underlying `Tvg_Gradient`; these views
// borrow it for `'a`, exposing the read-only surface that does
// not invalidate the owner.  No `Drop`: the shape frees the
// gradient on its own teardown.
//
// Naming and lifetime structure mirror [`BorrowedPaint`] and
// [`BorrowedAccessor`].

/// Read-only view of a linear gradient owned by a [`Shape`](crate::Shape).
///
/// Acquired through [`Shape::gradient`](crate::Shape::gradient) /
/// [`Shape::stroke_gradient`](crate::Shape::stroke_gradient) after
/// matching on the [`BorrowedGradient::Linear`] variant.
pub struct BorrowedLinearGradient<'a> {
    raw: sys::Tvg_Gradient,
    _life: core::marker::PhantomData<&'a ()>,
}

impl BorrowedLinearGradient<'_> {
    /// # Safety
    /// `raw` must be a valid linear gradient handle whose owner
    /// outlives `'a`.
    pub(crate) unsafe fn from_raw(raw: sys::Tvg_Gradient) -> Self {
        Self {
            raw,
            _life: core::marker::PhantomData,
        }
    }

    /// Returns the gradient bounds `(x1, y1, x2, y2)`.
    pub fn bounds(&self) -> Result<(f32, f32, f32, f32)> {
        let (mut x1, mut y1, mut x2, mut y2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
        Error::from_raw(unsafe {
            sys::tvg_linear_gradient_get(
                self.raw,
                &raw mut x1,
                &raw mut y1,
                &raw mut x2,
                &raw mut y2,
            )
        })?;
        Ok((x1, y1, x2, y2))
    }

    /// Returns the color stops.
    pub fn color_stops(&self) -> Result<Vec<ColorStop>> {
        get_color_stops_raw(self.raw)
    }

    /// Returns the fill spread method.
    pub fn spread(&self) -> Result<FillSpread> {
        get_spread_raw(self.raw)
    }

    /// Returns the affine transformation matrix.
    pub fn transform(&self) -> Result<Matrix> {
        get_transform_raw(self.raw)
    }
}

impl core::fmt::Debug for BorrowedLinearGradient<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BorrowedLinearGradient").finish_non_exhaustive()
    }
}

/// Read-only view of a radial gradient owned by a [`Shape`](crate::Shape).
///
/// Acquired through [`Shape::gradient`](crate::Shape::gradient) /
/// [`Shape::stroke_gradient`](crate::Shape::stroke_gradient) after
/// matching on the [`BorrowedGradient::Radial`] variant.
pub struct BorrowedRadialGradient<'a> {
    raw: sys::Tvg_Gradient,
    _life: core::marker::PhantomData<&'a ()>,
}

impl BorrowedRadialGradient<'_> {
    /// # Safety
    /// `raw` must be a valid radial gradient handle whose owner
    /// outlives `'a`.
    pub(crate) unsafe fn from_raw(raw: sys::Tvg_Gradient) -> Self {
        Self {
            raw,
            _life: core::marker::PhantomData,
        }
    }

    /// Returns the radial gradient attributes `(cx, cy, r, fx, fy, fr)`.
    pub fn radial(&self) -> Result<(f32, f32, f32, f32, f32, f32)> {
        let (mut cx, mut cy, mut r, mut fx, mut fy, mut fr) =
            (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
        Error::from_raw(unsafe {
            sys::tvg_radial_gradient_get(
                self.raw,
                &raw mut cx,
                &raw mut cy,
                &raw mut r,
                &raw mut fx,
                &raw mut fy,
                &raw mut fr,
            )
        })?;
        Ok((cx, cy, r, fx, fy, fr))
    }

    /// Returns the color stops.
    pub fn color_stops(&self) -> Result<Vec<ColorStop>> {
        get_color_stops_raw(self.raw)
    }

    /// Returns the fill spread method.
    pub fn spread(&self) -> Result<FillSpread> {
        get_spread_raw(self.raw)
    }

    /// Returns the affine transformation matrix.
    pub fn transform(&self) -> Result<Matrix> {
        get_transform_raw(self.raw)
    }
}

impl core::fmt::Debug for BorrowedRadialGradient<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BorrowedRadialGradient").finish_non_exhaustive()
    }
}

/// Discriminated read-only view of a gradient owned by a
/// [`Shape`](crate::Shape).
///
/// The variant is determined at borrow time via
/// `tvg_gradient_get_type`; subsequent reads through the matching
/// `BorrowedLinearGradient` / `BorrowedRadialGradient` are then
/// type-checked at compile time (no "linear method called on a
/// radial gradient" runtime errors).
///
/// Exhaustive: the C "gradient kind" set has only two members
/// (`TVG_TYPE_LINEAR_GRAD`, `TVG_TYPE_RADIAL_GRAD`) and is closed
/// in the same sense as [`BlurDirection`](crate::BlurDirection).
#[derive(Debug)]
pub enum BorrowedGradient<'a> {
    /// Linear gradient view.
    Linear(BorrowedLinearGradient<'a>),
    /// Radial gradient view.
    Radial(BorrowedRadialGradient<'a>),
}

impl BorrowedGradient<'_> {
    /// Discriminates the kind of gradient behind `raw` and builds
    /// the matching borrowed view.
    ///
    /// # Safety
    /// `raw` must be a valid gradient handle whose owner outlives `'a`.
    pub(crate) unsafe fn from_raw(raw: sys::Tvg_Gradient) -> Result<Self> {
        let kind = get_type_raw(raw)?;
        Ok(match kind {
            PaintType::LinearGradient => {
                Self::Linear(unsafe { BorrowedLinearGradient::from_raw(raw) })
            }
            PaintType::RadialGradient => {
                Self::Radial(unsafe { BorrowedRadialGradient::from_raw(raw) })
            }
            // `tvg_gradient_get_type` only ever reports linear or
            // radial on a valid gradient handle; any other answer
            // means the C engine returned junk and we cannot trust
            // further reads.
            _ => return Err(Error::Unknown),
        })
    }

    /// Returns the color stops (variant-independent).
    pub fn color_stops(&self) -> Result<Vec<ColorStop>> {
        match self {
            Self::Linear(g) => g.color_stops(),
            Self::Radial(g) => g.color_stops(),
        }
    }

    /// Returns the fill spread method (variant-independent).
    pub fn spread(&self) -> Result<FillSpread> {
        match self {
            Self::Linear(g) => g.spread(),
            Self::Radial(g) => g.spread(),
        }
    }

    /// Returns the affine transformation matrix (variant-independent).
    pub fn transform(&self) -> Result<Matrix> {
        match self {
            Self::Linear(g) => g.transform(),
            Self::Radial(g) => g.transform(),
        }
    }
}

// ── Shared helpers ─────────────────────────────────────────────────

#[allow(clippy::cast_possible_truncation)]
fn set_color_stops_raw(raw: sys::Tvg_Gradient, stops: &[ColorStop]) -> Result<()> {
    let raw_stops: Vec<sys::Tvg_Color_Stop> = stops
        .iter()
        .map(|s| sys::Tvg_Color_Stop {
            offset: s.offset,
            r: s.r,
            g: s.g,
            b: s.b,
            a: s.a,
        })
        .collect();
    Error::from_raw(unsafe {
        sys::tvg_gradient_set_color_stops(raw, raw_stops.as_ptr(), raw_stops.len() as u32)
    })
}

fn get_color_stops_raw(raw: sys::Tvg_Gradient) -> Result<Vec<ColorStop>> {
    let mut ptr: *const sys::Tvg_Color_Stop = core::ptr::null();
    let mut cnt: u32 = 0;
    Error::from_raw(unsafe { sys::tvg_gradient_get_color_stops(raw, &raw mut ptr, &raw mut cnt) })?;
    if ptr.is_null() || cnt == 0 {
        return Ok(Vec::new());
    }
    let slice = unsafe { core::slice::from_raw_parts(ptr, cnt as usize) };
    Ok(slice
        .iter()
        .map(|s| ColorStop {
            offset: s.offset,
            r: s.r,
            g: s.g,
            b: s.b,
            a: s.a,
        })
        .collect())
}

fn get_spread_raw(raw: sys::Tvg_Gradient) -> Result<FillSpread> {
    let mut spread = sys::Tvg_Stroke_Fill::TVG_STROKE_FILL_PAD;
    Error::from_raw(unsafe { sys::tvg_gradient_get_spread(raw, &raw mut spread) })?;
    Ok(FillSpread::from_raw(spread))
}

fn set_transform_raw(raw: sys::Tvg_Gradient, m: &Matrix) -> Result<()> {
    let rm = sys::Tvg_Matrix {
        e11: m.e11,
        e12: m.e12,
        e13: m.e13,
        e21: m.e21,
        e22: m.e22,
        e23: m.e23,
        e31: m.e31,
        e32: m.e32,
        e33: m.e33,
    };
    Error::from_raw(unsafe { sys::tvg_gradient_set_transform(raw, &raw const rm) })
}

fn get_type_raw(raw: sys::Tvg_Gradient) -> Result<PaintType> {
    let mut t = sys::Tvg_Type::TVG_TYPE_UNDEF;
    Error::from_raw(unsafe { sys::tvg_gradient_get_type(raw, &raw mut t) })?;
    Ok(PaintType::from_raw(t))
}

fn get_transform_raw(raw: sys::Tvg_Gradient) -> Result<Matrix> {
    let mut m = sys::Tvg_Matrix {
        e11: 0.0,
        e12: 0.0,
        e13: 0.0,
        e21: 0.0,
        e22: 0.0,
        e23: 0.0,
        e31: 0.0,
        e32: 0.0,
        e33: 0.0,
    };
    Error::from_raw(unsafe { sys::tvg_gradient_get_transform(raw, &raw mut m) })?;
    Ok(Matrix {
        e11: m.e11,
        e12: m.e12,
        e13: m.e13,
        e21: m.e21,
        e22: m.e22,
        e23: m.e23,
        e31: m.e31,
        e32: m.e32,
        e33: m.e33,
    })
}
