use alloc::ffi::CString;

use crate::error::{Error, Result};
use crate::gradient::{LinearGradient, RadialGradient};
use crate::paint::{Paint, Point};
use thorvg_sys as sys;

/// Text wrapping mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TextWrap {
    None,
    Character,
    Word,
    Smart,
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

/// Font metrics for a text object.
#[derive(Debug, Clone, Copy)]
pub struct TextMetrics {
    pub ascent: f32,
    pub descent: f32,
    pub linegap: f32,
    pub advance: f32,
}

/// Layout metrics of a glyph.
#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    pub advance: f32,
    pub bearing: f32,
    pub min: Point,
    pub max: Point,
}

/// A text object for rendering unicode text.
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
    pub(crate) fn new() -> Self {
        let raw = unsafe { sys::tvg_text_new() };
        assert!(!raw.is_null(), "failed to create Text");
        Self {
            raw,
            owned: true,
            _engine: core::marker::PhantomData,
        }
    }

    /// Sets the font family name.
    pub fn set_font(&mut self, name: &str) -> Result<()> {
        let c_name = CString::new(name).map_err(|_| Error::InvalidArguments)?;
        Error::from_raw(unsafe { sys::tvg_text_set_font(self.raw, c_name.as_ptr()) })
    }

    /// Sets the font size in points.
    pub fn set_size(&mut self, size: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_set_size(self.raw, size) })
    }

    /// Sets the text content (UTF-8).
    pub fn set_text(&mut self, text: &str) -> Result<()> {
        let c_text = CString::new(text).map_err(|_| Error::InvalidArguments)?;
        Error::from_raw(unsafe { sys::tvg_text_set_text(self.raw, c_text.as_ptr()) })
    }

    /// Sets the text fill color.
    pub fn set_color(&mut self, r: u8, g: u8, b: u8) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_set_color(self.raw, r, g, b) })
    }

    /// Sets text alignment / anchor.
    pub fn set_align(&mut self, x: f32, y: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_align(self.raw, x, y) })
    }

    /// Sets the layout constraints (virtual layout box).
    pub fn set_layout(&mut self, w: f32, h: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_layout(self.raw, w, h) })
    }

    /// Sets the italic shear factor (0.0–0.5, recommended: 0.18).
    pub fn set_italic(&mut self, shear: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_set_italic(self.raw, shear) })
    }

    /// Sets the text wrapping mode.
    pub fn set_wrap(&mut self, mode: TextWrap) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_wrap_mode(self.raw, mode.to_raw()) })
    }

    /// Returns the number of text lines after layout and wrapping.
    pub fn line_count(&self) -> u32 {
        unsafe { sys::tvg_text_line_count(self.raw) }
    }

    /// Sets letter and line spacing scale factors.
    pub fn set_spacing(&mut self, letter: f32, line: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_spacing(self.raw, letter, line) })
    }

    /// Sets an outline (stroke) around the text.
    pub fn set_outline(&mut self, width: f32, r: u8, g: u8, b: u8) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_set_outline(self.raw, width, r, g, b) })
    }

    /// Sets a linear gradient fill for the text.
    pub fn set_linear_gradient(&mut self, grad: LinearGradient<'_>) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_set_gradient(self.raw, grad.into_raw()) })
    }

    /// Sets a radial gradient fill for the text.
    pub fn set_radial_gradient(&mut self, grad: RadialGradient<'_>) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_text_set_gradient(self.raw, grad.into_raw()) })
    }

    /// Gets font metrics (ascent, descent, linegap, advance).
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

    /// Gets glyph metrics for a UTF-8 character.
    pub fn glyph_metrics(&self, ch: &str) -> Result<GlyphMetrics> {
        let c_ch = CString::new(ch).map_err(|_| Error::InvalidArguments)?;
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

    // ── Font loading (static methods) ──────────────────────────────

    /// Loads a font from a file path string.
    pub fn load_font_from_str(path: &str) -> Result<()> {
        let c_path = CString::new(path).map_err(|_| Error::InvalidArguments)?;
        Error::from_raw(unsafe { sys::tvg_font_load(c_path.as_ptr()) })
    }

    /// Loads a font from a file path.
    #[cfg(feature = "std")]
    pub fn load_font<P: AsRef<std::path::Path>>(path: P) -> Result<()> {
        Self::load_font_from_str(&path.as_ref().to_string_lossy())
    }

    /// Unloads a previously loaded font by path string.
    pub fn unload_font_from_str(path: &str) -> Result<()> {
        let c_path = CString::new(path).map_err(|_| Error::InvalidArguments)?;
        Error::from_raw(unsafe { sys::tvg_font_unload(c_path.as_ptr()) })
    }

    /// Unloads a previously loaded font.
    #[cfg(feature = "std")]
    pub fn unload_font<P: AsRef<std::path::Path>>(path: P) -> Result<()> {
        Self::unload_font_from_str(&path.as_ref().to_string_lossy())
    }

    /// Loads a font from memory, copying `data` into thorvg's
    /// internal registry.
    ///
    /// The font is registered under `name` and remains usable for
    /// the rest of the engine's lifetime (or until unloaded via
    /// `unload_font_from_str(name)`).  Use this variant when
    /// `data` is owned or has a non-`'static` lifetime.
    ///
    /// For zero-copy registration of `'static` data (e.g.
    /// `include_bytes!(...)`), use
    /// [`load_font_data_static`](Self::load_font_data_static).
    pub fn load_font_data(name: &str, data: &[u8], mimetype: Option<&str>) -> Result<()> {
        Self::load_font_data_inner(name, data, mimetype, /* copy = */ true)
    }

    /// Loads a font from `'static` memory without copying.
    ///
    /// thorvg stores the pointer to `data` in its global font
    /// registry and dereferences it on every subsequent text-render
    /// call, so the buffer must outlive the engine — the `'static`
    /// bound enforces this at compile time.  Typical use:
    /// `Text::load_font_data_static("Roboto", include_bytes!("Roboto.ttf"), None)`.
    ///
    /// For non-`'static` buffers, use
    /// [`load_font_data`](Self::load_font_data) which copies into
    /// thorvg-owned memory.
    ///
    /// # Compile-time safety
    ///
    /// The `'static` bound rejects local buffers at the type level:
    ///
    /// ```compile_fail,E0597
    /// let local: Vec<u8> = vec![0; 32];
    /// thorvg::Text::load_font_data_static("nope", &local, None).unwrap();
    /// // error[E0597]: `local` does not live long enough
    /// ```
    pub fn load_font_data_static(
        name: &str,
        data: &'static [u8],
        mimetype: Option<&str>,
    ) -> Result<()> {
        Self::load_font_data_inner(name, data, mimetype, /* copy = */ false)
    }

    #[allow(clippy::cast_possible_truncation)]
    fn load_font_data_inner(
        name: &str,
        data: &[u8],
        mimetype: Option<&str>,
        copy: bool,
    ) -> Result<()> {
        let c_name = CString::new(name).map_err(|_| Error::InvalidArguments)?;
        let c_mime = mimetype
            .map(|m| CString::new(m).map_err(|_| Error::InvalidArguments))
            .transpose()?;
        let mime_ptr = c_mime.as_ref().map_or(core::ptr::null(), |c| c.as_ptr());
        Error::from_raw(unsafe {
            sys::tvg_font_load_data(
                c_name.as_ptr(),
                data.as_ptr().cast::<core::ffi::c_char>(),
                data.len() as u32,
                mime_ptr,
                copy,
            )
        })
    }
}

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
