//! Shapes, primitives, and stroke/fill properties.
//!
//! Wraps the [`ThorVG` C API](https://www.thorvg.org/c-native).

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
/// Drawn at the ends of open stroked sub-paths. The engine default is
/// [`Square`](Self::Square).
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
/// Applied where two stroked segments meet. The engine default is
/// [`Bevel`](Self::Bevel).
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

/// Rendering order of a shape's fill and stroke.
///
/// Models the `strokeFirst` boolean of the C
/// `tvg_shape_set_paint_order` as a named two-state choice, so call
/// sites read `set_paint_order(PaintOrder::StrokeThenFill)` instead of
/// a bare `true`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PaintOrder {
    /// Fill is painted first, then the stroke on top — the engine
    /// default.
    FillThenStroke,
    /// Stroke is painted first, then the fill on top.
    StrokeThenFill,
}

impl PaintOrder {
    /// Maps to the C `strokeFirst` flag.
    fn to_raw(self) -> bool {
        matches!(self, PaintOrder::StrokeThenFill)
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

// SAFETY: Same rationale as other `ThorVG` handle types — exclusive
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
    /// `raw` must be a valid `Tvg_Paint` of runtime type Shape. When
    /// `owned` is `true` the returned value frees the handle on drop,
    /// so the caller must not also release it.
    pub(crate) unsafe fn from_raw(raw: sys::Tvg_Paint, owned: bool) -> Self {
        Self {
            raw,
            owned,
            _engine: core::marker::PhantomData,
        }
    }

    // ── Path commands ──────────────────────────────────────────────

    /// Resets the shape's path, clearing all sub-paths.
    ///
    /// Color, fill, and stroke properties are retained. The path-data
    /// storage is kept allocated for reuse rather than freed.
    pub fn reset(&mut self) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_reset(self.raw) })
    }

    /// Begins a new sub-path at `(x, y)`, setting the current point.
    pub fn move_to(&mut self, x: f32, y: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_move_to(self.raw, x, y) })
    }

    /// Draws a line from the current point to `(x, y)`.
    ///
    /// If this is the first command in the path it behaves like
    /// [`move_to`](Self::move_to). The current point becomes `(x, y)`.
    pub fn line_to(&mut self, x: f32, y: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_line_to(self.raw, x, y) })
    }

    /// Draws a cubic Bézier curve from the current point to `(x, y)`.
    ///
    /// `(cx1, cy1)` and `(cx2, cy2)` are the first and second control
    /// points. The current point becomes `(x, y)`.
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

    /// Closes the current sub-path with a line back to its start point.
    ///
    /// The current point is reset to that start point. Has no effect if
    /// the sub-path contains no points.
    pub fn close(&mut self) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_close(self.raw) })
    }

    /// Appends a sub-path described by a typed [`Path`].
    ///
    /// Closes the round-trip with [`path`](Self::path): a value
    /// returned by that getter can be re-applied here verbatim.
    /// Commands are translated through `PathCommand::to_raw` and
    /// points are repacked as `Tvg_Point` for the C call.  Both
    /// translations allocate a temporary `Vec`; the original
    /// `Path` is left intact.
    ///
    /// `ThorVG`'s C-side path builder reads `points` lock-step with
    /// `commands` according to [`PathCommand::points_consumed`]. If the
    /// `points` vector does not supply exactly as many points as the
    /// commands require, the sub-path is appended but the shape does
    /// not render — this is not reported as an error. Constructing such
    /// a [`Path`] is permitted on the Rust side.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if either `commands` or
    /// `points` is empty.
    #[allow(clippy::cast_possible_truncation)]
    pub fn append_path(&mut self, path: &Path) -> Result<()> {
        let raw_cmds: alloc::vec::Vec<sys::Tvg_Path_Command> =
            path.commands.iter().map(|c| c.to_raw()).collect();
        let raw_pts: alloc::vec::Vec<sys::Tvg_Point> = path
            .points
            .iter()
            .map(|p| sys::Tvg_Point { x: p.x, y: p.y })
            .collect();
        Error::from_raw(unsafe {
            sys::tvg_shape_append_path(
                self.raw,
                raw_cmds.as_ptr(),
                raw_cmds.len() as u32,
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
    /// Starts a new sub-path; it is not connected to the previous one.
    /// See [`Rect`] for the parameter layout. When `rx` and `ry` each
    /// reach or exceed half the width and half the height
    /// respectively, the rounded rectangle degenerates into an
    /// ellipse.
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

    /// Appends an ellipse to the path.
    ///
    /// Starts a new sub-path; it is not connected to the previous one.
    /// See [`Circle`] for the parameter layout — [`Circle::new`] for
    /// true circles, [`Circle::ellipse`] for elliptical shapes.
    pub fn append_circle(&mut self, circle: Circle) -> Result<()> {
        let Circle { cx, cy, rx, ry, cw } = circle;
        Error::from_raw(unsafe { sys::tvg_shape_append_circle(self.raw, cx, cy, rx, ry, cw) })
    }

    // ── Fill ───────────────────────────────────────────────────────

    /// Sets the shape's solid fill color.
    ///
    /// Each channel is in the range `0..=255`; `a` of `0` is fully
    /// transparent and `255` fully opaque. A shape carries either a
    /// solid fill or a gradient fill — whichever was set last wins, so
    /// this overrides any gradient set via
    /// [`set_linear_gradient`](Self::set_linear_gradient) /
    /// [`set_radial_gradient`](Self::set_radial_gradient).
    pub fn set_fill_color(&mut self, color: Rgba) -> Result<()> {
        let Rgba { r, g, b, a } = color;
        Error::from_raw(unsafe { sys::tvg_shape_set_fill_color(self.raw, r, g, b, a) })
    }

    /// Returns the shape's solid fill color.
    ///
    /// Defaults to fully transparent black (`0, 0, 0, 0`) on a freshly
    /// created shape.
    pub fn fill_color(&self) -> Result<Rgba> {
        let (mut r, mut g, mut b, mut a) = (0u8, 0u8, 0u8, 0u8);
        Error::from_raw(unsafe {
            sys::tvg_shape_get_fill_color(self.raw, &raw mut r, &raw mut g, &raw mut b, &raw mut a)
        })?;
        Ok(Rgba { r, g, b, a })
    }

    /// Sets the fill rule.
    ///
    /// Determines how the interior is computed when the path
    /// self-intersects. The engine default is [`FillRule::NonZero`].
    pub fn set_fill_rule(&mut self, rule: FillRule) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_fill_rule(self.raw, rule.to_raw()) })
    }

    /// Returns the current fill rule.
    pub fn fill_rule(&self) -> Result<FillRule> {
        let mut rule = sys::Tvg_Fill_Rule::TVG_FILL_RULE_NON_ZERO;
        Error::from_raw(unsafe { sys::tvg_shape_get_fill_rule(self.raw, &raw mut rule) })?;
        Ok(FillRule::from_raw(rule))
    }

    /// Sets a linear gradient fill, taking ownership of `grad`.
    ///
    /// Overrides any solid color set via
    /// [`set_fill_color`](Self::set_fill_color): a shape carries either
    /// a solid fill or a gradient fill, whichever was set last.
    pub fn set_linear_gradient(&mut self, grad: LinearGradient<'_>) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_gradient(self.raw, grad.into_raw()) })
    }

    /// Sets a radial gradient fill, taking ownership of `grad`.
    ///
    /// Overrides any solid color set via
    /// [`set_fill_color`](Self::set_fill_color), as with
    /// [`set_linear_gradient`](Self::set_linear_gradient).
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

    /// Sets the rendering order of fill and stroke.  See [`PaintOrder`].
    pub fn set_paint_order(&mut self, order: PaintOrder) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_paint_order(self.raw, order.to_raw()) })
    }

    // ── Stroke ─────────────────────────────────────────────────────

    /// Sets the stroke width in pixels.
    ///
    /// A width of `0.0` (the engine default) disables the stroke.
    pub fn set_stroke_width(&mut self, width: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_width(self.raw, width) })
    }

    /// Returns the stroke width in pixels.
    pub fn stroke_width(&self) -> Result<f32> {
        let mut width: f32 = 0.0;
        Error::from_raw(unsafe { sys::tvg_shape_get_stroke_width(self.raw, &raw mut width) })?;
        Ok(width)
    }

    /// Sets the stroke's solid color.
    ///
    /// Each channel is in the range `0..=255`. The stroke is invisible
    /// while the stroke width is `0.0` (the default), regardless of
    /// color. As with the fill, the stroke carries either a solid
    /// color or a gradient ([`set_stroke_linear_gradient`](Self::set_stroke_linear_gradient) /
    /// [`set_stroke_radial_gradient`](Self::set_stroke_radial_gradient)) —
    /// whichever was set last.
    pub fn set_stroke_color(&mut self, color: Rgba) -> Result<()> {
        let Rgba { r, g, b, a } = color;
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_color(self.raw, r, g, b, a) })
    }

    /// Returns the stroke's solid color.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InsufficientCondition`] if no stroke has been
    /// set on the shape.
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
    ///
    /// The engine default is [`StrokeCap::Square`].
    pub fn set_stroke_cap(&mut self, cap: StrokeCap) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_cap(self.raw, cap.to_raw()) })
    }

    /// Returns the stroke line cap style.
    pub fn stroke_cap(&self) -> Result<StrokeCap> {
        let mut cap = sys::Tvg_Stroke_Cap::TVG_STROKE_CAP_BUTT;
        Error::from_raw(unsafe { sys::tvg_shape_get_stroke_cap(self.raw, &raw mut cap) })?;
        Ok(StrokeCap::from_raw(cap))
    }

    /// Sets the stroke line join style.
    ///
    /// The engine default is [`StrokeJoin::Bevel`].
    pub fn set_stroke_join(&mut self, join: StrokeJoin) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_join(self.raw, join.to_raw()) })
    }

    /// Returns the stroke line join style.
    pub fn stroke_join(&self) -> Result<StrokeJoin> {
        let mut join = sys::Tvg_Stroke_Join::TVG_STROKE_JOIN_MITER;
        Error::from_raw(unsafe { sys::tvg_shape_get_stroke_join(self.raw, &raw mut join) })?;
        Ok(StrokeJoin::from_raw(join))
    }

    /// Sets the stroke miter limit.
    ///
    /// Caps how far a [`StrokeJoin::Miter`] join may extend before it
    /// is converted to a bevel; ignored for other join styles. The
    /// engine default is `4.0`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if `miterlimit` is negative.
    pub fn set_stroke_miterlimit(&mut self, miterlimit: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_miterlimit(self.raw, miterlimit) })
    }

    /// Returns the stroke miter limit.
    pub fn stroke_miterlimit(&self) -> Result<f32> {
        let mut ml: f32 = 0.0;
        Error::from_raw(unsafe { sys::tvg_shape_get_stroke_miterlimit(self.raw, &raw mut ml) })?;
        Ok(ml)
    }

    /// Sets the stroke dash pattern.
    ///
    /// `pattern` holds alternating dash and gap lengths; `offset`
    /// shifts the starting point within the repeating pattern. Negative
    /// entries are treated as `0.0`, and if every entry is `<= 0.0` the
    /// dash is ignored. An odd-length `pattern` is repeated once so
    /// dashes and gaps stay alternating.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] when `pattern` is empty: the
    /// engine rejects a non-`null` pointer paired with a count of zero,
    /// and a Rust empty slice yields exactly that. To clear an existing
    /// dash pattern, set a single-element pattern of `0.0` (which the
    /// engine then ignores) rather than passing an empty slice.
    #[allow(clippy::cast_possible_truncation)]
    pub fn set_stroke_dash(&mut self, pattern: &[f32], offset: f32) -> Result<()> {
        Error::from_raw(unsafe {
            sys::tvg_shape_set_stroke_dash(self.raw, pattern.as_ptr(), pattern.len() as u32, offset)
        })
    }

    /// Returns the stroke dash pattern and offset.
    ///
    /// The pattern is empty when no dashing is set.
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
    /// on the fill side. The underlying C call
    /// (`tvg_shape_set_stroke_gradient`) takes a polymorphic
    /// `Tvg_Gradient`, so both gradient kinds are accepted as a stroke
    /// fill.
    pub fn set_stroke_radial_gradient(&mut self, grad: RadialGradient<'_>) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_shape_set_stroke_gradient(self.raw, grad.into_raw()) })
    }

    /// Sets the visible segment of the path via trimming.
    ///
    /// `begin` and `end` are normalised positions along the path,
    /// where `0.0` is the start and `1.0` the end. Values outside
    /// `[0.0, 1.0]` wrap around circularly (like angle wrapping)
    /// rather than being clamped.
    ///
    /// When a shape has multiple sub-paths, `simultaneous` controls
    /// how they are trimmed. `true` (the engine default) applies the
    /// trim to every sub-path independently; `false` treats them as a
    /// single concatenated path whose total length is the sum of the
    /// individual lengths.
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
