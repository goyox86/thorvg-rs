//! Color types shared across the public API.
//!
//! [`Rgb`] and [`Rgba`] are general-purpose 8-bit value types used by
//! every part of the crate that names a color: `Shape` fill / stroke,
//! `Scene` effect parameters ([`DropShadow`](crate::DropShadow),
//! [`Tint`](crate::Tint), [`Tritone`](crate::Tritone)), and any
//! future color-bearing API (text, gradients, …).  Both are plain
//! `Copy` PODs with `pub` fields, so struct literal,
//! `..Default::default()`, and `Self::new(...)` construction all work.
//!
//! [`ColorSpace`] describes the pixel layout of a rendering buffer and
//! is named by the canvas `set_target` APIs.

use thorvg_sys as sys;

/// 8-bit RGB color.  Field set is closed; literal construction
/// (`Rgb { r, g, b }`) is supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgb {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
}

impl Rgb {
    /// Builds an [`Rgb`] from its three channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// 8-bit RGB color with an alpha channel.  Field set is closed;
/// literal construction (`Rgba { r, g, b, a }`) is supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgba {
    /// Red channel.
    pub r: u8,
    /// Green channel.
    pub g: u8,
    /// Blue channel.
    pub b: u8,
    /// Alpha (opacity) channel.
    pub a: u8,
}

impl Rgba {
    /// Builds an [`Rgba`] from its four channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// Color space for the rendering buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ColorSpace {
    /// Alpha, Blue, Green, Red (premultiplied alpha).
    ABGR8888,
    /// Alpha, Red, Green, Blue (premultiplied alpha).
    ARGB8888,
    /// Alpha, Blue, Green, Red (straight alpha).
    ABGR8888S,
    /// Alpha, Red, Green, Blue (straight alpha).
    ARGB8888S,
}

impl ColorSpace {
    pub(crate) fn to_raw(self) -> sys::Tvg_Colorspace {
        match self {
            ColorSpace::ABGR8888 => sys::Tvg_Colorspace::TVG_COLORSPACE_ABGR8888,
            ColorSpace::ARGB8888 => sys::Tvg_Colorspace::TVG_COLORSPACE_ARGB8888,
            ColorSpace::ABGR8888S => sys::Tvg_Colorspace::TVG_COLORSPACE_ABGR8888S,
            ColorSpace::ARGB8888S => sys::Tvg_Colorspace::TVG_COLORSPACE_ARGB8888S,
        }
    }
}
