use alloc::ffi::CString;

use crate::error::{Error, Result};
use crate::paint::Paint;
use thorvg_sys as ffi;

/// Image filtering method used during scaling or transformation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterMethod {
    /// Smooth interpolation using surrounding pixels.
    Bilinear,
    /// Fast filtering using nearest-neighbor sampling.
    Nearest,
}

impl FilterMethod {
    fn to_raw(self) -> ffi::Tvg_Filter_Method {
        match self {
            FilterMethod::Bilinear => ffi::Tvg_Filter_Method::TVG_FILTER_METHOD_BILINEAR,
            FilterMethod::Nearest => ffi::Tvg_Filter_Method::TVG_FILTER_METHOD_NEAREST,
        }
    }
}

/// A picture object for loading and displaying images (SVG, PNG, JPG, Lottie, etc.).
pub struct Picture {
    raw: ffi::Tvg_Paint,
    owned: bool,
}

impl Picture {
    /// Creates a new Picture object.
    pub fn new() -> Self {
        let raw = unsafe { ffi::tvg_picture_new() };
        assert!(!raw.is_null(), "failed to create Picture");
        Self { raw, owned: true }
    }

    /// Wraps an existing raw paint pointer.
    ///
    /// # Safety
    /// The pointer must be a valid `Tvg_Paint` of type Picture.
    pub(crate) unsafe fn from_raw(raw: ffi::Tvg_Paint, owned: bool) -> Self {
        Self { raw, owned }
    }

    /// Loads a picture from a file path string.
    pub fn load_from_str(&mut self, path: &str) -> Result<()> {
        let c_path = CString::new(path).map_err(|_| Error::InvalidArguments)?;
        Error::from_raw(unsafe { ffi::tvg_picture_load(self.raw, c_path.as_ptr()) })
    }

    /// Loads a picture from a file path.
    #[cfg(feature = "std")]
    pub fn load<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
        self.load_from_str(&path.as_ref().to_string_lossy())
    }

    /// Loads a picture from memory.
    #[allow(clippy::cast_possible_truncation)]
    pub fn load_data(
        &mut self, data: &[u8], mimetype: &str, resource_path: Option<&str>, copy: bool,
    ) -> Result<()> {
        let c_mime = CString::new(mimetype).map_err(|_| Error::InvalidArguments)?;
        let c_rpath = resource_path
            .map(|p| CString::new(p).map_err(|_| Error::InvalidArguments))
            .transpose()?;
        let rpath_ptr = c_rpath.as_ref().map_or(core::ptr::null(), |c| c.as_ptr());
        Error::from_raw(unsafe {
            ffi::tvg_picture_load_data(
                self.raw,
                data.as_ptr().cast::<core::ffi::c_char>(),
                data.len() as u32,
                c_mime.as_ptr(),
                rpath_ptr,
                copy,
            )
        })
    }

    /// Loads raw image data (pixel buffer).
    pub fn load_raw(
        &mut self, data: &[u32], w: u32, h: u32,
        cs: crate::ColorSpace, copy: bool,
    ) -> Result<()> {
        Error::from_raw(unsafe {
            ffi::tvg_picture_load_raw(self.raw, data.as_ptr(), w, h, cs.to_raw(), copy)
        })
    }

    /// Resizes the picture content.
    pub fn set_size(&mut self, w: f32, h: f32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_picture_set_size(self.raw, w, h) })
    }

    /// Gets the size of the loaded picture.
    pub fn size(&self) -> Result<(f32, f32)> {
        let (mut w, mut h) = (0.0f32, 0.0f32);
        Error::from_raw(unsafe { ffi::tvg_picture_get_size(self.raw, &raw mut w, &raw mut h) })?;
        Ok((w, h))
    }

    /// Sets the normalized origin point.
    pub fn set_origin(&mut self, x: f32, y: f32) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_picture_set_origin(self.raw, x, y) })
    }

    /// Gets the normalized origin point.
    pub fn origin(&self) -> Result<(f32, f32)> {
        let (mut x, mut y) = (0.0f32, 0.0f32);
        Error::from_raw(unsafe { ffi::tvg_picture_get_origin(self.raw, &raw mut x, &raw mut y) })?;
        Ok((x, y))
    }

    /// Sets the image filtering method.
    pub fn set_filter(&mut self, method: FilterMethod) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_picture_set_filter(self.raw, method.to_raw()) })
    }

    /// Enables or disables accessible mode for efficient ID-based lookup.
    pub fn set_accessible(&mut self, accessible: bool) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_picture_set_accessible(self.raw, accessible) })
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
        resolver: ffi::Tvg_Picture_Asset_Resolver,
        data: *mut core::ffi::c_void,
    ) -> Result<()> {
        Error::from_raw(unsafe {
            ffi::tvg_picture_set_asset_resolver(self.raw, resolver, data)
        })
    }

    /// Retrieves a paint object from the picture scene by its unique ID.
    pub fn get_paint(&self, id: u32) -> Option<ffi::Tvg_Paint> {
        let raw = unsafe { ffi::tvg_picture_get_paint(self.raw, id) };
        if raw.is_null() { None } else { Some(raw) }
    }
}

impl Default for Picture {
    fn default() -> Self { Self::new() }
}

impl Paint for Picture {
    fn raw(&self) -> ffi::Tvg_Paint { self.raw }

    fn into_raw(mut self) -> ffi::Tvg_Paint {
        self.owned = false;
        self.raw
    }

    unsafe fn from_raw_paint(raw: ffi::Tvg_Paint) -> Self {
        Self { raw, owned: true }
    }
}

impl Drop for Picture {
    fn drop(&mut self) {
        if self.owned {
            unsafe { ffi::tvg_paint_rel(self.raw); }
        }
    }
}
