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
    /// Boxed asset resolver, kept alive for the picture's lifetime.
    /// The outer `Box` is what `Picture` owns; the C side stores the
    /// address of the *inner* `Box` value, which lives on the heap
    /// (the outer `Box`'s allocation).  That address is stable
    /// regardless of where the `Picture` itself lives — so moving the
    /// `Picture` between [`set_asset_resolver`](Self::set_asset_resolver)
    /// and a `load*` call does not invalidate the pointer thorvg holds.
    /// `None` until [`set_asset_resolver`](Self::set_asset_resolver) is called.
    resolver: Option<alloc::boxed::Box<alloc::boxed::Box<AssetResolverFn>>>,
    _engine: core::marker::PhantomData<&'eng ()>,
}

/// Boxed closure type behind the asset resolver registration.
/// `Send + 'static` so `Picture` stays `Send` and the closure
/// outlives any rendering thread that may invoke it.
type AssetResolverFn =
    dyn FnMut(&str) -> Option<(alloc::vec::Vec<u8>, MimeType)> + Send + 'static;

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
            resolver: None,
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
            resolver: None,
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

    /// Loads a picture from memory, copying `data` into thorvg.
    ///
    /// `mime` selects the loader (see [`MimeType`]). `resource_path`
    /// is the base directory for SVG external assets; pass `None`
    /// for self-contained content.
    ///
    /// For zero-copy loading of `'static` buffers (e.g.
    /// `include_bytes!(...)`), use [`load_data_static`](Self::load_data_static).
    pub fn load_data(
        &mut self,
        data: &[u8],
        mime: MimeType,
        resource_path: Option<&str>,
    ) -> Result<()> {
        load_data_inner(self.raw, data, mime, resource_path, /* copy = */ true)
    }

    /// Loads a picture from `'static` memory without copying.
    ///
    /// thorvg borrows `data`; the `'static` bound enforces at the type
    /// level that the buffer outlives the picture. Typical use:
    /// `pic.load_data_static(include_bytes!("logo.svg"), MimeType::Svg, None)`.
    ///
    /// # Compile-time safety
    ///
    /// ```compile_fail,E0597
    /// let engine = thorvg::Thorvg::init(0).unwrap();
    /// let mut pic = engine.picture();
    /// let local = vec![0u8; 32];
    /// pic.load_data_static(&local, thorvg::MimeType::Svg, None).unwrap();
    /// // error[E0597]: `local` does not live long enough
    /// ```
    pub fn load_data_static(
        &mut self,
        data: &'static [u8],
        mime: MimeType,
        resource_path: Option<&str>,
    ) -> Result<()> {
        load_data_inner(self.raw, data, mime, resource_path, /* copy = */ false)
    }

    /// Loads raw image data (pixel buffer), copying `data` into thorvg.
    ///
    /// For zero-copy loading of `'static` buffers, use
    /// [`load_raw_static`](Self::load_raw_static).
    pub fn load_raw(
        &mut self,
        data: &[u32],
        w: u32,
        h: u32,
        cs: crate::ColorSpace,
    ) -> Result<()> {
        load_raw_inner(self.raw, data, w, h, cs, /* copy = */ true)
    }

    /// Loads raw image data from `'static` memory without copying.
    ///
    /// thorvg borrows `data`; the `'static` bound enforces at the type
    /// level that the buffer outlives the picture.
    pub fn load_raw_static(
        &mut self,
        data: &'static [u32],
        w: u32,
        h: u32,
        cs: crate::ColorSpace,
    ) -> Result<()> {
        load_raw_inner(self.raw, data, w, h, cs, /* copy = */ false)
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

    /// Installs a Rust closure as the external-asset resolver.
    ///
    /// thorvg invokes `resolver` from inside [`Picture::load`] /
    /// [`Picture::load_from_str`] whenever the loaded document
    /// references an external image, font, etc.  The closure
    /// receives the requested asset src and returns either the
    /// resolved bytes paired with a [`MimeType`], or `None` to
    /// signal that the asset cannot be supplied.
    ///
    /// The closure is stored inside this `Picture` and lives for
    /// the picture's lifetime; calling `set_asset_resolver` again
    /// replaces the previous closure (and drops it).
    ///
    /// Must be called **before** [`Picture::load`] /
    /// [`Picture::load_from_str`] for asset resolution to kick in.
    ///
    /// ```ignore
    /// pic.set_asset_resolver(|src| {
    ///     let bytes = my_loader.fetch(src)?;
    ///     Some((bytes, thorvg::MimeType::Png))
    /// })?;
    /// pic.load_from_str("scene.svg")?;
    /// ```
    pub fn set_asset_resolver<F>(&mut self, resolver: F) -> Result<()>
    where
        F: FnMut(&str) -> Option<(alloc::vec::Vec<u8>, MimeType)> + Send + 'static,
    {
        // Detach any previous resolver from the C side first.
        // Without this, replacing `self.resolver` would drop the
        // old Box while the C side still holds its address — any
        // asset resolution in between would dereference freed memory.
        if self.resolver.is_some() {
            unsafe {
                sys::tvg_picture_set_asset_resolver(
                    self.raw,
                    None,
                    core::ptr::null_mut(),
                );
            }
        }
        // Now safe to swap.  Box<dyn> is fat; we need a thin
        // pointer for FFI, so we pass the address of the *inner*
        // Box value, which lives on the heap inside the outer Box's
        // allocation.  That heap address is stable across moves of
        // `Picture`, so storing it in C is safe even if the wrapper
        // is later moved.  The trampoline reconstructs
        // `&mut Box<dyn ...>` from this ptr.
        self.resolver = Some(alloc::boxed::Box::new(alloc::boxed::Box::new(resolver)));
        let outer = self.resolver.as_mut().unwrap();
        let data_ptr: *mut alloc::boxed::Box<AssetResolverFn> = &raw mut **outer;
        // SAFETY: `data_ptr` references a heap allocation owned by
        // `self.resolver`; `Picture::Drop` unregisters the resolver
        // before that allocation is freed, so C never dereferences
        // a dangling pointer.
        Error::from_raw(unsafe {
            sys::tvg_picture_set_asset_resolver(
                self.raw,
                Some(resolver_trampoline),
                data_ptr.cast::<core::ffi::c_void>(),
            )
        })
    }

    /// Removes any previously installed asset resolver.
    pub fn clear_asset_resolver(&mut self) -> Result<()> {
        let r = Error::from_raw(unsafe {
            sys::tvg_picture_set_asset_resolver(self.raw, None, core::ptr::null_mut())
        });
        self.resolver = None;
        r
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
            resolver: None,
            _engine: core::marker::PhantomData,
        }
    }
}

impl Drop for Picture<'_> {
    fn drop(&mut self) {
        // Unregister any installed resolver BEFORE freeing the C
        // handle and our resolver Box.  The C side keeps a pointer
        // into our Box for asset resolution; if we let the Box drop
        // first, a subsequent resolution would dereference freed
        // memory.  This matters for the `owned == false` branch too
        // because the underlying paint may outlive this wrapper.
        if self.resolver.is_some() {
            unsafe {
                sys::tvg_picture_set_asset_resolver(self.raw, None, core::ptr::null_mut());
            }
        }
        if self.owned {
            unsafe {
                sys::tvg_paint_rel(self.raw);
            }
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn load_data_inner(
    raw: sys::Tvg_Paint,
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
            raw,
            data.as_ptr().cast::<core::ffi::c_char>(),
            data.len() as u32,
            mime.as_c_str().as_ptr(),
            rpath_ptr,
            copy,
        )
    })
}

fn load_raw_inner(
    raw: sys::Tvg_Paint,
    data: &[u32],
    w: u32,
    h: u32,
    cs: crate::ColorSpace,
    copy: bool,
) -> Result<()> {
    Error::from_raw(unsafe { sys::tvg_picture_load_raw(raw, data.as_ptr(), w, h, cs.to_raw(), copy) })
}

/// FFI trampoline that bridges thorvg's C callback to the boxed
/// Rust closure stored in [`Picture::resolver`].  Looks up the
/// closure via the supplied `data` pointer (a thin pointer to the
/// Box), invokes it with the requested src path, and on success
/// loads the resolved bytes into thorvg's target paint.
unsafe extern "C" fn resolver_trampoline(
    paint: sys::Tvg_Paint,
    src: *const core::ffi::c_char,
    data: *mut core::ffi::c_void,
) -> bool {
    if data.is_null() || src.is_null() {
        return false;
    }
    let boxed = unsafe { &mut *data.cast::<alloc::boxed::Box<AssetResolverFn>>() };
    let src_str = unsafe { core::ffi::CStr::from_ptr(src) }.to_string_lossy();
    // SAFETY: user closure runs in Rust context; a panic here would
    // unwind across the C++ caller above us, which is UB.  Catch and
    // convert to a "not resolved" return.  In `no_std` builds the
    // crate-level docs require `panic = "abort"`, which makes panic
    // termination strictly safer (the process is gone before unwinding
    // could reach the FFI boundary).
    let resolved = invoke_resolver(boxed, &src_str);
    let Some((bytes, mime)) = resolved else {
        return false;
    };
    // Copy into thorvg so the consumer's Vec can drop after return.
    #[allow(clippy::cast_possible_truncation)]
    let r = unsafe {
        sys::tvg_picture_load_data(
            paint,
            bytes.as_ptr().cast::<core::ffi::c_char>(),
            bytes.len() as u32,
            mime.as_c_str().as_ptr(),
            core::ptr::null(),
            true,
        )
    };
    r == sys::Tvg_Result::TVG_RESULT_SUCCESS
}

#[cfg(feature = "std")]
fn invoke_resolver(
    boxed: &mut alloc::boxed::Box<AssetResolverFn>,
    src: &str,
) -> Option<(alloc::vec::Vec<u8>, MimeType)> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (boxed)(src))).unwrap_or(None)
}

#[cfg(not(feature = "std"))]
fn invoke_resolver(
    boxed: &mut alloc::boxed::Box<AssetResolverFn>,
    src: &str,
) -> Option<(alloc::vec::Vec<u8>, MimeType)> {
    // `no_std` users are required to build with `panic = "abort"`
    // (see crate docs).  An aborting panic cannot cross the FFI
    // boundary, so no `catch_unwind` is needed.
    (boxed)(src)
}

impl core::fmt::Debug for Picture<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Picture").finish_non_exhaustive()
    }
}
