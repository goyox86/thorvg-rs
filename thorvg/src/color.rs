//! 8-bit color primitives shared across the public API.
//!
//! These are general-purpose value types used by every part of the
//! crate that names a color: `Shape` fill / stroke, `Scene` effect
//! parameters ([`DropShadow`](crate::DropShadow),
//! [`Tint`](crate::Tint), [`Tritone`](crate::Tritone)), and any
//! future color-bearing API (text, gradients, …).
//!
//! Both types are plain `Copy` PODs with `pub` fields, so all of
//! struct literal, `..Default::default()`, and `Self::new(...)`
//! construction work.

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
