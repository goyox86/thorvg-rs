use crate::error::{Error, Result};
use crate::gradient::{LinearGradient, RadialGradient};
use crate::paint::{Paint, Point};
use thorvg_sys as ffi;

/// Fill rule for determining the interior of a shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FillRule {
    /// Non-zero winding rule.
    NonZero,
    /// Even-odd rule.
    EvenOdd,
}

impl FillRule {
    fn to_raw(self) -> ffi::Tvg_Fill_Rule {
        match self {
            FillRule::NonZero => ffi::Tvg_Fill_Rule::TVG_FILL_RULE_NON_ZERO,
            FillRule::EvenOdd => ffi::Tvg_Fill_Rule::TVG_FILL_RULE_EVEN_ODD,
        }
    }

    fn from_raw(r: ffi::Tvg_Fill_Rule) -> Self {
        match r {
            ffi::Tvg_Fill_Rule::TVG_FILL_RULE_EVEN_ODD => FillRule::EvenOdd,
            _ => FillRule::NonZero,
        }
    }
}

/// Stroke line cap style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StrokeCap {
    Butt,
    Round,
    Square,
}

impl StrokeCap {
    fn to_raw(self) -> ffi::Tvg_Stroke_Cap {
        match self {
            StrokeCap::Butt => ffi::Tvg_Stroke_Cap::TVG_STROKE_CAP_BUTT,
            StrokeCap::Round => ffi::Tvg_Stroke_Cap::TVG_STROKE_CAP_ROUND,
            StrokeCap::Square => ffi::Tvg_Stroke_Cap::TVG_STROKE_CAP_SQUARE,
        }
    }

    fn from_raw(c: ffi::Tvg_Stroke_Cap) -> Self {
        match c {
            ffi::Tvg_Stroke_Cap::TVG_STROKE_CAP_ROUND => StrokeCap::Round,
            ffi::Tvg_Stroke_Cap::TVG_STROKE_CAP_SQUARE => StrokeCap::Square,
            _ => StrokeCap::Butt,
        }
    }
}

/// Stroke line join style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StrokeJoin {
    Miter,
    Round,
    Bevel,
}

impl StrokeJoin {
    fn to_raw(self) -> ffi::Tvg_Stroke_Join {
        match self {
            StrokeJoin::Miter => ffi::Tvg_Stroke_Join::TVG_STROKE_JOIN_MITER,
            StrokeJoin::Round => ffi::Tvg_Stroke_Join::TVG_STROKE_JOIN_ROUND,
            StrokeJoin::Bevel => ffi::Tvg_Stroke_Join::TVG_STROKE_JOIN_BEVEL,
        }
    }

    fn from_raw(j: ffi::Tvg_Stroke_Join) -> Self {
        match j {
            ffi::Tvg_Stroke_Join::TVG_STROKE_JOIN_ROUND => StrokeJoin::Round,
            ffi::Tvg_Stroke_Join::TVG_STROKE_JOIN_BEVEL => StrokeJoin::Bevel,
            _ => StrokeJoin::Miter,
        }
    }
}

/// A two-dimensional shape with path, fill, and stroke properties.
pub struct Shape {
    raw: ffi::Tvg_Paint,
    owned: bool,
}

impl Shape {
    /// Creates a new Shape object.
    pub fn new() -> Self {
        let raw = unsafe { ffi::tvg_shape_new() };
        assert!(!raw.is_null(), "failed to create Shape");
        Self { raw, owned: true }
    }

    /// Wraps an existing raw paint pointer.
    ///
    /// # Safety
    /// The pointer must be a valid `Tvg_Paint` of type Shape.
    pub(crate) unsafe fn from_raw(raw: ffi::Tvg_Paint, owned: bool) -> Self {
        Self { raw, owned }
    }

    // ── Path commands ──────────────────────────────────────────────

    /// Resets the shape path. Retains color, fill, and stroke properties.
    pub fn reset(&mut self) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_reset(self.raw) })
    }

    /// Sets the starting point of a new sub-path.
    pub fn move_to(&mut self, x: f32, y: f32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_move_to(self.raw, x, y) })
    }

    /// Draws a line from the current point to `(x, y)`.
    pub fn line_to(&mut self, x: f32, y: f32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_line_to(self.raw, x, y) })
    }

    /// Draws a cubic Bézier curve.
    pub fn cubic_to(
        &mut self,
        cx1: f32,
        cy1: f32,
        cx2: f32,
        cy2: f32,
        x: f32,
        y: f32,
    ) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_cubic_to(self.raw, cx1, cy1, cx2, cy2, x, y) })
    }

    /// Closes the current sub-path.
    pub fn close(&mut self) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_close(self.raw) })
    }

    /// Appends a raw sub-path from commands and points.
    #[allow(clippy::cast_possible_truncation)]
    pub fn append_path(&mut self, cmds: &[u8], pts: &[Point]) -> Result<()> {
        let raw_pts: alloc::vec::Vec<ffi::Tvg_Point> = pts
            .iter()
            .map(|p| ffi::Tvg_Point { x: p.x, y: p.y })
            .collect();
        Error::from_raw(unsafe {
            ffi::tvg_shape_append_path(
                self.raw,
                cmds.as_ptr(),
                cmds.len() as u32,
                raw_pts.as_ptr(),
                raw_pts.len() as u32,
            )
        })
    }

    /// Gets the current path data (commands count and points count).
    #[allow(clippy::cast_possible_truncation)]
    pub fn path(&self) -> Result<(u32, u32)> {
        let mut cmds_cnt: u32 = 0;
        let mut pts_cnt: u32 = 0;
        Error::from_raw(unsafe {
            ffi::tvg_shape_get_path(
                self.raw,
                core::ptr::null_mut(),
                &raw mut cmds_cnt,
                core::ptr::null_mut(),
                &raw mut pts_cnt,
            )
        })?;
        Ok((cmds_cnt, pts_cnt))
    }

    // ── Shape primitives ───────────────────────────────────────────

    /// Appends a rectangle to the path.
    #[allow(clippy::too_many_arguments)]
    pub fn append_rect(
        &mut self,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        rx: f32,
        ry: f32,
        cw: bool,
    ) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_append_rect(self.raw, x, y, w, h, rx, ry, cw) })
    }

    /// Appends an ellipse (or circle) to the path.
    pub fn append_circle(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, cw: bool) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_append_circle(self.raw, cx, cy, rx, ry, cw) })
    }

    // ── Fill ───────────────────────────────────────────────────────

    /// Sets the fill color (RGBA).
    pub fn set_fill_color(&mut self, r: u8, g: u8, b: u8, a: u8) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_set_fill_color(self.raw, r, g, b, a) })
    }

    /// Gets the fill color (RGBA).
    pub fn fill_color(&self) -> Result<(u8, u8, u8, u8)> {
        let (mut r, mut g, mut b, mut a) = (0u8, 0u8, 0u8, 0u8);
        Error::from_raw(unsafe {
            ffi::tvg_shape_get_fill_color(self.raw, &raw mut r, &raw mut g, &raw mut b, &raw mut a)
        })?;
        Ok((r, g, b, a))
    }

    /// Sets the fill rule.
    pub fn set_fill_rule(&mut self, rule: FillRule) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_set_fill_rule(self.raw, rule.to_raw()) })
    }

    /// Gets the fill rule.
    pub fn fill_rule(&self) -> Result<FillRule> {
        let mut rule = ffi::Tvg_Fill_Rule::TVG_FILL_RULE_NON_ZERO;
        Error::from_raw(unsafe { ffi::tvg_shape_get_fill_rule(self.raw, &raw mut rule) })?;
        Ok(FillRule::from_raw(rule))
    }

    /// Sets a linear gradient fill.
    pub fn set_linear_gradient(&mut self, grad: LinearGradient) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_set_gradient(self.raw, grad.into_raw()) })
    }

    /// Sets a radial gradient fill.
    pub fn set_radial_gradient(&mut self, grad: RadialGradient) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_set_gradient(self.raw, grad.into_raw()) })
    }

    /// Gets the raw gradient fill handle (borrowed, not owned).
    ///
    /// Returns `None` if no gradient is set. The returned handle is owned
    /// by the shape and must not be freed.
    pub fn gradient_raw(&self) -> Option<ffi::Tvg_Gradient> {
        let mut grad: ffi::Tvg_Gradient = core::ptr::null_mut();
        let r = unsafe { ffi::tvg_shape_get_gradient(self.raw, &raw mut grad) };
        if Error::from_raw(r).is_err() || grad.is_null() {
            None
        } else {
            Some(grad)
        }
    }

    /// Sets the rendering order of stroke and fill.
    pub fn set_paint_order(&mut self, stroke_first: bool) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_set_paint_order(self.raw, stroke_first) })
    }

    // ── Stroke ─────────────────────────────────────────────────────

    /// Sets the stroke width.
    pub fn set_stroke_width(&mut self, width: f32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_set_stroke_width(self.raw, width) })
    }

    /// Gets the stroke width.
    pub fn stroke_width(&self) -> Result<f32> {
        let mut width: f32 = 0.0;
        Error::from_raw(unsafe { ffi::tvg_shape_get_stroke_width(self.raw, &raw mut width) })?;
        Ok(width)
    }

    /// Sets the stroke color (RGBA).
    pub fn set_stroke_color(&mut self, r: u8, g: u8, b: u8, a: u8) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_set_stroke_color(self.raw, r, g, b, a) })
    }

    /// Gets the stroke color (RGBA).
    pub fn stroke_color(&self) -> Result<(u8, u8, u8, u8)> {
        let (mut r, mut g, mut b, mut a) = (0u8, 0u8, 0u8, 0u8);
        Error::from_raw(unsafe {
            ffi::tvg_shape_get_stroke_color(
                self.raw, &raw mut r, &raw mut g, &raw mut b, &raw mut a,
            )
        })?;
        Ok((r, g, b, a))
    }

    /// Sets the stroke line cap style.
    pub fn set_stroke_cap(&mut self, cap: StrokeCap) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_set_stroke_cap(self.raw, cap.to_raw()) })
    }

    /// Gets the stroke line cap style.
    pub fn stroke_cap(&self) -> Result<StrokeCap> {
        let mut cap = ffi::Tvg_Stroke_Cap::TVG_STROKE_CAP_BUTT;
        Error::from_raw(unsafe { ffi::tvg_shape_get_stroke_cap(self.raw, &raw mut cap) })?;
        Ok(StrokeCap::from_raw(cap))
    }

    /// Sets the stroke line join style.
    pub fn set_stroke_join(&mut self, join: StrokeJoin) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_set_stroke_join(self.raw, join.to_raw()) })
    }

    /// Gets the stroke line join style.
    pub fn stroke_join(&self) -> Result<StrokeJoin> {
        let mut join = ffi::Tvg_Stroke_Join::TVG_STROKE_JOIN_MITER;
        Error::from_raw(unsafe { ffi::tvg_shape_get_stroke_join(self.raw, &raw mut join) })?;
        Ok(StrokeJoin::from_raw(join))
    }

    /// Sets the stroke miter limit.
    pub fn set_stroke_miterlimit(&mut self, miterlimit: f32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_set_stroke_miterlimit(self.raw, miterlimit) })
    }

    /// Gets the stroke miter limit.
    pub fn stroke_miterlimit(&self) -> Result<f32> {
        let mut ml: f32 = 0.0;
        Error::from_raw(unsafe { ffi::tvg_shape_get_stroke_miterlimit(self.raw, &raw mut ml) })?;
        Ok(ml)
    }

    /// Sets the stroke dash pattern.
    #[allow(clippy::cast_possible_truncation)]
    pub fn set_stroke_dash(&mut self, pattern: &[f32], offset: f32) -> Result<()> {
        Error::from_raw(unsafe {
            ffi::tvg_shape_set_stroke_dash(self.raw, pattern.as_ptr(), pattern.len() as u32, offset)
        })
    }

    /// Gets the stroke dash pattern and offset.
    pub fn stroke_dash(&self) -> Result<(alloc::vec::Vec<f32>, f32)> {
        let mut ptr: *const f32 = core::ptr::null();
        let mut cnt: u32 = 0;
        let mut offset: f32 = 0.0;
        Error::from_raw(unsafe {
            ffi::tvg_shape_get_stroke_dash(self.raw, &raw mut ptr, &raw mut cnt, &raw mut offset)
        })?;
        let pattern = if ptr.is_null() || cnt == 0 {
            alloc::vec::Vec::new()
        } else {
            unsafe { core::slice::from_raw_parts(ptr, cnt as usize) }.to_vec()
        };
        Ok((pattern, offset))
    }

    /// Gets the raw stroke gradient fill handle (borrowed, not owned).
    ///
    /// Returns `None` if no stroke gradient is set.
    pub fn stroke_gradient_raw(&self) -> Option<ffi::Tvg_Gradient> {
        let mut grad: ffi::Tvg_Gradient = core::ptr::null_mut();
        let r = unsafe { ffi::tvg_shape_get_stroke_gradient(self.raw, &raw mut grad) };
        if Error::from_raw(r).is_err() || grad.is_null() {
            None
        } else {
            Some(grad)
        }
    }

    /// Sets the stroke gradient fill.
    pub fn set_stroke_gradient(&mut self, grad: LinearGradient) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_set_stroke_gradient(self.raw, grad.into_raw()) })
    }

    /// Sets the trim path (visible segment along the path).
    pub fn set_trimpath(&mut self, begin: f32, end: f32, simultaneous: bool) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_shape_set_trimpath(self.raw, begin, end, simultaneous) })
    }
}

impl Paint for Shape {
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

impl Drop for Shape {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                ffi::tvg_paint_rel(self.raw);
            }
        }
    }
}

impl core::fmt::Debug for Shape {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Shape").finish_non_exhaustive()
    }
}
