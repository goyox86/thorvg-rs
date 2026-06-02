use alloc::ffi::CString;

use crate::error::{Error, Result};
use crate::paint::Paint;
use thorvg_sys as sys;

/// Image filtering method used during scaling or transformation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FilterMethod {
    /// Smooth interpolation using surrounding pixels.
    Bilinear,
    /// Fast filtering using nearest-neighbor sampling.
    Nearest,
}

impl FilterMethod {
    fn to_raw(self) -> sys::Tvg_Filter_Method {
        match self {
            FilterMethod::Bilinear => sys::Tvg_Filter_Method::TVG_FILTER_METHOD_BILINEAR,
            FilterMethod::Nearest => sys::Tvg_Filter_Method::TVG_FILTER_METHOD_NEAREST,
        }
    }
}

/// Picture data format passed to [`Picture::load_data`].
///
/// Maps to the mime strings thorvg's loader manager recognises (see
/// `tvgLoaderMgr.cpp`).  Runtime availability of each loader
/// depends on the `thorvg-sys` features enabled (e.g. `svg`,
/// `png`, `lottie`); selecting a format whose loader isn't
/// compiled returns `Error::NonSupport`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MimeType {
    /// Scalable Vector Graphics (`svg`, `svg+xml`).
    Svg,
    /// PNG bitmap (`png`).
    Png,
    /// JPEG bitmap (`jpg`, `jpeg`).
    Jpg,
    /// WebP bitmap (`webp`).
    Webp,
    /// Lottie animation (`lot`, `lottie+json`).
    Lottie,
    /// Raw pixel buffer (`raw`).
    Raw,
}

impl MimeType {
    fn as_c_str(self) -> &'static core::ffi::CStr {
        match self {
            MimeType::Svg => c"svg",
            MimeType::Png => c"png",
            MimeType::Jpg => c"jpg",
            MimeType::Webp => c"webp",
            MimeType::Lottie => c"lottie+json",
            MimeType::Raw => c"raw",
        }
    }
}

/// A picture object for loading and displaying images (SVG, PNG, JPG, Lottie, etc.).
///
/// The lifetime `'eng` ties this picture to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create pictures via [`Thorvg::picture()`](crate::Thorvg::picture).
///
/// # Thread Safety
///
/// `Picture` is [`Send`] but not [`Sync`].
pub struct Picture<'eng> {
    raw: sys::Tvg_Paint,
    owned: bool,
    _engine: core::marker::PhantomData<&'eng ()>,
}

// SAFETY: `Picture` exclusively owns (or borrows) a heap-allocated ThorVG
// paint handle.  Shared global state is mutex-protected in C++.  Sole
// ownership transfer to another thread is safe.
unsafe impl Send for Picture<'_> {}

impl Picture<'_> {
    /// Creates a new Picture object.
    pub(crate) fn new() -> Self {
        let raw = unsafe { sys::tvg_picture_new() };
        assert!(!raw.is_null(), "failed to create Picture");
        Self {
            raw,
            owned: true,
            _engine: core::marker::PhantomData,
        }
    }

    /// Wraps an existing raw paint pointer.
    ///
    /// # Safety
    /// The pointer must be a valid `Tvg_Paint` of type Picture.
    pub(crate) unsafe fn from_raw(raw: sys::Tvg_Paint, owned: bool) -> Self {
        Self {
            raw,
            owned,
            _engine: core::marker::PhantomData,
        }
    }

    /// Loads a picture from a file path string.
    pub fn load_from_str(&mut self, path: &str) -> Result<()> {
        let c_path = CString::new(path).map_err(|_| Error::InvalidArguments)?;
        Error::from_raw(unsafe { sys::tvg_picture_load(self.raw, c_path.as_ptr()) })
    }

    /// Loads a picture from a file path.
    #[cfg(feature = "std")]
    pub fn load<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
        self.load_from_str(&path.as_ref().to_string_lossy())
    }

    /// Loads a picture from memory.
    ///
    /// `mime` selects the loader (see [`MimeType`]).  `resource_path`
    /// is the base directory for SVG external assets; pass `None`
    /// for self-contained content.  `copy=false` borrows the buffer
    /// — caller must keep it alive for the picture's lifetime.
    #[allow(clippy::cast_possible_truncation)]
    pub fn load_data(
        &mut self,
        data: &[u8],
        mime: MimeType,
        resource_path: Option<&str>,
        copy: bool,
    ) -> Result<()> {
        let c_rpath = resource_path
            .map(|p| CString::new(p).map_err(|_| Error::InvalidArguments))
            .transpose()?;
        let rpath_ptr = c_rpath.as_ref().map_or(core::ptr::null(), |c| c.as_ptr());
        Error::from_raw(unsafe {
            sys::tvg_picture_load_data(
                self.raw,
                data.as_ptr().cast::<core::ffi::c_char>(),
                data.len() as u32,
                mime.as_c_str().as_ptr(),
                rpath_ptr,
                copy,
            )
        })
    }

    /// Loads raw image data (pixel buffer).
    pub fn load_raw(
        &mut self,
        data: &[u32],
        w: u32,
        h: u32,
        cs: crate::ColorSpace,
        copy: bool,
    ) -> Result<()> {
        Error::from_raw(unsafe {
            sys::tvg_picture_load_raw(self.raw, data.as_ptr(), w, h, cs.to_raw(), copy)
        })
    }

    /// Resizes the picture content.
    pub fn set_size(&mut self, w: f32, h: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_picture_set_size(self.raw, w, h) })
    }

    /// Gets the size of the loaded picture.
    pub fn size(&self) -> Result<(f32, f32)> {
        let (mut w, mut h) = (0.0f32, 0.0f32);
        Error::from_raw(unsafe { sys::tvg_picture_get_size(self.raw, &raw mut w, &raw mut h) })?;
        Ok((w, h))
    }

    /// Sets the normalized origin point.
    pub fn set_origin(&mut self, x: f32, y: f32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_picture_set_origin(self.raw, x, y) })
    }

    /// Gets the normalized origin point.
    pub fn origin(&self) -> Result<(f32, f32)> {
        let (mut x, mut y) = (0.0f32, 0.0f32);
        Error::from_raw(unsafe { sys::tvg_picture_get_origin(self.raw, &raw mut x, &raw mut y) })?;
        Ok((x, y))
    }

    /// Sets the image filtering method.
    pub fn set_filter(&mut self, method: FilterMethod) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_picture_set_filter(self.raw, method.to_raw()) })
    }

    /// Enables or disables accessible mode for efficient ID-based lookup.
    pub fn set_accessible(&mut self, accessible: bool) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_picture_set_accessible(self.raw, accessible) })
    }

    /// Sets the asset resolver callback for external resources.
    ///
    /// The `resolver` is called when external assets (images, fonts) need to be loaded.
    /// `data` is a user pointer passed to every callback invocation.
    ///
    /// Must be called **before** [`Picture::load`] / [`Picture::load_from_str`].
    /// Pass `None` to unset.
    ///
    /// # Safety
    /// `data` must remain valid for the lifetime of the picture, and the `resolver`
    /// function pointer must be safe to call from any thread `ThorVG` uses.
    pub unsafe fn set_asset_resolver(
        &mut self,
        resolver: sys::Tvg_Picture_Asset_Resolver,
        data: *mut core::ffi::c_void,
    ) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_picture_set_asset_resolver(self.raw, resolver, data) })
    }

    /// Retrieves a paint object from the picture scene by its unique ID.
    pub fn get_paint(&self, id: u32) -> Option<sys::Tvg_Paint> {
        let raw = unsafe { sys::tvg_picture_get_paint(self.raw, id) };
        if raw.is_null() {
            None
        } else {
            Some(raw)
        }
    }
}

impl Paint for Picture<'_> {
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

impl Drop for Picture<'_> {
    fn drop(&mut self) {
        if self.owned {
            unsafe {
                sys::tvg_paint_rel(self.raw);
            }
        }
    }
}

impl core::fmt::Debug for Picture<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Picture").finish_non_exhaustive()
    }
}
