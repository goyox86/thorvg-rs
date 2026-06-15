//! Color types shared across the public API.
//!
//! [`Rgb`] and [`Rgba`] are general-purpose 8-bit-per-channel value
//! types used by every part of the crate that names a color: shape
//! fill and stroke, scene effect parameters
//! ([`DropShadow`](crate::DropShadow), [`Tint`](crate::Tint),
//! [`Tritone`](crate::Tritone)), and any future color-bearing API.
//! Both are plain `Copy` PODs with `pub` fields, so struct-literal,
//! `..Default::default()`, and `new` construction all work.
//!
//! [`ColorSpace`] describes the pixel layout of a rendering buffer and
//! is named by the canvas `set_target` APIs.
//!
//! Wraps the [`ThorVG` C API](https://www.thorvg.org/c-native).

use thorvg_sys as sys;

/// An 8-bit-per-channel RGB color.
///
/// A plain `Copy` POD with `pub` fields, so struct-literal
/// construction (`Rgb { r, g, b }`), [`Default`], and [`Rgb::new`]
/// all work. Channel values range `0..=255`.
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
    /// Creates an [`Rgb`] from its red, green, and blue channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// An 8-bit-per-channel RGB color with an alpha channel.
///
/// A plain `Copy` POD with `pub` fields, so struct-literal
/// construction (`Rgba { r, g, b, a }`), [`Default`], and
/// [`Rgba::new`] all work. Channel values range `0..=255`, where
/// `a == 0` is fully transparent and `a == 255` is fully opaque.
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
    /// Creates an [`Rgba`] from its red, green, blue, and alpha
    /// channels.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// Pixel layout of a rendering buffer.
///
/// Names the byte order and alpha convention of a canvas target
/// buffer; passed to the canvas `set_target` APIs. Each variant is a
/// 32-bit-per-pixel format. The `S` suffix denotes straight
/// (non-premultiplied) alpha; the unsuffixed variants use
/// premultiplied alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ColorSpace {
    /// Channels in alpha, blue, green, red order; premultiplied alpha.
    ABGR8888,
    /// Channels in alpha, red, green, blue order; premultiplied alpha.
    ARGB8888,
    /// Channels in alpha, blue, green, red order; straight
    /// (non-premultiplied) alpha.
    ABGR8888S,
    /// Channels in alpha, red, green, blue order; straight
    /// (non-premultiplied) alpha.
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
