use crate::color::Rgba;
use crate::error::{Error, Result};
use crate::gradient::{BorrowedGradient, LinearGradient, RadialGradient};
use crate::paint::{Paint, Point};
use crate::path::{Path, PathCommand};
use thorvg_sys as sys;

/// An axis-aligned rectangle, optionally rounded.
///
/// Parameter bundle for [`Shape::append_rect`], mirroring the C call
/// `tvg_shape_append_rect(x, y, w, h, rx, ry, cw)`.
///
/// Three construction styles are supported:
///
/// ```ignore
/// // 1. Struct literal:
/// Rect { x: 0.0, y: 0.0, width: 100.0, height: 50.0,
///        rx: 0.0, ry: 0.0, cw: true }
///
/// // 2. Default + field override:
/// Rect { width: 100.0, height: 50.0, ..Default::default() }
///
/// // 3. Builder:
/// Rect::new(0.0, 0.0, 100.0, 50.0).corner_radius(8.0)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Top-left X coordinate.
    pub x: f32,
    /// Top-left Y coordinate.
    pub y: f32,
    /// Width along the X axis.
    pub width: f32,
    /// Height along the Y axis.
    pub height: f32,
    /// Corner radius on the X axis.  `0.0` for sharp corners.
    pub rx: f32,
    /// Corner radius on the Y axis.  `0.0` for sharp corners.
    pub ry: f32,
    /// Winding direction.  `true` = clockwise (the C-side default),
    /// `false` = counter-clockwise.
    pub cw: bool,
}

impl Rect {
    /// Sharp-cornered rectangle, clockwise winding.
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
            rx: 0.0,
            ry: 0.0,
            cw: true,
        }
    }

    /// Sets the X-axis corner radius.
    #[must_use]
    pub const fn rx(mut self, rx: f32) -> Self {
        self.rx = rx;
        self
    }

    /// Sets the Y-axis corner radius.
    #[must_use]
    pub const fn ry(mut self, ry: f32) -> Self {
        self.ry = ry;
        self
    }

    /// Sets both `rx` and `ry` to the same value (uniform rounded
    /// corners).
    #[must_use]
    pub const fn corner_radius(mut self, r: f32) -> Self {
        self.rx = r;
        self.ry = r;
        self
    }

    /// Switches winding to counter-clockwise.
    #[must_use]
    pub const fn ccw(mut self) -> Self {
        self.cw = false;
        self
    }
}

/// An ellipse (or circle, when `rx == ry`).
///
/// Named `Circle` to match the C function name
/// `tvg_shape_append_circle`, but the underlying primitive is a
/// **general axis-aligned ellipse** — use [`Circle::new`] for the
/// circular case and [`Circle::ellipse`] when the radii differ.
///
/// Same three construction styles as [`Rect`] are supported.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Circle {
    /// Center X coordinate.
    pub cx: f32,
    /// Center Y coordinate.
    pub cy: f32,
    /// Radius on the X axis.
    pub rx: f32,
    /// Radius on the Y axis.
    pub ry: f32,
    /// Winding direction.  `true` = clockwise (the C-side default),
    /// `false` = counter-clockwise.
    pub cw: bool,
}

impl Circle {
    /// True circle centred at `(cx, cy)` with radius `r`.
    #[must_use]
    pub const fn new(cx: f32, cy: f32, r: f32) -> Self {
        Self {
            cx,
            cy,
            rx: r,
            ry: r,
            cw: true,
        }
    }

    /// Axis-aligned ellipse centred at `(cx, cy)` with separate
    /// horizontal and vertical radii.
    #[must_use]
    pub const fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> Self {
        Self {
            cx,
            cy,
            rx,
            ry,
            cw: true,
        }
    }

    /// Switches winding to counter-clockwise.
    #[must_use]
    pub const fn ccw(mut self) -> Self {
        self.cw = false;
        self
    }
}

/// Fill rule for determining the interior of a shape.
///
/// Exhaustive: the C header documents both values
/// (`Tvg_Fill_Rule`) and has not grown since the enum was
/// introduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FillRule {
    /// Non-zero winding rule.
    NonZero,
    /// Even-odd rule.
    EvenOdd,
}

impl FillRule {
    fn to_raw(self) -> sys::Tvg_Fill_Rule {
        match self {
            FillRule::NonZero => sys::Tvg_Fill_Rule::TVG_FILL_RULE_NON_ZERO,
            FillRule::EvenOdd => sys::Tvg_Fill_Rule::TVG_FILL_RULE_EVEN_ODD,
        }
    }

    fn from_raw(r: sys::Tvg_Fill_Rule) -> Self {
        match r {
            sys::Tvg_Fill_Rule::TVG_FILL_RULE_NON_ZERO => FillRule::NonZero,
            sys::Tvg_Fill_Rule::TVG_FILL_RULE_EVEN_ODD => FillRule::EvenOdd,
        }
    }
}

/// Stroke line cap style.
///
/// Exhaustive: the C header documents all three values
/// (`Tvg_Stroke_Cap`) and has not grown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrokeCap {
    /// Butted cap — the stroke ends exactly at the endpoint.
    Butt,
    /// Rounded cap — a semicircle drawn at the endpoint.
    Round,
    /// Square cap — a half-square drawn past the endpoint.
    Square,
}

impl StrokeCap {
    fn to_raw(self) -> sys::Tvg_Stroke_Cap {
        match self {
            StrokeCap::Butt => sys::Tvg_Stroke_Cap::TVG_STROKE_CAP_BUTT,
            StrokeCap::Round => sys::Tvg_Stroke_Cap::TVG_STROKE_CAP_ROUND,
            StrokeCap::Square => sys::Tvg_Stroke_Cap::TVG_STROKE_CAP_SQUARE,
        }
    }

    fn from_raw(c: sys::Tvg_Stroke_Cap) -> Self {
        match c {
            sys::Tvg_Stroke_Cap::TVG_STROKE_CAP_BUTT => StrokeCap::Butt,
            sys::Tvg_Stroke_Cap::TVG_STROKE_CAP_ROUND => StrokeCap::Round,
            sys::Tvg_Stroke_Cap::TVG_STROKE_CAP_SQUARE => StrokeCap::Square,
        }
    }
}

/// Stroke line join style.
///
/// Exhaustive: the C header documents all three values
/// (`Tvg_Stroke_Join`) and has not grown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StrokeJoin {
    /// Mitered join — sharp corner, extending out to the miter
    /// limit.
    Miter,
    /// Rounded join — a circular arc smooths the corner.
    Round,
    /// Bevelled join — the corner is cut off with a straight edge.
    Bevel,
}

impl StrokeJoin {
    fn to_raw(self) -> sys::Tvg_Stroke_Join {
        match self {
            StrokeJoin::Miter => sys::Tvg_Stroke_Join::TVG_STROKE_JOIN_MITER,
            StrokeJoin::Round => sys::Tvg_Stroke_Join::TVG_STROKE_JOIN_ROUND,
            StrokeJoin::Bevel => sys::Tvg_Stroke_Join::TVG_STROKE_JOIN_BEVEL,
        }
    }

    fn from_raw(j: sys::Tvg_Stroke_Join) -> Self {
        match j {
            sys::Tvg_Stroke_Join::TVG_STROKE_JOIN_MITER => StrokeJoin::Miter,
            sys::Tvg_Stroke_Join::TVG_STROKE_JOIN_ROUND => StrokeJoin::Round,
            sys::Tvg_Stroke_Join::TVG_STROKE_JOIN_BEVEL => StrokeJoin::Bevel,
        }
    }
}

/// A two-dimensional shape with path, fill, and stroke properties.
///
/// The lifetime `'eng` ties this shape to a [`Thorvg`](crate::Thorvg) engine
/// instance, ensuring the engine cannot be terminated while the shape exists.
/// Create shapes via [`Thorvg::shape()`](crate::Thorvg::shape).
pub struct Shape<'eng> {
    raw: sys::Tvg_Paint,
    owned: bool,
    _engine: core::marker::PhantomData<&'eng ()>,
}

// SAFETY: Same rationale as other ThorVG handle types — exclusive
// ownership of a C heap object; global state is mutex-protected.
unsafe impl Send for Shape<'_> {}

impl Shape<'_> {
    /// Creates a new Shape object.
    pub(crate) fn new() -> Result<Self> {
        let raw = unsafe { sys::tvg_shape_new() };
        if raw.is_null() {
            return Err(Error::FailedAllocation);
        }
        Ok(Self {
            raw,
            owned: true,
            _engine: core::marker::PhantomData,
        })
    }

    /// Wraps an existing raw paint pointer.
    ///
    /// # Safety
    /// The pointer must be a valid `Tvg_Paint` of type Shape.
    pub(crate) unsafe fn from_raw(raw: sys::Tvg_Paint, owned: bool) -> Self {
        Self {
            raw,
            owned,
            _engine: core::marker::PhantomData,
        }
    }

    // ── Path commands ──────────────────────────────────────────────

    /// Resets the shape path. Retains color, fill, and stroke properties.
    pub fn reset(&mut self) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_reset(self.raw) })
    }

    /// Sets the starting point of a new sub-path.
    pub fn move_to(&mut self, x: f32, y: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_move_to(self.raw, x, y) })
    }

    /// Draws a line from the current point to `(x, y)`.
    pub fn line_to(&mut self, x: f32, y: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_line_to(self.raw, x, y) })
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
        Error::from_raw(unsafe { sys::tvg_shape_cubic_to(self.raw, cx1, cy1, cx2, cy2, x, y) })
    }

    /// Closes the current sub-path.
    pub fn close(&mut self) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_close(self.raw) })
    }

    /// Appends a raw sub-path from commands and points.
    #[allow(clippy::cast_possible_truncation)]
    pub fn append_path(&mut self, cmds: &[u8], pts: &[Point]) -> Result<()> {
        let raw_pts: alloc::vec::Vec<sys::Tvg_Point> = pts
            .iter()
            .map(|p| sys::Tvg_Point { x: p.x, y: p.y })
            .collect();
        Error::from_raw(unsafe {
            sys::tvg_shape_append_path(
                self.raw,
                cmds.as_ptr(),
                cmds.len() as u32,
                raw_pts.as_ptr(),
                raw_pts.len() as u32,
            )
        })
    }

    /// Returns the shape's current path data.
    ///
    /// The result bundles the command and point buffers as a
    /// single [`Path`] value; walk it with [`Path::segments`] to
    /// receive typed [`Segment`](crate::Segment)s instead of
    /// managing a separate points cursor.
    ///
    /// Allocates two `Vec`s; for the cheap "how big is it?" query
    /// use [`path_counts`](Self::path_counts) instead, which
    /// passes `null` for the data pointers and avoids the copy.
    ///
    /// Unknown command bytes from the C side (none are expected
    /// for a thorvg-built shape) are dropped silently while
    /// keeping the points buffer intact, which keeps
    /// [`Path::segments`] from desynchronising.
    #[allow(clippy::cast_possible_truncation)]
    pub fn path(&self) -> Result<Path> {
        let mut cmd_ptr: *const sys::Tvg_Path_Command = core::ptr::null();
        let mut pts_ptr: *const sys::Tvg_Point = core::ptr::null();
        let mut cmds_cnt: u32 = 0;
        let mut pts_cnt: u32 = 0;
        Error::from_raw(unsafe {
            sys::tvg_shape_get_path(
                self.raw,
                &raw mut cmd_ptr,
                &raw mut cmds_cnt,
                &raw mut pts_ptr,
                &raw mut pts_cnt,
            )
        })?;

        let commands = if cmd_ptr.is_null() || cmds_cnt == 0 {
            alloc::vec::Vec::new()
        } else {
            // SAFETY: `cmd_ptr` was just returned non-null by
            // `tvg_shape_get_path` and points to `cmds_cnt`
            // command bytes living in the shape's path storage,
            // which outlives `&self`.
            let raw_cmds = unsafe { core::slice::from_raw_parts(cmd_ptr, cmds_cnt as usize) };
            raw_cmds
                .iter()
                .filter_map(|&c| PathCommand::from_raw(c))
                .collect()
        };

        let points = if pts_ptr.is_null() || pts_cnt == 0 {
            alloc::vec::Vec::new()
        } else {
            // SAFETY: same as above for the points buffer.
            let raw_pts = unsafe { core::slice::from_raw_parts(pts_ptr, pts_cnt as usize) };
            raw_pts.iter().map(|p| Point { x: p.x, y: p.y }).collect()
        };

        Ok(Path { commands, points })
    }

    /// Returns `(commands_count, points_count)` for the shape's
    /// current path without copying the buffers.
    ///
    /// Cheaper than [`path`](Self::path) when only the sizes are
    /// needed (passes `null` for the data pointers, matching the
    /// pre-typed-path Rust signature).
    #[allow(clippy::cast_possible_truncation)]
    pub fn path_counts(&self) -> Result<(u32, u32)> {
        let mut cmds_cnt: u32 = 0;
        let mut pts_cnt: u32 = 0;
        Error::from_raw(unsafe {
            sys::tvg_shape_get_path(
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

    /// Appends a rectangle (optionally rounded) to the path.
    ///
    /// See [`Rect`] for the parameter layout.
    pub fn append_rect(&mut self, rect: Rect) -> Result<()> {
        let Rect {
            x,
            y,
            width,
            height,
            rx,
            ry,
            cw,
        } = rect;
        Error::from_raw(unsafe {
            sys::tvg_shape_append_rect(self.raw, x, y, width, height, rx, ry, cw)
        })
    }

    /// Appends an ellipse to the path.  See [`Circle`] for the
    /// parameter layout — `Circle::new` for true circles,
    /// `Circle::ellipse` for elliptical shapes.
    pub fn append_circle(&mut self, circle: Circle) -> Result<()> {
        let Circle { cx, cy, rx, ry, cw } = circle;
        Error::from_raw(unsafe { sys::tvg_shape_append_circle(self.raw, cx, cy, rx, ry, cw) })
    }

    // ── Fill ───────────────────────────────────────────────────────

    /// Sets the fill color.
    pub fn set_fill_color(&mut self, color: Rgba) -> Result<()> {
        let Rgba { r, g, b, a } = color;
        Error::from_raw(unsafe { sys::tvg_shape_set_fill_color(self.raw, r, g, b, a) })
    }

    /// Gets the fill color.
    pub fn fill_color(&self) -> Result<Rgba> {
        let (mut r, mut g, mut b, mut a) = (0u8, 0u8, 0u8, 0u8);
        Error::from_raw(unsafe {
            sys::tvg_shape_get_fill_color(self.raw, &raw mut r, &raw mut g, &raw mut b, &raw mut a)
        })?;
        Ok(Rgba { r, g, b, a })
    }

    /// Sets the fill rule.
    pub fn set_fill_rule(&mut self, rule: FillRule) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_fill_rule(self.raw, rule.to_raw()) })
    }

    /// Gets the fill rule.
    pub fn fill_rule(&self) -> Result<FillRule> {
        let mut rule = sys::Tvg_Fill_Rule::TVG_FILL_RULE_NON_ZERO;
        Error::from_raw(unsafe { sys::tvg_shape_get_fill_rule(self.raw, &raw mut rule) })?;
        Ok(FillRule::from_raw(rule))
    }

    /// Sets a linear gradient fill.
    pub fn set_linear_gradient(&mut self, grad: LinearGradient<'_>) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_gradient(self.raw, grad.into_raw()) })
    }

    /// Sets a radial gradient fill.
    pub fn set_radial_gradient(&mut self, grad: RadialGradient<'_>) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_gradient(self.raw, grad.into_raw()) })
    }

    /// Returns a read-only view of the shape's fill gradient.
    ///
    /// * `Ok(None)`  — no gradient is set.
    /// * `Ok(Some(view))`  — fill gradient present; the view
    ///   discriminates linear vs radial at borrow time so subsequent
    ///   reads are type-checked.
    /// * `Err(_)`  — the C side rejected the read or returned an
    ///   unrecognised gradient kind.
    ///
    /// The borrow lifetime is tied to `&self`; the shape owns the
    /// underlying gradient and frees it on its own teardown.
    pub fn gradient(&self) -> Result<Option<BorrowedGradient<'_>>> {
        let mut grad: sys::Tvg_Gradient = core::ptr::null_mut();
        Error::from_raw(unsafe { sys::tvg_shape_get_gradient(self.raw, &raw mut grad) })?;
        if grad.is_null() {
            return Ok(None);
        }
        // SAFETY: `grad` was just returned non-null by
        // `tvg_shape_get_gradient`, and the shape (which owns it)
        // outlives `&self`.
        Ok(Some(unsafe { BorrowedGradient::from_raw(grad) }?))
    }

    /// Sets the rendering order of stroke and fill.
    pub fn set_paint_order(&mut self, stroke_first: bool) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_paint_order(self.raw, stroke_first) })
    }

    // ── Stroke ─────────────────────────────────────────────────────

    /// Sets the stroke width.
    pub fn set_stroke_width(&mut self, width: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_width(self.raw, width) })
    }

    /// Gets the stroke width.
    pub fn stroke_width(&self) -> Result<f32> {
        let mut width: f32 = 0.0;
        Error::from_raw(unsafe { sys::tvg_shape_get_stroke_width(self.raw, &raw mut width) })?;
        Ok(width)
    }

    /// Sets the stroke color.
    pub fn set_stroke_color(&mut self, color: Rgba) -> Result<()> {
        let Rgba { r, g, b, a } = color;
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_color(self.raw, r, g, b, a) })
    }

    /// Gets the stroke color.
    pub fn stroke_color(&self) -> Result<Rgba> {
        let (mut r, mut g, mut b, mut a) = (0u8, 0u8, 0u8, 0u8);
        Error::from_raw(unsafe {
            sys::tvg_shape_get_stroke_color(
                self.raw, &raw mut r, &raw mut g, &raw mut b, &raw mut a,
            )
        })?;
        Ok(Rgba { r, g, b, a })
    }

    /// Sets the stroke line cap style.
    pub fn set_stroke_cap(&mut self, cap: StrokeCap) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_cap(self.raw, cap.to_raw()) })
    }

    /// Gets the stroke line cap style.
    pub fn stroke_cap(&self) -> Result<StrokeCap> {
        let mut cap = sys::Tvg_Stroke_Cap::TVG_STROKE_CAP_BUTT;
        Error::from_raw(unsafe { sys::tvg_shape_get_stroke_cap(self.raw, &raw mut cap) })?;
        Ok(StrokeCap::from_raw(cap))
    }

    /// Sets the stroke line join style.
    pub fn set_stroke_join(&mut self, join: StrokeJoin) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_join(self.raw, join.to_raw()) })
    }

    /// Gets the stroke line join style.
    pub fn stroke_join(&self) -> Result<StrokeJoin> {
        let mut join = sys::Tvg_Stroke_Join::TVG_STROKE_JOIN_MITER;
        Error::from_raw(unsafe { sys::tvg_shape_get_stroke_join(self.raw, &raw mut join) })?;
        Ok(StrokeJoin::from_raw(join))
    }

    /// Sets the stroke miter limit.
    pub fn set_stroke_miterlimit(&mut self, miterlimit: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_miterlimit(self.raw, miterlimit) })
    }

    /// Gets the stroke miter limit.
    pub fn stroke_miterlimit(&self) -> Result<f32> {
        let mut ml: f32 = 0.0;
        Error::from_raw(unsafe { sys::tvg_shape_get_stroke_miterlimit(self.raw, &raw mut ml) })?;
        Ok(ml)
    }

    /// Sets the stroke dash pattern.
    #[allow(clippy::cast_possible_truncation)]
    pub fn set_stroke_dash(&mut self, pattern: &[f32], offset: f32) -> Result<()> {
        Error::from_raw(unsafe {
            sys::tvg_shape_set_stroke_dash(self.raw, pattern.as_ptr(), pattern.len() as u32, offset)
        })
    }

    /// Gets the stroke dash pattern and offset.
    pub fn stroke_dash(&self) -> Result<(alloc::vec::Vec<f32>, f32)> {
        let mut ptr: *const f32 = core::ptr::null();
        let mut cnt: u32 = 0;
        let mut offset: f32 = 0.0;
        Error::from_raw(unsafe {
            sys::tvg_shape_get_stroke_dash(self.raw, &raw mut ptr, &raw mut cnt, &raw mut offset)
        })?;
        let pattern = if ptr.is_null() || cnt == 0 {
            alloc::vec::Vec::new()
        } else {
            unsafe { core::slice::from_raw_parts(ptr, cnt as usize) }.to_vec()
        };
        Ok((pattern, offset))
    }

    /// Returns a read-only view of the shape's stroke gradient.
    ///
    /// Same semantics as [`gradient`](Self::gradient), but for the
    /// stroke gradient configured via
    /// [`set_stroke_linear_gradient`](Self::set_stroke_linear_gradient) /
    /// [`set_stroke_radial_gradient`](Self::set_stroke_radial_gradient).
    pub fn stroke_gradient(&self) -> Result<Option<BorrowedGradient<'_>>> {
        let mut grad: sys::Tvg_Gradient = core::ptr::null_mut();
        Error::from_raw(unsafe { sys::tvg_shape_get_stroke_gradient(self.raw, &raw mut grad) })?;
        if grad.is_null() {
            return Ok(None);
        }
        // SAFETY: see `gradient` above.
        Ok(Some(unsafe { BorrowedGradient::from_raw(grad) }?))
    }

    /// Sets a linear gradient as the stroke fill.
    ///
    /// Companion to [`set_linear_gradient`](Self::set_linear_gradient)
    /// on the fill side.
    pub fn set_stroke_linear_gradient(&mut self, grad: LinearGradient<'_>) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_gradient(self.raw, grad.into_raw()) })
    }

    /// Sets a radial gradient as the stroke fill.
    ///
    /// Companion to [`set_radial_gradient`](Self::set_radial_gradient)
    /// on the fill side.  The underlying C call
    /// (`tvg_shape_set_stroke_gradient`) takes a polymorphic
    /// `Tvg_Gradient` and the C++ implementation
    /// (`Shape::strokeFill(Fill*)`) accepts both gradient kinds —
    /// the Rust binding previously restricted the input type to
    /// linear by oversight.
    pub fn set_stroke_radial_gradient(&mut self, grad: RadialGradient<'_>) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_gradient(self.raw, grad.into_raw()) })
    }

    /// Sets the trim path (visible segment along the path).
    pub fn set_trimpath(&mut self, begin: f32, end: f32, simultaneous: bool) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_trimpath(self.raw, begin, end, simultaneous) })
    }
}

impl crate::paint::sealed::Sealed for Shape<'_> {}

impl Paint for Shape<'_> {
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

impl Drop for Shape<'_> {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                sys::tvg_paint_rel(self.raw);
            }
        }
    }
}

impl core::fmt::Debug for Shape<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Shape").finish_non_exhaustive()
    }
}
