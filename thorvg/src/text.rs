//! Unicode text rendering: fonts, layout, and glyph metrics.
//!
//! Wraps the [`ThorVG` C API](https://www.thorvg.org/c-native).

use alloc::ffi::CString;
use alloc::string::String;

use crate::color::Rgb;
use crate::error::{Error, Result};
use crate::gradient::{LinearGradient, RadialGradient};
use crate::paint::{Paint, Point};
use thorvg_sys as sys;

/// Text wrapping mode for a [`Text`] object.
///
/// Controls how text that exceeds the layout box (set via
/// [`Text::set_layout`]) is broken across lines. Maps to the C
/// `Tvg_Text_Wrap` enum. The default is [`None`](Self::None).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextWrap {
    /// No wrapping; text overflows the layout box.
    None,
    /// Breaks at any character that exceeds the layout width.
    Character,
    /// Breaks at word boundaries (whitespace).
    Word,
    /// Word-aware wrapping that falls back to character breaks for
    /// words too long to fit a line.
    Smart,
    /// Truncates overflowing text and appends an ellipsis (`…`).
    Ellipsis,
}

impl TextWrap {
    fn to_raw(self) -> sys::Tvg_Text_Wrap {
        match self {
            TextWrap::None => sys::Tvg_Text_Wrap::TVG_TEXT_WRAP_NONE,
            TextWrap::Character => sys::Tvg_Text_Wrap::TVG_TEXT_WRAP_CHARACTER,
            TextWrap::Word => sys::Tvg_Text_Wrap::TVG_TEXT_WRAP_WORD,
            TextWrap::Smart => sys::Tvg_Text_Wrap::TVG_TEXT_WRAP_SMART,
            TextWrap::Ellipsis => sys::Tvg_Text_Wrap::TVG_TEXT_WRAP_ELLIPSIS,
        }
    }
}

/// Vertical font metrics for a [`Text`] object.
///
/// Reflect the font size set on the text object but exclude any
/// transform (scale, rotation, translation). Obtained from
/// [`Text::text_metrics`].
#[derive(Debug, Clone, Copy)]
pub struct TextMetrics {
    /// Distance from the baseline to the top of the highest glyph
    /// (usually positive).
    pub ascent: f32,
    /// Distance from the baseline to the bottom of the lowest glyph
    /// (usually negative, following the TTF convention).
    pub descent: f32,
    /// Additional recommended spacing (leading) between lines.
    pub linegap: f32,
    /// Total vertical advance between lines: `ascent - descent +
    /// linegap`.
    pub advance: f32,
}

/// Layout metrics of a single glyph.
///
/// Reflect the font size set on the text object but exclude any
/// transform. The bounding box is given in the glyph's local
/// coordinate space. Obtained from [`Text::glyph_metrics`].
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    /// Advance of the pen position along the baseline (inline
    /// direction) to the next glyph's origin.
    pub advance: f32,
    /// Left-side bearing — offset from the inline-axis origin to the
    /// glyph's edge.
    pub bearing: f32,
    /// Minimum (lower-left) corner of the glyph's bounding box.
    pub min: Point,
    /// Maximum (upper-right) corner of the glyph's bounding box.
    pub max: Point,
}

/// A paint object for rendering Unicode text.
///
/// A renderable text object needs at least a font family
/// ([`set_font`](Self::set_font)), a size ([`set_size`](Self::set_size)),
/// and content ([`set_text`](Self::set_text)). Fonts themselves are
/// loaded into the engine globally; see
/// [`Thorvg::load_font_data`](crate::Thorvg::load_font_data).
///
/// The lifetime `'eng` ties this text object to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create text objects via [`Thorvg::text()`](crate::Thorvg::text).
pub struct Text<'eng> {
    raw: sys::Tvg_Paint,
    owned: bool,
    _engine: core::marker::PhantomData<&'eng ()>,
}

// SAFETY: Same rationale as other ThorVG handle types — exclusive
// ownership of a C heap object; global state is mutex-protected.
unsafe impl Send for Text<'_> {}

impl Text<'_> {
    /// Creates a new Text object.
    pub(crate) fn new() -> Result<Self> {
        let raw = unsafe { sys::tvg_text_new() };
        if raw.is_null() {
            return Err(Error::FailedAllocation);
        }
        Ok(Self {
            raw,
            owned: true,
            _engine: core::marker::PhantomData,
        })
    }

    /// Sets the font family used to render the text.
    ///
    /// `name` must match a font registered with the engine via
    /// [`Thorvg::load_font_data`](crate::Thorvg::load_font_data). This
    /// only selects the family; use [`set_size`](Self::set_size) for the
    /// size.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if `name` contains an
    /// interior NUL byte, or [`Error::InsufficientCondition`] if the
    /// named font cannot be found.
    pub fn set_font(&mut self, name: &str) -> Result<()> {
        let c_name = CString::new(name)?;
        Error::from_raw(unsafe { sys::tvg_text_set_font(self.raw, c_name.as_ptr()) })
    }

    /// Sets the font size in points.
    ///
    /// Fractional sizes are supported for sub-pixel rendering and
    /// animation.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if `size <= 0.0`.
    pub fn set_size(&mut self, size: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_set_size(self.raw, size) })
    }

    /// Sets the UTF-8 text content to be rendered.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if `text` contains an
    /// interior NUL byte.
    pub fn set_text(&mut self, text: &str) -> Result<()> {
        let c_text = CString::new(text)?;
        Error::from_raw(unsafe { sys::tvg_text_set_text(self.raw, c_text.as_ptr()) })
    }

    /// Returns the currently assigned text (UTF-8), or `None` if none
    /// has been set.
    ///
    /// The returned [`String`] is a copy, so it is independent of the
    /// text object's later mutations.
    ///
    /// *Experimental in `ThorVG`; the API may change.*
    pub fn text(&self) -> Option<String> {
        let ptr = unsafe { sys::tvg_text_get_text(self.raw) };
        if ptr.is_null() {
            None
        } else {
            Some(
                unsafe { core::ffi::CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }

    /// Sets the solid fill color of the text, with channels in
    /// `0..=255`.
    ///
    /// Text color is RGB only (no alpha), so this takes [`Rgb`] rather
    /// than [`Rgba`](crate::Rgba); use [`Paint::set_opacity`] for
    /// translucency. A solid color and a gradient fill are mutually
    /// exclusive — whichever was set last applies.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if the engine rejects the
    /// request.
    pub fn set_color(&mut self, color: Rgb) -> Result<()> {
        let Rgb { r, g, b } = color;
        Error::from_raw(unsafe { sys::tvg_text_set_color(self.raw, r, g, b) })
    }

    /// Sets the per-axis text alignment or anchor.
    ///
    /// Each value is in `0.0..=1.0`: `0.0` is left/top, `0.5` is
    /// center/middle, `1.0` is right/bottom. On an axis constrained by
    /// [`set_layout`](Self::set_layout) this aligns within the layout
    /// box; on an unconstrained axis it anchors the text bounds to the
    /// paint position.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if the engine rejects the
    /// request.
    pub fn set_align(&mut self, x: f32, y: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_align(self.raw, x, y) })
    }

    /// Sets the virtual layout box (constraints) for the text.
    ///
    /// A non-zero `w` or `h` constrains that axis, so the text may
    /// wrap and align inside it; `0.0` leaves the axis unconstrained,
    /// and [`set_align`](Self::set_align) then anchors on that axis.
    /// This sets constraints only; alignment is controlled by
    /// [`set_align`](Self::set_align).
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if the engine rejects the
    /// request.
    pub fn set_layout(&mut self, w: f32, h: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_layout(self.raw, w, h) })
    }

    /// Applies an italic (oblique) slant by shearing along the X-axis.
    ///
    /// `shear` is in `0.0..=0.5` (`0.0` = upright); values outside the
    /// range are clamped by the engine. The recommended value is
    /// `0.18`. This simulates italics with a transform and does not
    /// require an italic font.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if the engine rejects the
    /// request.
    pub fn set_italic(&mut self, shear: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_set_italic(self.raw, shear) })
    }

    /// Sets the text wrapping mode.
    ///
    /// Wrapping applies within the layout box set by
    /// [`set_layout`](Self::set_layout).
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if the engine rejects the
    /// request.
    pub fn set_wrap(&mut self, mode: TextWrap) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_wrap_mode(self.raw, mode.to_raw()) })
    }

    /// Returns the number of lines after layout and wrapping.
    ///
    /// Reflects the current [`set_wrap`](Self::set_wrap) configuration
    /// and is also increased by explicit line-feed (`\n`) characters in
    /// the text.
    ///
    /// *Experimental in `ThorVG`; the API may change.*
    pub fn line_count(&self) -> u32 {
        unsafe { sys::tvg_text_line_count(self.raw) }
    }

    /// Sets the letter and line spacing scale factors.
    ///
    /// Both are relative to the font's default metrics: `letter` scales
    /// the per-glyph advance width and `line` scales the line advance
    /// height. The default is `1.0`; values `> 1.0` increase spacing,
    /// `< 1.0` decrease it, and both must be `>= 0.0`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if the engine rejects the
    /// request.
    pub fn set_spacing(&mut self, letter: f32, line: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_spacing(self.raw, letter, line) })
    }

    /// Sets an outline (stroke) of the given `width` around the text.
    ///
    /// The outline color is RGB only (no alpha), with channels in
    /// `0..=255`, matching [`set_color`](Self::set_color). A `width` of
    /// `0.0` disables the outline.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if the engine rejects the
    /// request.
    pub fn set_outline(&mut self, width: f32, color: Rgb) -> Result<()> {
        let Rgb { r, g, b } = color;
        Error::from_raw(unsafe { sys::tvg_text_set_outline(self.raw, width, r, g, b) })
    }

    /// Sets a linear gradient fill for the text.
    ///
    /// Replaces any solid color set with [`set_color`](Self::set_color)
    /// — whichever was set last applies.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MemoryCorruption`] if the gradient object is
    /// invalid.
    pub fn set_linear_gradient(&mut self, grad: LinearGradient<'_>) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_set_gradient(self.raw, grad.into_raw()) })
    }

    /// Sets a radial gradient fill for the text.
    ///
    /// Replaces any solid color set with [`set_color`](Self::set_color)
    /// — whichever was set last applies.
    ///
    /// # Errors
    ///
    /// Returns [`Error::MemoryCorruption`] if the gradient object is
    /// invalid.
    pub fn set_radial_gradient(&mut self, grad: RadialGradient<'_>) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_set_gradient(self.raw, grad.into_raw()) })
    }

    /// Returns the vertical font metrics of the text object.
    ///
    /// *Experimental in `ThorVG`; the API may change.*
    ///
    /// # Errors
    ///
    /// Returns [`Error::InsufficientCondition`] if no font or size has
    /// been set yet.
    pub fn text_metrics(&self) -> Result<TextMetrics> {
        let mut m = sys::Tvg_Text_Metrics {
            ascent: 0.0,
            descent: 0.0,
            linegap: 0.0,
            advance: 0.0,
        };
        Error::from_raw(unsafe { sys::tvg_text_get_text_metrics(self.raw, &raw mut m) })?;
        Ok(TextMetrics {
            ascent: m.ascent,
            descent: m.descent,
            linegap: m.linegap,
            advance: m.advance,
        })
    }

    /// Returns the layout metrics of a single glyph (horizontal layout
    /// only).
    ///
    /// `ch` must be a single UTF-8 encoded character.
    ///
    /// *Experimental in `ThorVG`; the API may change.*
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if `ch` contains an interior
    /// NUL byte or the engine reports the character as invalid or
    /// unsupported, or [`Error::InsufficientCondition`] if no font or
    /// size has been set yet.
    pub fn glyph_metrics(&self, ch: &str) -> Result<GlyphMetrics> {
        let c_ch = CString::new(ch)?;
        let mut m = sys::Tvg_Glyph_Metrics {
            advance: 0.0,
            bearing: 0.0,
            min: sys::Tvg_Point { x: 0.0, y: 0.0 },
            max: sys::Tvg_Point { x: 0.0, y: 0.0 },
        };
        Error::from_raw(unsafe {
            sys::tvg_text_get_glyph_metrics(self.raw, c_ch.as_ptr(), &raw mut m)
        })?;
        Ok(GlyphMetrics {
            advance: m.advance,
            bearing: m.bearing,
            min: Point {
                x: m.min.x,
                y: m.min.y,
            },
            max: Point {
                x: m.max.x,
                y: m.max.y,
            },
        })
    }

    // Font loading is engine-global state — see [`Thorvg::load_font_data`].
}

impl crate::paint::sealed::Sealed for Text<'_> {}

impl Paint for Text<'_> {
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

impl Drop for Text<'_> {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                sys::tvg_paint_rel(self.raw);
            }
        }
    }
}

impl core::fmt::Debug for Text<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Text").finish_non_exhaustive()
    }
}
