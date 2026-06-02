use crate::error::{Error, Result};
use crate::shape::Shape;
use thorvg_sys as sys;

/// A 3×3 affine transformation matrix.
///
/// Note: `PartialEq` uses exact floating-point comparison.
/// Use approximate comparison for transformed values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    pub e11: f32,
    pub e12: f32,
    pub e13: f32,
    pub e21: f32,
    pub e22: f32,
    pub e23: f32,
    pub e31: f32,
    pub e32: f32,
    pub e33: f32,
}

impl Matrix {
    /// The identity matrix.
    pub const IDENTITY: Self = Self {
        e11: 1.0,
        e12: 0.0,
        e13: 0.0,
        e21: 0.0,
        e22: 1.0,
        e23: 0.0,
        e31: 0.0,
        e32: 0.0,
        e33: 1.0,
    };

    fn to_raw(self) -> sys::Tvg_Matrix {
        sys::Tvg_Matrix {
            e11: self.e11,
            e12: self.e12,
            e13: self.e13,
            e21: self.e21,
            e22: self.e22,
            e23: self.e23,
            e31: self.e31,
            e32: self.e32,
            e33: self.e33,
        }
    }

    fn from_raw(m: sys::Tvg_Matrix) -> Self {
        Self {
            e11: m.e11,
            e12: m.e12,
            e13: m.e13,
            e21: m.e21,
            e22: m.e22,
            e23: m.e23,
            e31: m.e31,
            e32: m.e32,
            e33: m.e33,
        }
    }
}

/// A point in 2D space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

/// Blending method for compositing paint objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum BlendMethod {
    Normal = 0,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
    Add,
}

impl BlendMethod {
    fn to_raw(self) -> sys::Tvg_Blend_Method {
        match self {
            Self::Normal => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_NORMAL,
            Self::Multiply => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_MULTIPLY,
            Self::Screen => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_SCREEN,
            Self::Overlay => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_OVERLAY,
            Self::Darken => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_DARKEN,
            Self::Lighten => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_LIGHTEN,
            Self::ColorDodge => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_COLORDODGE,
            Self::ColorBurn => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_COLORBURN,
            Self::HardLight => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_HARDLIGHT,
            Self::SoftLight => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_SOFTLIGHT,
            Self::Difference => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_DIFFERENCE,
            Self::Exclusion => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_EXCLUSION,
            Self::Hue => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_HUE,
            Self::Saturation => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_SATURATION,
            Self::Color => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_COLOR,
            Self::Luminosity => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_LUMINOSITY,
            Self::Add => sys::Tvg_Blend_Method::TVG_BLEND_METHOD_ADD,
        }
    }
}

/// Masking method for combining two paint objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum MaskMethod {
    None = 0,
    Alpha,
    InvAlpha,
    Luma,
    InvLuma,
    Add,
    Subtract,
    Intersect,
    Difference,
    Lighten,
    Darken,
}

impl MaskMethod {
    fn to_raw(self) -> sys::Tvg_Mask_Method {
        match self {
            Self::None => sys::Tvg_Mask_Method::TVG_MASK_METHOD_NONE,
            Self::Alpha => sys::Tvg_Mask_Method::TVG_MASK_METHOD_ALPHA,
            Self::InvAlpha => sys::Tvg_Mask_Method::TVG_MASK_METHOD_INVERSE_ALPHA,
            Self::Luma => sys::Tvg_Mask_Method::TVG_MASK_METHOD_LUMA,
            Self::InvLuma => sys::Tvg_Mask_Method::TVG_MASK_METHOD_INVERSE_LUMA,
            Self::Add => sys::Tvg_Mask_Method::TVG_MASK_METHOD_ADD,
            Self::Subtract => sys::Tvg_Mask_Method::TVG_MASK_METHOD_SUBTRACT,
            Self::Intersect => sys::Tvg_Mask_Method::TVG_MASK_METHOD_INTERSECT,
            Self::Difference => sys::Tvg_Mask_Method::TVG_MASK_METHOD_DIFFERENCE,
            Self::Lighten => sys::Tvg_Mask_Method::TVG_MASK_METHOD_LIGHTEN,
            Self::Darken => sys::Tvg_Mask_Method::TVG_MASK_METHOD_DARKEN,
        }
    }

    pub(crate) fn from_raw(m: sys::Tvg_Mask_Method) -> Self {
        match m {
            sys::Tvg_Mask_Method::TVG_MASK_METHOD_ALPHA => Self::Alpha,
            sys::Tvg_Mask_Method::TVG_MASK_METHOD_INVERSE_ALPHA => Self::InvAlpha,
            sys::Tvg_Mask_Method::TVG_MASK_METHOD_LUMA => Self::Luma,
            sys::Tvg_Mask_Method::TVG_MASK_METHOD_INVERSE_LUMA => Self::InvLuma,
            sys::Tvg_Mask_Method::TVG_MASK_METHOD_ADD => Self::Add,
            sys::Tvg_Mask_Method::TVG_MASK_METHOD_SUBTRACT => Self::Subtract,
            sys::Tvg_Mask_Method::TVG_MASK_METHOD_INTERSECT => Self::Intersect,
            sys::Tvg_Mask_Method::TVG_MASK_METHOD_DIFFERENCE => Self::Difference,
            sys::Tvg_Mask_Method::TVG_MASK_METHOD_LIGHTEN => Self::Lighten,
            sys::Tvg_Mask_Method::TVG_MASK_METHOD_DARKEN => Self::Darken,
            _ => Self::None,
        }
    }
}

/// A read-only borrow of a paint owned by another object.
///
/// Returned by getters that expose an internal paint handle whose
/// lifetime is governed by the containing object (e.g. the mask
/// target stored inside a `Paint::mask` relationship).  The `'a`
/// lifetime ties the borrow to the source: dropping the source
/// invalidates the borrow.
///
/// Read-only surface only — mutating the borrowed paint via raw
/// access would race with the owner.
pub struct BorrowedPaint<'a> {
    raw: sys::Tvg_Paint,
    _life: core::marker::PhantomData<&'a ()>,
}

impl<'a> BorrowedPaint<'a> {
    /// Returns the underlying raw handle.  Intended for use with
    /// `thorvg-sys` C APIs not yet wrapped here; lifetime `'a` keeps
    /// the borrow honest.
    pub fn raw(&self) -> sys::Tvg_Paint {
        self.raw
    }

    /// The runtime paint type of the borrowed handle.
    pub fn paint_type(&self) -> Result<PaintType> {
        let mut t = sys::Tvg_Type::TVG_TYPE_UNDEF;
        Error::from_raw(unsafe { sys::tvg_paint_get_type(self.raw, &raw mut t) })?;
        Ok(PaintType::from_raw(t))
    }

    /// The user-assigned id, or 0 if none was set.
    pub fn id(&self) -> u32 {
        unsafe { sys::tvg_paint_get_id(self.raw) }
    }

    /// The opacity (0..=255).
    pub fn opacity(&self) -> Result<u8> {
        let mut o: u8 = 0;
        Error::from_raw(unsafe { sys::tvg_paint_get_opacity(self.raw, &raw mut o) })?;
        Ok(o)
    }

    /// The axis-aligned bounding box `(x, y, w, h)` in canvas space.
    pub fn bounds(&self) -> Result<(f32, f32, f32, f32)> {
        let (mut x, mut y, mut w, mut h) = (0f32, 0f32, 0f32, 0f32);
        Error::from_raw(unsafe {
            sys::tvg_paint_get_aabb(
                self.raw,
                &raw mut x,
                &raw mut y,
                &raw mut w,
                &raw mut h,
            )
        })?;
        Ok((x, y, w, h))
    }
}

impl core::fmt::Debug for BorrowedPaint<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BorrowedPaint").finish_non_exhaustive()
    }
}

/// The concrete type of a paint object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PaintType {
    Undefined,
    Shape,
    Scene,
    Picture,
    Text,
    LinearGradient,
    RadialGradient,
}

impl PaintType {
    pub(crate) fn from_raw(t: sys::Tvg_Type) -> Self {
        match t {
            sys::Tvg_Type::TVG_TYPE_SHAPE => Self::Shape,
            sys::Tvg_Type::TVG_TYPE_SCENE => Self::Scene,
            sys::Tvg_Type::TVG_TYPE_PICTURE => Self::Picture,
            sys::Tvg_Type::TVG_TYPE_TEXT => Self::Text,
            sys::Tvg_Type::TVG_TYPE_LINEAR_GRAD => Self::LinearGradient,
            sys::Tvg_Type::TVG_TYPE_RADIAL_GRAD => Self::RadialGradient,
            _ => Self::Undefined,
        }
    }
}

/// Common trait for all paint objects (Shape, Scene, Picture, Text).
///
/// Paint represents a graphical element that can be added to a canvas
/// with transformations, opacity, masking, and clipping.
pub trait Paint {
    /// Returns the raw `Tvg_Paint` pointer.
    fn raw(&self) -> sys::Tvg_Paint;

    /// Consumes self and returns the raw pointer, transferring ownership.
    fn into_raw(self) -> sys::Tvg_Paint;

    /// Sets the opacity of the paint object (0 = transparent, 255 = opaque).
    fn set_opacity(&mut self, opacity: u8) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_paint_set_opacity(self.raw(), opacity) })
    }

    /// Gets the opacity of the paint object.
    fn opacity(&self) -> Result<u8> {
        let mut opacity: u8 = 0;
        Error::from_raw(unsafe { sys::tvg_paint_get_opacity(self.raw(), &raw mut opacity) })?;
        Ok(opacity)
    }

    /// Sets the visibility of the paint object.
    fn set_visible(&mut self, visible: bool) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_paint_set_visible(self.raw(), visible) })
    }

    /// Gets the visibility status of the paint object.
    fn visible(&self) -> bool {
        unsafe { sys::tvg_paint_get_visible(self.raw()) }
    }

    /// Scales the paint object by the given factor.
    fn scale(&mut self, factor: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_paint_scale(self.raw(), factor) })
    }

    /// Rotates the paint object by the given angle in degrees.
    fn rotate(&mut self, degree: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_paint_rotate(self.raw(), degree) })
    }

    /// Translates the paint object by the given offset.
    fn translate(&mut self, x: f32, y: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_paint_translate(self.raw(), x, y) })
    }

    /// Sets the affine transformation matrix.
    fn set_transform(&mut self, m: &Matrix) -> Result<()> {
        let raw_m = m.to_raw();
        Error::from_raw(unsafe { sys::tvg_paint_set_transform(self.raw(), &raw const raw_m) })
    }

    /// Gets the affine transformation matrix.
    fn transform(&self) -> Result<Matrix> {
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
        Error::from_raw(unsafe { sys::tvg_paint_get_transform(self.raw(), &raw mut m) })?;
        Ok(Matrix::from_raw(m))
    }

    /// Gets the axis-aligned bounding box (AABB): `(x, y, width, height)`.
    fn bounds(&self) -> Result<(f32, f32, f32, f32)> {
        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        let mut w: f32 = 0.0;
        let mut h: f32 = 0.0;
        Error::from_raw(unsafe {
            sys::tvg_paint_get_aabb(self.raw(), &raw mut x, &raw mut y, &raw mut w, &raw mut h)
        })?;
        Ok((x, y, w, h))
    }

    /// Gets the oriented bounding box (OBB) as 4 corner points.
    fn bounds_obb(&self) -> Result<[Point; 4]> {
        let mut pts = [sys::Tvg_Point { x: 0.0, y: 0.0 }; 4];
        Error::from_raw(unsafe { sys::tvg_paint_get_obb(self.raw(), pts.as_mut_ptr()) })?;
        Ok([
            Point {
                x: pts[0].x,
                y: pts[0].y,
            },
            Point {
                x: pts[1].x,
                y: pts[1].y,
            },
            Point {
                x: pts[2].x,
                y: pts[2].y,
            },
            Point {
                x: pts[3].x,
                y: pts[3].y,
            },
        ])
    }

    /// Clips the drawing region to the specified shape's paths.
    ///
    /// Consumes the `clipper`: the C side stores its raw pointer in this
    /// paint's state and refcount-manages its lifetime via the canvas's
    /// destruction.  Taking ownership here lets us call `into_raw` so the
    /// Rust wrapper's `Drop` never tries to delete a handle the C side
    /// still references.
    fn set_clip(&mut self, clipper: Shape<'_>) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_paint_set_clip(self.raw(), clipper.into_raw()) })
    }

    /// Gets the clip shape, if any.
    fn clip(&self) -> Option<Shape<'_>> {
        let raw = unsafe { sys::tvg_paint_get_clip(self.raw()) };
        if raw.is_null() {
            None
        } else {
            Some(unsafe { Shape::from_raw(raw, false) })
        }
    }

    /// Sets the masking target and method.
    ///
    /// Consumes the `mask`: the C side stores its raw pointer in this
    /// paint's state and refcount-manages its lifetime via the canvas's
    /// destruction.  Taking ownership here lets us call `into_raw` so the
    /// Rust wrapper's `Drop` never tries to delete a handle the C side
    /// still references.
    fn set_mask<P: Paint>(&mut self, mask: P, method: MaskMethod) -> Result<()> {
        Error::from_raw(unsafe {
            sys::tvg_paint_set_mask_method(self.raw(), mask.into_raw(), method.to_raw())
        })
    }

    /// Gets the mask target and method, if a mask is set.
    ///
    /// Returns `None` when no mask is attached.  The returned
    /// [`BorrowedPaint`] is read-only and tied to the source paint's
    /// lifetime — dropping the source invalidates the borrow.
    ///
    /// # Why a borrow rather than `Shape<'_>`
    ///
    /// `set_mask` accepts any `Paint` (Shape, Scene, Picture, Text),
    /// so the returned mask cannot be type-pinned to `Shape` without
    /// risking a silent type-pun.  `BorrowedPaint::paint_type` lets
    /// callers dispatch on the runtime type.
    fn mask(&self) -> Option<(BorrowedPaint<'_>, MaskMethod)> {
        let mut target: sys::Tvg_Paint = core::ptr::null_mut();
        let mut method = sys::Tvg_Mask_Method::TVG_MASK_METHOD_NONE;

        // The C signature `tvg_paint_get_mask_method(paint, target,
        // method)` types `target` as `Tvg_Paint` (= *mut c_void) but
        // treats it internally as a `Paint**` out-param — passing a
        // real paint handle here would corrupt that paint's vtable.
        // We pass the address of a local `Tvg_Paint` slot cast to
        // `Tvg_Paint`, which is what the C side actually wants.
        let r = unsafe {
            sys::tvg_paint_get_mask_method(
                self.raw(),
                (&raw mut target).cast::<sys::_Tvg_Paint>(),
                &raw mut method,
            )
        };

        if Error::from_raw(r).is_err() || target.is_null() {
            return None;
        }
        Some((
            BorrowedPaint {
                raw: target,
                _life: core::marker::PhantomData,
            },
            MaskMethod::from_raw(method),
        ))
    }

    /// Sets the blending method.
    fn set_blend(&mut self, method: BlendMethod) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_paint_set_blend_method(self.raw(), method.to_raw()) })
    }

    /// Sets the paint ID.
    fn set_id(&mut self, id: u32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_paint_set_id(self.raw(), id) })
    }

    /// Gets the paint ID.
    fn id(&self) -> u32 {
        unsafe { sys::tvg_paint_get_id(self.raw()) }
    }

    /// Gets the concrete type of this paint object.
    fn paint_type(&self) -> Result<PaintType> {
        let mut t = sys::Tvg_Type::TVG_TYPE_UNDEF;
        Error::from_raw(unsafe { sys::tvg_paint_get_type(self.raw(), &raw mut t) })?;
        Ok(PaintType::from_raw(t))
    }

    /// Checks whether a region intersects the filled area (hit-testing).
    fn intersects(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        unsafe { sys::tvg_paint_intersects(self.raw(), x, y, w, h) }
    }

    /// Duplicates this paint object.
    fn duplicate(&self) -> Option<Self>
    where
        Self: Sized,
    {
        let raw = unsafe { sys::tvg_paint_duplicate(self.raw()) };
        if raw.is_null() {
            None
        } else {
            // Safety: duplicate returns a new owned paint
            Some(unsafe { Self::from_raw_paint(raw) })
        }
    }

    // Note: paint_ref/paint_unref/ref_cnt/parent_raw are intentionally
    // not exposed. They manipulate ThorVG's internal reference counting
    // which conflicts with Rust's ownership model. Misuse (e.g.
    // paint_unref(true)) causes double-free when Drop runs. Use
    // thorvg-sys directly if you need low-level refcount control.

    /// Constructs a new owned instance from a raw `Tvg_Paint` pointer.
    ///
    /// # Safety
    /// `raw` must be a valid, owned `Tvg_Paint`.
    unsafe fn from_raw_paint(raw: sys::Tvg_Paint) -> Self
    where
        Self: Sized;
}
