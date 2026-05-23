use crate::error::{Error, Result};
use crate::shape::Shape;
use thorvg_sys as ffi;

/// A 3×3 affine transformation matrix.
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

    fn to_raw(self) -> ffi::Tvg_Matrix {
        ffi::Tvg_Matrix {
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

    fn from_raw(m: ffi::Tvg_Matrix) -> Self {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
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
    fn to_raw(self) -> ffi::Tvg_Blend_Method {
        match self {
            Self::Normal => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_NORMAL,
            Self::Multiply => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_MULTIPLY,
            Self::Screen => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_SCREEN,
            Self::Overlay => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_OVERLAY,
            Self::Darken => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_DARKEN,
            Self::Lighten => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_LIGHTEN,
            Self::ColorDodge => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_COLORDODGE,
            Self::ColorBurn => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_COLORBURN,
            Self::HardLight => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_HARDLIGHT,
            Self::SoftLight => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_SOFTLIGHT,
            Self::Difference => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_DIFFERENCE,
            Self::Exclusion => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_EXCLUSION,
            Self::Hue => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_HUE,
            Self::Saturation => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_SATURATION,
            Self::Color => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_COLOR,
            Self::Luminosity => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_LUMINOSITY,
            Self::Add => ffi::Tvg_Blend_Method::TVG_BLEND_METHOD_ADD,
        }
    }
}

/// Masking method for combining two paint objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
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
    fn to_raw(self) -> ffi::Tvg_Mask_Method {
        match self {
            Self::None => ffi::Tvg_Mask_Method::TVG_MASK_METHOD_NONE,
            Self::Alpha => ffi::Tvg_Mask_Method::TVG_MASK_METHOD_ALPHA,
            Self::InvAlpha => ffi::Tvg_Mask_Method::TVG_MASK_METHOD_INVERSE_ALPHA,
            Self::Luma => ffi::Tvg_Mask_Method::TVG_MASK_METHOD_LUMA,
            Self::InvLuma => ffi::Tvg_Mask_Method::TVG_MASK_METHOD_INVERSE_LUMA,
            Self::Add => ffi::Tvg_Mask_Method::TVG_MASK_METHOD_ADD,
            Self::Subtract => ffi::Tvg_Mask_Method::TVG_MASK_METHOD_SUBTRACT,
            Self::Intersect => ffi::Tvg_Mask_Method::TVG_MASK_METHOD_INTERSECT,
            Self::Difference => ffi::Tvg_Mask_Method::TVG_MASK_METHOD_DIFFERENCE,
            Self::Lighten => ffi::Tvg_Mask_Method::TVG_MASK_METHOD_LIGHTEN,
            Self::Darken => ffi::Tvg_Mask_Method::TVG_MASK_METHOD_DARKEN,
        }
    }
}

/// The concrete type of a paint object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaintType {
    Undefined,
    Shape,
    Scene,
    Picture,
    Text,
}

impl PaintType {
    pub(crate) fn from_raw(t: ffi::Tvg_Type) -> Self {
        match t {
            ffi::Tvg_Type::TVG_TYPE_SHAPE => Self::Shape,
            ffi::Tvg_Type::TVG_TYPE_SCENE => Self::Scene,
            ffi::Tvg_Type::TVG_TYPE_PICTURE => Self::Picture,
            ffi::Tvg_Type::TVG_TYPE_TEXT => Self::Text,
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
    fn raw(&self) -> ffi::Tvg_Paint;

    /// Consumes self and returns the raw pointer, transferring ownership.
    fn into_raw(self) -> ffi::Tvg_Paint;

    /// Sets the opacity of the paint object (0 = transparent, 255 = opaque).
    fn set_opacity(&mut self, opacity: u8) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_paint_set_opacity(self.raw(), opacity) })
    }

    /// Gets the opacity of the paint object.
    fn opacity(&self) -> Result<u8> {
        let mut opacity: u8 = 0;
        Error::from_raw(unsafe { ffi::tvg_paint_get_opacity(self.raw(), &raw mut opacity) })?;
        Ok(opacity)
    }

    /// Sets the visibility of the paint object.
    fn set_visible(&mut self, visible: bool) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_paint_set_visible(self.raw(), visible) })
    }

    /// Gets the visibility status of the paint object.
    fn visible(&self) -> bool {
        unsafe { ffi::tvg_paint_get_visible(self.raw()) }
    }

    /// Scales the paint object by the given factor.
    fn scale(&mut self, factor: f32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_paint_scale(self.raw(), factor) })
    }

    /// Rotates the paint object by the given angle in degrees.
    fn rotate(&mut self, degree: f32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_paint_rotate(self.raw(), degree) })
    }

    /// Translates the paint object by the given offset.
    fn translate(&mut self, x: f32, y: f32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_paint_translate(self.raw(), x, y) })
    }

    /// Sets the affine transformation matrix.
    fn set_transform(&mut self, m: &Matrix) -> Result<()> {
        let raw_m = m.to_raw();
        Error::from_raw(unsafe { ffi::tvg_paint_set_transform(self.raw(), &raw const raw_m) })
    }

    /// Gets the affine transformation matrix.
    fn transform(&self) -> Result<Matrix> {
        let mut m = ffi::Tvg_Matrix {
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
        Error::from_raw(unsafe { ffi::tvg_paint_get_transform(self.raw(), &raw mut m) })?;
        Ok(Matrix::from_raw(m))
    }

    /// Gets the axis-aligned bounding box (AABB): `(x, y, width, height)`.
    fn bounds(&self) -> Result<(f32, f32, f32, f32)> {
        let mut x: f32 = 0.0;
        let mut y: f32 = 0.0;
        let mut w: f32 = 0.0;
        let mut h: f32 = 0.0;
        Error::from_raw(unsafe {
            ffi::tvg_paint_get_aabb(self.raw(), &raw mut x, &raw mut y, &raw mut w, &raw mut h)
        })?;
        Ok((x, y, w, h))
    }

    /// Gets the oriented bounding box (OBB) as 4 corner points.
    fn bounds_obb(&self) -> Result<[Point; 4]> {
        let mut pts = [ffi::Tvg_Point { x: 0.0, y: 0.0 }; 4];
        Error::from_raw(unsafe { ffi::tvg_paint_get_obb(self.raw(), pts.as_mut_ptr()) })?;
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
    fn set_clip(&mut self, clipper: &Shape) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_paint_set_clip(self.raw(), clipper.raw()) })
    }

    /// Gets the clip shape, if any.
    fn clip(&self) -> Option<Shape> {
        let raw = unsafe { ffi::tvg_paint_get_clip(self.raw()) };
        if raw.is_null() {
            None
        } else {
            Some(unsafe { Shape::from_raw(raw, false) })
        }
    }

    /// Sets the masking target and method.
    fn set_mask<P: Paint>(&mut self, target: &P, method: MaskMethod) -> Result<()> {
        Error::from_raw(unsafe {
            ffi::tvg_paint_set_mask_method(self.raw(), target.raw(), method.to_raw())
        })
    }

    /// Gets the masking method for a given target.
    fn mask_method<P: Paint>(&self, target: &P) -> Result<MaskMethod> {
        let mut method = ffi::Tvg_Mask_Method::TVG_MASK_METHOD_NONE;
        Error::from_raw(unsafe {
            ffi::tvg_paint_get_mask_method(self.raw(), target.raw(), &raw mut method)
        })?;
        Ok(match method {
            ffi::Tvg_Mask_Method::TVG_MASK_METHOD_ALPHA => MaskMethod::Alpha,
            ffi::Tvg_Mask_Method::TVG_MASK_METHOD_INVERSE_ALPHA => MaskMethod::InvAlpha,
            ffi::Tvg_Mask_Method::TVG_MASK_METHOD_LUMA => MaskMethod::Luma,
            ffi::Tvg_Mask_Method::TVG_MASK_METHOD_INVERSE_LUMA => MaskMethod::InvLuma,
            ffi::Tvg_Mask_Method::TVG_MASK_METHOD_ADD => MaskMethod::Add,
            ffi::Tvg_Mask_Method::TVG_MASK_METHOD_SUBTRACT => MaskMethod::Subtract,
            ffi::Tvg_Mask_Method::TVG_MASK_METHOD_INTERSECT => MaskMethod::Intersect,
            ffi::Tvg_Mask_Method::TVG_MASK_METHOD_DIFFERENCE => MaskMethod::Difference,
            ffi::Tvg_Mask_Method::TVG_MASK_METHOD_LIGHTEN => MaskMethod::Lighten,
            ffi::Tvg_Mask_Method::TVG_MASK_METHOD_DARKEN => MaskMethod::Darken,
            _ => MaskMethod::None,
        })
    }

    /// Sets the blending method.
    fn set_blend(&mut self, method: BlendMethod) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_paint_set_blend_method(self.raw(), method.to_raw()) })
    }

    /// Sets the paint ID.
    fn set_id(&mut self, id: u32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_paint_set_id(self.raw(), id) })
    }

    /// Gets the paint ID.
    fn id(&self) -> u32 {
        unsafe { ffi::tvg_paint_get_id(self.raw()) }
    }

    /// Gets the concrete type of this paint object.
    fn paint_type(&self) -> Result<PaintType> {
        let mut t = ffi::Tvg_Type::TVG_TYPE_UNDEF;
        Error::from_raw(unsafe { ffi::tvg_paint_get_type(self.raw(), &raw mut t) })?;
        Ok(PaintType::from_raw(t))
    }

    /// Checks whether a region intersects the filled area (hit-testing).
    fn intersects(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        unsafe { ffi::tvg_paint_intersects(self.raw(), x, y, w, h) }
    }

    /// Duplicates this paint object.
    fn duplicate(&self) -> Option<Self>
    where
        Self: Sized,
    {
        let raw = unsafe { ffi::tvg_paint_duplicate(self.raw()) };
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
    unsafe fn from_raw_paint(raw: ffi::Tvg_Paint) -> Self
    where
        Self: Sized;
}
