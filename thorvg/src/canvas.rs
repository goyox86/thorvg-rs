use crate::error::{Error, Result};
use crate::paint::Paint;
use thorvg_sys as sys;

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

/// Engine rendering options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum EngineOption {
    /// No options.
    None,
    /// Default rendering mode.
    #[default]
    Default,
    /// Enable smart (partial) rendering.
    SmartRender,
}

impl EngineOption {
    pub(crate) fn to_raw(self) -> sys::Tvg_Engine_Option {
        match self {
            EngineOption::None => sys::Tvg_Engine_Option::TVG_ENGINE_OPTION_NONE,
            EngineOption::Default => sys::Tvg_Engine_Option::TVG_ENGINE_OPTION_DEFAULT,
            EngineOption::SmartRender => sys::Tvg_Engine_Option::TVG_ENGINE_OPTION_SMART_RENDER,
        }
    }
}

/// A software-rendered canvas.
///
/// This canvas uses the CPU engine for rendering.
///
/// The lifetime `'eng` ties this canvas to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create canvases via [`Thorvg::sw_canvas()`](crate::Thorvg::sw_canvas).
///
/// # Thread Safety
///
/// `SwCanvas` is [`Send`] but not [`Sync`]: you may move it to another
/// thread, but you must not share references across threads.  All
/// mutation goes through `&mut self`, so the borrow checker enforces
/// exclusive access at compile time.
pub struct SwCanvas<'eng> {
    raw: sys::Tvg_Canvas,
    _engine: core::marker::PhantomData<&'eng ()>,
}

// SAFETY: `SwCanvas` exclusively owns a heap-allocated ThorVG canvas
// (`Tvg_Canvas`).  The C++ implementation guards shared global state
// (renderer ref-count, memory pool, loader registry) with internal
// mutexes (`_rendererMtx`, `ScopedLock` / `StrictKey`), and per-canvas
// state is only accessed through `&mut self`.  Transferring sole
// ownership to another thread is therefore safe.
//
// `SwCanvas` is intentionally `!Sync`: the raw pointer field prevents
// the auto-`Sync` impl, which is correct — concurrent `&`-access to
// the same C handle would be a data race.
unsafe impl Send for SwCanvas<'_> {}

impl SwCanvas<'_> {
    /// Creates a new software canvas with the given engine options.
    pub(crate) fn new(option: EngineOption) -> Result<Self> {
        let raw = unsafe { sys::tvg_swcanvas_create(option.to_raw()) };
        if raw.is_null() {
            return Err(Error::Unknown);
        }
        Ok(Self {
            raw,
            _engine: core::marker::PhantomData,
        })
    }

    /// Sets the rendering target buffer.
    ///
    /// The buffer must be at least `stride * height` elements.
    ///
    /// # Safety
    /// The caller must ensure that `buffer` remains valid and is not moved,
    /// reallocated, or dropped for the entire lifetime of the canvas (or until
    /// `set_target` is called again with a different buffer). The canvas stores
    /// the pointer internally and writes to it during [`draw`](Self::draw) and
    /// [`sync`](Self::sync).
    pub unsafe fn set_target(
        &mut self,
        buffer: &mut [u32],
        stride: u32,
        width: u32,
        height: u32,
        colorspace: ColorSpace,
    ) -> Result<()> {
        // Check `stride * height` in u64 — the obvious `u32 * u32`
        // wraps in release (overflow-checks off) and would let a
        // pathological size pass the bound check, after which thorvg
        // would compute the same product on its side and read/write
        // past the buffer.
        let needed = u64::from(stride).checked_mul(u64::from(height));
        let Some(needed) = needed else {
            return Err(Error::InvalidArguments);
        };
        if (buffer.len() as u64) < needed {
            return Err(Error::InvalidArguments);
        }
        let result = unsafe {
            sys::tvg_swcanvas_set_target(
                self.raw,
                buffer.as_mut_ptr(),
                stride,
                width,
                height,
                colorspace.to_raw(),
            )
        };
        Error::from_raw(result)
    }

    /// Adds a paint object to the canvas for rendering.
    ///
    /// Ownership of the paint is transferred to the canvas.
    pub fn add<P: Paint>(&mut self, paint: P) -> Result<()> {
        let raw_paint = paint.into_raw();
        let result = unsafe { sys::tvg_canvas_add(self.raw, raw_paint) };
        Error::from_raw(result)
    }

    /// Inserts a paint object before another existing paint in the canvas.
    ///
    /// Ownership of `target` is transferred to the canvas.
    pub fn insert<P: Paint, Q: Paint>(&mut self, target: P, at: &Q) -> Result<()> {
        let result = unsafe { sys::tvg_canvas_insert(self.raw, target.into_raw(), at.raw()) };
        Error::from_raw(result)
    }

    /// Removes a paint object from the canvas.
    pub fn remove<P: Paint>(&mut self, paint: &P) -> Result<()> {
        let result = unsafe { sys::tvg_canvas_remove(self.raw, paint.raw()) };
        Error::from_raw(result)
    }

    /// Removes all paint objects from the canvas.
    pub fn clear(&mut self) -> Result<()> {
        let result = unsafe { sys::tvg_canvas_remove(self.raw, core::ptr::null_mut()) };
        Error::from_raw(result)
    }

    /// Updates all modified paint objects in preparation for rendering.
    pub fn update(&mut self) -> Result<()> {
        let result = unsafe { sys::tvg_canvas_update(self.raw) };
        Error::from_raw(result)
    }

    /// Renders all paint objects on the canvas.
    ///
    /// If `clear` is true, the target buffer is cleared before drawing.
    pub fn draw(&mut self, clear: bool) -> Result<()> {
        let result = unsafe { sys::tvg_canvas_draw(self.raw, clear) };
        Error::from_raw(result)
    }

    /// Waits for the rendering to finish.
    pub fn sync(&mut self) -> Result<()> {
        let result = unsafe { sys::tvg_canvas_sync(self.raw) };
        Error::from_raw(result)
    }

    /// Sets the drawing viewport (clipping region).
    pub fn set_viewport(&mut self, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
        let result = unsafe { sys::tvg_canvas_set_viewport(self.raw, x, y, w, h) };
        Error::from_raw(result)
    }

    /// Update, draw (clearing the buffer), and sync in one call.
    ///
    /// Equivalent to calling [`update`](Self::update), [`draw(true)`](Self::draw),
    /// and [`sync`](Self::sync) in sequence.
    pub fn render(&mut self) -> Result<()> {
        self.update()?;
        self.draw(true)?;
        self.sync()
    }
}

impl Drop for SwCanvas<'_> {
    fn drop(&mut self) {
        unsafe {
            sys::tvg_canvas_destroy(self.raw);
        }
    }
}

// ── Shared canvas operations ───────────────────────────────────────

macro_rules! impl_canvas_ops {
    ($ty:ident) => {
        impl $ty<'_> {
            /// Adds a paint object to the canvas for rendering.
            ///
            /// Ownership of the paint is transferred to the canvas.
            pub fn add<P: Paint>(&mut self, paint: P) -> Result<()> {
                let raw_paint = paint.into_raw();
                Error::from_raw(unsafe { sys::tvg_canvas_add(self.raw, raw_paint) })
            }

            /// Inserts a paint object before another existing paint in the canvas.
            pub fn insert<P: Paint, Q: Paint>(&mut self, target: P, at: &Q) -> Result<()> {
                Error::from_raw(unsafe {
                    sys::tvg_canvas_insert(self.raw, target.into_raw(), at.raw())
                })
            }

            /// Removes a paint object from the canvas.
            pub fn remove<P: Paint>(&mut self, paint: &P) -> Result<()> {
                Error::from_raw(unsafe { sys::tvg_canvas_remove(self.raw, paint.raw()) })
            }

            /// Removes all paint objects from the canvas.
            pub fn clear(&mut self) -> Result<()> {
                Error::from_raw(unsafe { sys::tvg_canvas_remove(self.raw, core::ptr::null_mut()) })
            }

            /// Updates all modified paint objects in preparation for rendering.
            pub fn update(&mut self) -> Result<()> {
                Error::from_raw(unsafe { sys::tvg_canvas_update(self.raw) })
            }

            /// Renders all paint objects on the canvas.
            pub fn draw(&mut self, clear: bool) -> Result<()> {
                Error::from_raw(unsafe { sys::tvg_canvas_draw(self.raw, clear) })
            }

            /// Waits for the rendering to finish.
            pub fn sync(&mut self) -> Result<()> {
                Error::from_raw(unsafe { sys::tvg_canvas_sync(self.raw) })
            }

            /// Sets the drawing viewport (clipping region).
            pub fn set_viewport(&mut self, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
                Error::from_raw(unsafe { sys::tvg_canvas_set_viewport(self.raw, x, y, w, h) })
            }

            /// Update, draw (clearing the buffer), and sync in one call.
            ///
            /// Equivalent to calling [`update`](Self::update), [`draw(true)`](Self::draw),
            /// and [`sync`](Self::sync) in sequence.
            pub fn render(&mut self) -> Result<()> {
                self.update()?;
                self.draw(true)?;
                self.sync()
            }
        }

        impl Drop for $ty<'_> {
            fn drop(&mut self) {
                unsafe {
                    sys::tvg_canvas_destroy(self.raw);
                }
            }
        }
    };
}

// ── GlCanvas ───────────────────────────────────────────────────────

/// An OpenGL/ES-rendered canvas.
///
/// The lifetime `'eng` ties this canvas to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create canvases via [`Thorvg::gl_canvas()`](crate::Thorvg::gl_canvas).
///
/// # Thread Safety
///
/// `GlCanvas` is [`Send`] but not [`Sync`]: you may move it to another
/// thread, but you must not share references across threads.
pub struct GlCanvas<'eng> {
    raw: sys::Tvg_Canvas,
    _engine: core::marker::PhantomData<&'eng ()>,
}

// SAFETY: same rationale as `SwCanvas` — exclusive ownership of a
// heap-allocated C handle; global state is mutex-protected.
unsafe impl Send for GlCanvas<'_> {}

impl GlCanvas<'_> {
    /// Creates a new OpenGL canvas with the given engine options.
    pub(crate) fn new(option: EngineOption) -> Result<Self> {
        let raw = unsafe { sys::tvg_glcanvas_create(option.to_raw()) };
        if raw.is_null() {
            return Err(Error::Unknown);
        }
        Ok(Self {
            raw,
            _engine: core::marker::PhantomData,
        })
    }

    /// Sets the OpenGL drawing target.
    ///
    /// - `display` — platform-specific display handle (`EGLDisplay`), or `null` for non-EGL.
    /// - `surface` — platform-specific surface handle (`EGLSurface` / `HDC`), or `null`.
    /// - `context` — the OpenGL context for rendering.
    /// - `id` — GL target ID (FBO ID), `0` for the main surface.
    /// - `w`, `h` — dimensions in pixels.
    /// - `colorspace` — pixel format (currently only `ABGR8888S` as `GL_RGBA8`).
    ///
    /// # Safety
    /// The caller must ensure the provided pointers are valid GL/EGL handles.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn set_target(
        &mut self,
        display: *mut core::ffi::c_void,
        surface: *mut core::ffi::c_void,
        context: *mut core::ffi::c_void,
        id: i32,
        w: u32,
        h: u32,
        colorspace: ColorSpace,
    ) -> Result<()> {
        Error::from_raw(unsafe {
            sys::tvg_glcanvas_set_target(
                self.raw,
                display,
                surface,
                context,
                id,
                w,
                h,
                colorspace.to_raw(),
            )
        })
    }
}

impl_canvas_ops!(GlCanvas);

// ── WgCanvas ───────────────────────────────────────────────────────

/// WebGPU target type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WgTargetType {
    /// Use a `WGPUSurface` as the presentable target.
    Surface = 0,
    /// Use a `WGPUTexture` as the presentable target.
    Texture = 1,
}

/// A WebGPU-rendered canvas.
///
/// The lifetime `'eng` ties this canvas to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create canvases via [`Thorvg::wg_canvas()`](crate::Thorvg::wg_canvas).
///
/// # Thread Safety
///
/// `WgCanvas` is [`Send`] but not [`Sync`]: you may move it to another
/// thread, but you must not share references across threads.
pub struct WgCanvas<'eng> {
    raw: sys::Tvg_Canvas,
    _engine: core::marker::PhantomData<&'eng ()>,
}

// SAFETY: same rationale as `SwCanvas` — exclusive ownership of a
// heap-allocated C handle; global state is mutex-protected.
unsafe impl Send for WgCanvas<'_> {}

impl WgCanvas<'_> {
    /// Creates a new WebGPU canvas with the given engine options.
    pub(crate) fn new(option: EngineOption) -> Result<Self> {
        let raw = unsafe { sys::tvg_wgcanvas_create(option.to_raw()) };
        if raw.is_null() {
            return Err(Error::Unknown);
        }
        Ok(Self {
            raw,
            _engine: core::marker::PhantomData,
        })
    }

    /// Sets the WebGPU drawing target.
    ///
    /// - `device` — `WGPUDevice` handle, or `null` to let `ThorVG` assign one.
    /// - `instance` — `WGPUInstance` context.
    /// - `target` — `WGPUSurface` or `WGPUTexture` handle.
    /// - `w`, `h` — dimensions.
    /// - `colorspace` — pixel format (currently only `ABGR8888S` as `RGBA8Unorm`).
    /// - `target_type` — whether `target` is a surface or texture.
    ///
    /// # Safety
    /// The caller must ensure the provided pointers are valid WebGPU handles.
    #[allow(clippy::too_many_arguments)]
    pub unsafe fn set_target(
        &mut self,
        device: *mut core::ffi::c_void,
        instance: *mut core::ffi::c_void,
        target: *mut core::ffi::c_void,
        w: u32,
        h: u32,
        colorspace: ColorSpace,
        target_type: WgTargetType,
    ) -> Result<()> {
        Error::from_raw(unsafe {
            sys::tvg_wgcanvas_set_target(
                self.raw,
                device,
                instance,
                target,
                w,
                h,
                colorspace.to_raw(),
                target_type as i32,
            )
        })
    }
}

impl_canvas_ops!(WgCanvas);

impl core::fmt::Debug for SwCanvas<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SwCanvas").finish_non_exhaustive()
    }
}

impl core::fmt::Debug for GlCanvas<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("GlCanvas").finish_non_exhaustive()
    }
}

impl core::fmt::Debug for WgCanvas<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WgCanvas").finish_non_exhaustive()
    }
}
