//! Vector path data — the typed counterpart of thorvg's
//! `Tvg_Path_Command` / `Tvg_Point` buffer pair.
//!
//! Returned from [`Shape::path`](crate::Shape::path) and intended
//! as a single value type that bundles commands and the points
//! they consume.  Walking the data is done through
//! [`Path::segments`], which yields typed [`Segment`]s and removes
//! the need for callers to track a separate points cursor.
//!
//! Wraps the [`ThorVG` C API](https://www.thorvg.org/c-native).

use alloc::vec::Vec;

use crate::paint::Point;
use thorvg_sys as sys;

/// One of thorvg's four path command kinds.
///
/// Maps directly to the values encoded behind `Tvg_Path_Command`
/// (a `uint8_t` typedef in the C header).  Each variant consumes
/// a fixed number of points from the parallel point buffer:
///
/// | Variant   | Points consumed | SVG equivalent |
/// |-----------|-----------------|----------------|
/// | `Close`   | 0               | `Z`            |
/// | `MoveTo`  | 1               | `M`            |
/// | `LineTo`  | 1               | `L`            |
/// | `CubicTo` | 3               | `C`            |
///
/// Exhaustive: the C header has shipped these four kinds since
/// the enum's introduction and has not added new ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PathCommand {
    /// Close the current sub-path back to its start point.
    Close,
    /// Begin a new sub-path at the next point.
    MoveTo,
    /// Straight line to the next point.
    LineTo,
    /// Cubic Bézier curve through the next two control points
    /// to the third point.
    CubicTo,
}

impl PathCommand {
    /// Number of points this command consumes from a paired
    /// point buffer.
    #[must_use]
    pub const fn points_consumed(self) -> usize {
        match self {
            Self::Close => 0,
            Self::MoveTo | Self::LineTo => 1,
            Self::CubicTo => 3,
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    pub(crate) fn to_raw(self) -> sys::Tvg_Path_Command {
        // `Tvg_Path_Command` is `u8`; the bindgen constants are
        // `c_uint`.  All four values fit in a byte, so the
        // narrowing cast is well-defined.
        match self {
            Self::Close => sys::TVG_PATH_COMMAND_CLOSE as sys::Tvg_Path_Command,
            Self::MoveTo => sys::TVG_PATH_COMMAND_MOVE_TO as sys::Tvg_Path_Command,
            Self::LineTo => sys::TVG_PATH_COMMAND_LINE_TO as sys::Tvg_Path_Command,
            Self::CubicTo => sys::TVG_PATH_COMMAND_CUBIC_TO as sys::Tvg_Path_Command,
        }
    }

    /// Decodes a raw command byte coming back from the C side.
    ///
    /// Returns `None` for any value outside the documented set so
    /// callers (currently only [`Path`]'s decoder) can decide how
    /// to react to an unrecognised command rather than silently
    /// coercing it to a default kind.
    // The constant `as u32` casts are no-ops where bindgen types these as
    // `c_uint`, but real `c_int` -> `u32` conversions on MSVC. clippy only
    // sees the current target, so silence its same-type-cast lint here.
    #[allow(clippy::unnecessary_cast)]
    pub(crate) fn from_raw(c: sys::Tvg_Path_Command) -> Option<Self> {
        // bindgen types these constants as `c_uint` on some targets and
        // `c_int` on others (MSVC). Normalise them to `u32` consts so the
        // match patterns are stable regardless of the generated repr.
        const CLOSE: u32 = sys::TVG_PATH_COMMAND_CLOSE as u32;
        const MOVE_TO: u32 = sys::TVG_PATH_COMMAND_MOVE_TO as u32;
        const LINE_TO: u32 = sys::TVG_PATH_COMMAND_LINE_TO as u32;
        const CUBIC_TO: u32 = sys::TVG_PATH_COMMAND_CUBIC_TO as u32;
        match u32::from(c) {
            CLOSE => Some(Self::Close),
            MOVE_TO => Some(Self::MoveTo),
            LINE_TO => Some(Self::LineTo),
            CUBIC_TO => Some(Self::CubicTo),
            _ => None,
        }
    }
}

/// Vector path data — a sequence of commands paired with the
/// points they consume.
///
/// Returned by [`Shape::path`](crate::Shape::path).  The
/// `commands` and `points` vectors are independent storage, but
/// semantically they walk in lockstep: each entry in `commands`
/// consumes [`PathCommand::points_consumed`] entries from
/// `points`.  Use [`segments`](Self::segments) to walk both at
/// once and receive typed [`Segment`]s instead of managing the
/// points cursor by hand.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path {
    /// Sequence of path operations.
    pub commands: Vec<PathCommand>,
    /// Supporting points, consumed in declaration order by the
    /// `commands` vector.
    pub points: Vec<Point>,
}

impl Path {
    /// Empty path.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of commands in the path.
    #[must_use]
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Number of points in the path.
    #[must_use]
    pub fn point_count(&self) -> usize {
        self.points.len()
    }

    /// Returns `true` if the path holds no commands.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Iterator over typed [`Segment`]s.  Each segment bundles a
    /// command with the points it consumes, so callers don't need
    /// to track a separate cursor into `points`.
    #[must_use]
    pub fn segments(&self) -> Segments<'_> {
        Segments {
            commands: self.commands.iter(),
            points: self.points.iter(),
        }
    }
}

/// One step in a path traversal.
///
/// Yielded by [`Path::segments`].  Variants own copies of the
/// supporting [`Point`]s (which are `Copy`) so segments can be
/// passed around by value without lifetime entanglement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    /// Close the current sub-path (no points consumed).
    Close,
    /// Start a new sub-path at the given point.
    MoveTo(Point),
    /// Straight line from the current point to `Point`.
    LineTo(Point),
    /// Cubic Bézier curve through the two control points `c1`
    /// and `c2` to the end point `end`.
    CubicTo {
        /// First control point.
        c1: Point,
        /// Second control point.
        c2: Point,
        /// Curve end point (also the new current point).
        end: Point,
    },
}

/// Iterator over typed [`Segment`]s produced by [`Path::segments`].
pub struct Segments<'a> {
    commands: core::slice::Iter<'a, PathCommand>,
    points: core::slice::Iter<'a, Point>,
}

impl Iterator for Segments<'_> {
    type Item = Segment;

    fn next(&mut self) -> Option<Self::Item> {
        // If the points buffer runs out before commands do (only
        // possible for a malformed path from the C side), stop
        // emitting rather than panic — the path is already
        // damaged at that point and the caller has the surviving
        // segments.
        Some(match *self.commands.next()? {
            PathCommand::Close => Segment::Close,
            PathCommand::MoveTo => Segment::MoveTo(*self.points.next()?),
            PathCommand::LineTo => Segment::LineTo(*self.points.next()?),
            PathCommand::CubicTo => Segment::CubicTo {
                c1: *self.points.next()?,
                c2: *self.points.next()?,
                end: *self.points.next()?,
            },
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // Each remaining command yields exactly one segment, so the
        // remaining command count is an upper bound.  The lower bound
        // stays 0: a malformed path (points exhausted before commands)
        // stops early, so the count is not exact.
        (0, Some(self.commands.len()))
    }
}

impl core::iter::FusedIterator for Segments<'_> {}

impl core::fmt::Debug for Segments<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Segments").finish_non_exhaustive()
    }
}
