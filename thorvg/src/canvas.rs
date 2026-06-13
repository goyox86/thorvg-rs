use crate::error::{Error, Result};
use crate::paint::Paint;
use thorvg_sys as sys;

/// Sealed-trait marker.
///
/// Restricts [`Canvas`] to the three concrete canvas types defined
/// in this crate.  Downstream crates can use [`Canvas`] as a bound
/// in generic code but cannot add new canvas backends — those are
/// owned by the engine.
mod sealed {
    pub trait Sealed {}
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

/// Engine rendering options, modelled as bitflags.
///
/// Maps to the C bitfield enum `Tvg_Engine_Option`.  Values are
/// power-of-two bits intended to be combined with `|`:
///
/// ```ignore
/// engine.sw_canvas(EngineOption::DEFAULT | EngineOption::SMART_RENDER)
/// ```
///
/// # Per-canvas restrictions
///
/// * [`SwCanvas`] honours every flag.
/// * [`GlCanvas`] and [`WgCanvas`] ignore [`SMART_RENDER`](Self::SMART_RENDER)
///   today — the C engine documents the request as silently dropped
///   on the GPU backends (`tvg_glcanvas_create` / `tvg_wgcanvas_create`
///   header notes).  The flag is still accepted to keep the API
///   uniform; behaviour is identical to passing
///   [`DEFAULT`](Self::DEFAULT) on those backends.
///
/// `Default::default()` returns [`DEFAULT`](Self::DEFAULT), matching
/// the previous behaviour of the now-removed `Default` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EngineOption(sys::Tvg_Engine_Option);

impl EngineOption {
    /// No options enabled.  Explicitly disables every optional
    /// behaviour.
    pub const NONE: Self = Self(sys::Tvg_Engine_Option::TVG_ENGINE_OPTION_NONE);
    /// Use the default rendering mode.
    pub const DEFAULT: Self = Self(sys::Tvg_Engine_Option::TVG_ENGINE_OPTION_DEFAULT);
    /// Enable smart (partial) rendering optimisations — thorvg only
    /// redraws regions of the canvas that changed between frames.
    ///
    /// May *hurt* performance on full-screen / large-area updates
    /// because of change-tracking overhead; recommended only for
    /// mostly-static UIs.  Ignored by [`GlCanvas`] / [`WgCanvas`]
    /// (see type docs).
    pub const SMART_RENDER: Self = Self(sys::Tvg_Engine_Option::TVG_ENGINE_OPTION_SMART_RENDER);

    /// Returns the empty flag set (equivalent to [`NONE`](Self::NONE)).
    #[must_use]
    pub const fn empty() -> Self {
        Self::NONE
    }

    /// Returns `true` if every flag in `other` is also set in `self`.
    #[must_use]
    pub const fn contains(self, other: Self) -> bool {
        // `self.0` is the `sys::Tvg_Engine_Option` newtype; its `.0`
        // is the raw `u32` bitfield underneath.
        let bits = self.0 .0;
        let mask = other.0 .0;
        (bits & mask) == mask
    }

    pub(crate) fn to_raw(self) -> sys::Tvg_Engine_Option {
        self.0
    }
}

impl Default for EngineOption {
    /// Returns [`EngineOption::DEFAULT`].
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl core::ops::BitOr for EngineOption {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for EngineOption {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl core::ops::BitAnd for EngineOption {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl core::ops::BitAndAssign for EngineOption {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

// ── Shared canvas operations ───────────────────────────────────────

/// Operations shared across [`SwCanvas`], [`GlCanvas`], and
/// [`WgCanvas`].
///
/// All methods forward to the same `tvg_canvas_*` C functions; the
/// trait exists so generic code can accept "any canvas" without
/// caring about the rendering backend:
///
/// ```ignore
/// fn finalise<C: thorvg::Canvas>(c: &mut C) -> thorvg::Result<()> {
///     c.draw(true)?;
///     c.sync()
/// }
/// ```
///
/// Every method is also exposed as an inherent method on each
/// canvas type, so callers that don't need genericity can keep
/// writing `canvas.add(shape)` without a `use thorvg::Canvas;`.
///
/// # Sealed
///
/// The trait is sealed via `sealed::Sealed` — only the canvas types
/// defined in this crate can implement it.  Adding a new canvas
/// backend is the engine's responsibility, not the user's.
pub trait Canvas: sealed::Sealed {
    /// Adds a paint object to the canvas for rendering.
    ///
    /// Ownership of the paint is transferred to the canvas.
    fn add<P: Paint>(&mut self, paint: P) -> Result<()>;

    /// Inserts a paint object before another existing paint in the canvas.
    ///
    /// Ownership of `target` is transferred to the canvas.
    fn insert<P: Paint, Q: Paint>(&mut self, target: P, at: &Q) -> Result<()>;

    /// Removes a paint object from the canvas.
    fn remove<P: Paint>(&mut self, paint: &P) -> Result<()>;

    /// Removes all paint objects from the canvas.
    fn clear(&mut self) -> Result<()>;

    /// Updates all modified paint objects in preparation for rendering.
    fn update(&mut self) -> Result<()>;

    /// Renders all paint objects on the canvas.
    fn draw(&mut self, clear: bool) -> Result<()>;

    /// Waits for the rendering to finish.
    fn sync(&mut self) -> Result<()>;

    /// Sets the drawing viewport (clipping region).
    fn set_viewport(&mut self, x: i32, y: i32, w: i32, h: i32) -> Result<()>;

    /// Update, draw (clearing the buffer), and sync in one call.
    fn render(&mut self) -> Result<()>;
}

/// Declares a canvas backend.
///
/// Generates the wrapper struct, its `Send` impl, the
/// `(pub(crate)) new` constructor that calls the backend-specific
/// `tvg_*canvas_create`, the inherent canvas ops, the [`Canvas`]
/// trait impl that forwards to them, plus `Debug` and `Drop`.
///
/// Backend-specific `set_target` methods live in their own
/// `impl Foo<'_>` blocks below — the macro only handles the parts
/// that are byte-for-byte identical across SW/GL/WG.
macro_rules! impl_canvas {
    (
        $(#[$meta:meta])*
        $ty:ident, $create_fn:ident
    ) => {
        $(#[$meta])*
        pub struct $ty<'eng> {
            raw: sys::Tvg_Canvas,
            _engine: core::marker::PhantomData<&'eng ()>,
        }

        // SAFETY: each canvas exclusively owns a heap-allocated
        // `Tvg_Canvas`.  The C++ engine guards shared global state
        // (renderer ref-count, memory pool, loader registry) with
        // internal mutexes (`_rendererMtx`, `ScopedLock` /
        // `StrictKey`), and per-canvas state is reached only through
        // `&mut self`.  Transferring sole ownership across threads is
        // therefore sound.
        //
        // The type is intentionally `!Sync`: the raw pointer field
        // suppresses the auto-`Sync` impl, which is correct —
        // concurrent shared access to the same C handle would race.
        unsafe impl Send for $ty<'_> {}

        impl $ty<'_> {
            /// Creates a new canvas with the given engine options.
            pub(crate) fn new(option: EngineOption) -> Result<Self> {
                let raw = unsafe { sys::$create_fn(option.to_raw()) };
                if raw.is_null() {
                    return Err(Error::Unknown);
                }
                Ok(Self {
                    raw,
                    _engine: core::marker::PhantomData,
                })
            }

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

        impl sealed::Sealed for $ty<'_> {}

        impl Canvas for $ty<'_> {
            // Forward each method to the inherent impl above.  The
            // explicit `$ty::method(self, ...)` syntax avoids any
            // method-resolution ambiguity with the trait method
            // being defined here.
            fn add<P: Paint>(&mut self, paint: P) -> Result<()> {
                $ty::add(self, paint)
            }
            fn insert<P: Paint, Q: Paint>(&mut self, target: P, at: &Q) -> Result<()> {
                $ty::insert(self, target, at)
            }
            fn remove<P: Paint>(&mut self, paint: &P) -> Result<()> {
                $ty::remove(self, paint)
            }
            fn clear(&mut self) -> Result<()> {
                $ty::clear(self)
            }
            fn update(&mut self) -> Result<()> {
                $ty::update(self)
            }
            fn draw(&mut self, clear: bool) -> Result<()> {
                $ty::draw(self, clear)
            }
            fn sync(&mut self) -> Result<()> {
                $ty::sync(self)
            }
            fn set_viewport(&mut self, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
                $ty::set_viewport(self, x, y, w, h)
            }
            fn render(&mut self) -> Result<()> {
                $ty::render(self)
            }
        }

        impl core::fmt::Debug for $ty<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                f.debug_struct(stringify!($ty)).finish_non_exhaustive()
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

// ── SwCanvas ───────────────────────────────────────────────────────

impl_canvas! {
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
    SwCanvas, tvg_swcanvas_create
}

impl SwCanvas<'_> {
    /// Sets the rendering target buffer.
    ///
    /// The buffer must be at least `stride * height` elements, and
    /// `stride` must be `>= width` (otherwise rows would overlap or
    /// truncate).
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
        // Stride is the row pitch in pixels and must be at least as
        // wide as the visible row — otherwise thorvg would write
        // partial rows that bleed into the next one.
        if stride < width {
            return Err(Error::InvalidArguments);
        }
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
}

// ── GlCanvas ───────────────────────────────────────────────────────

impl_canvas! {
    /// An OpenGL/ES-rendered canvas.
    ///
    /// The lifetime `'eng` ties this canvas to a [`Thorvg`](crate::Thorvg) engine
    /// instance. Create canvases via [`Thorvg::gl_canvas()`](crate::Thorvg::gl_canvas).
    ///
    /// # Thread Safety
    ///
    /// `GlCanvas` is [`Send`] but not [`Sync`]: you may move it to another
    /// thread, but you must not share references across threads.
    GlCanvas, tvg_glcanvas_create
}

impl GlCanvas<'_> {
    /// Sets the OpenGL drawing target.
    ///
    /// See [`GlTarget`] for the parameter layout.
    ///
    /// # Safety
    /// The caller must ensure the pointer fields of `target` are
    /// valid GL/EGL handles (or null where allowed; see
    /// [`GlTarget`]).
    pub unsafe fn set_target(&mut self, target: GlTarget) -> Result<()> {
        let GlTarget {
            display,
            surface,
            context,
            id,
            width,
            height,
            colorspace,
        } = target;
        Error::from_raw(unsafe {
            sys::tvg_glcanvas_set_target(
                self.raw,
                display,
                surface,
                context,
                id,
                width,
                height,
                colorspace.to_raw(),
            )
        })
    }
}

/// Parameters for [`GlCanvas::set_target`].
///
/// Bundles the seven positional arguments of the underlying
/// `tvg_glcanvas_set_target(display, surface, context, id, w, h, cs)`
/// call.  All pointer fields are opaque GL/EGL handles; the
/// `set_target` method is `unsafe` because the caller is
/// responsible for handle validity.
///
/// Field set is closed; either construct via the [`new`](Self::new)
/// helper or use a struct literal directly.
#[derive(Debug, Clone, Copy)]
pub struct GlTarget {
    /// Platform-specific display handle (`EGLDisplay` for EGL,
    /// `null` for non-EGL backends).
    pub display: *mut core::ffi::c_void,
    /// Platform-specific surface handle (`EGLSurface` for EGL,
    /// `HDC` for WGL, `null` if not applicable).
    pub surface: *mut core::ffi::c_void,
    /// OpenGL context handle used for rendering.
    pub context: *mut core::ffi::c_void,
    /// GL target ID (typically an FBO ID); `0` selects the main
    /// surface.
    pub id: i32,
    /// Target width in pixels.
    pub width: u32,
    /// Target height in pixels.
    pub height: u32,
    /// Pixel format.  thorvg currently accepts only
    /// [`ColorSpace::ABGR8888S`] (mapped to `GL_RGBA8`); other
    /// values are rejected by the engine at runtime.
    pub colorspace: ColorSpace,
}

impl GlTarget {
    /// Builds a [`GlTarget`] from its seven required fields.
    ///
    /// All fields are required — unlike [`Rect`](crate::Rect),
    /// there are no sensible defaults for GL/EGL handles.  This
    /// constructor exists for callers that prefer positional args
    /// to a struct literal; both forms produce identical values.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        display: *mut core::ffi::c_void,
        surface: *mut core::ffi::c_void,
        context: *mut core::ffi::c_void,
        id: i32,
        width: u32,
        height: u32,
        colorspace: ColorSpace,
    ) -> Self {
        Self {
            display,
            surface,
            context,
            id,
            width,
            height,
            colorspace,
        }
    }
}

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

impl_canvas! {
    /// A WebGPU-rendered canvas.
    ///
    /// The lifetime `'eng` ties this canvas to a [`Thorvg`](crate::Thorvg) engine
    /// instance. Create canvases via [`Thorvg::wg_canvas()`](crate::Thorvg::wg_canvas).
    ///
    /// # Thread Safety
    ///
    /// `WgCanvas` is [`Send`] but not [`Sync`]: you may move it to another
    /// thread, but you must not share references across threads.
    WgCanvas, tvg_wgcanvas_create
}

impl WgCanvas<'_> {
    /// Sets the WebGPU drawing target.
    ///
    /// See [`WgTarget`] for the parameter layout.
    ///
    /// # Safety
    /// The caller must ensure the pointer fields of `target` are
    /// valid WebGPU handles (or null where allowed; see
    /// [`WgTarget`]).
    pub unsafe fn set_target(&mut self, target: WgTarget) -> Result<()> {
        let WgTarget {
            device,
            instance,
            target: target_handle,
            width,
            height,
            colorspace,
            target_type,
        } = target;
        Error::from_raw(unsafe {
            sys::tvg_wgcanvas_set_target(
                self.raw,
                device,
                instance,
                target_handle,
                width,
                height,
                colorspace.to_raw(),
                target_type as i32,
            )
        })
    }
}

/// Parameters for [`WgCanvas::set_target`].
///
/// Bundles the seven positional arguments of the underlying
/// `tvg_wgcanvas_set_target(device, instance, target, w, h, cs, type)`
/// call.  All pointer fields are opaque WebGPU handles; the
/// `set_target` method is `unsafe` because the caller is
/// responsible for handle validity.
#[derive(Debug, Clone, Copy)]
pub struct WgTarget {
    /// `WGPUDevice` handle, or `null` to let thorvg assign one
    /// internally.
    pub device: *mut core::ffi::c_void,
    /// `WGPUInstance` context handle.
    pub instance: *mut core::ffi::c_void,
    /// Presentable target: either a `WGPUSurface` or a
    /// `WGPUTexture`, discriminated by [`target_type`](Self::target_type).
    pub target: *mut core::ffi::c_void,
    /// Target width in pixels.
    pub width: u32,
    /// Target height in pixels.
    pub height: u32,
    /// Pixel format.  thorvg currently accepts only
    /// [`ColorSpace::ABGR8888S`] (mapped to `WGPUTextureFormat_RGBA8Unorm`).
    pub colorspace: ColorSpace,
    /// Whether [`target`](Self::target) is a surface or texture.
    pub target_type: WgTargetType,
}

impl WgTarget {
    /// Builds a [`WgTarget`] from its seven required fields.
    ///
    /// See [`GlTarget::new`] for the rationale on positional vs
    /// struct-literal construction.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        device: *mut core::ffi::c_void,
        instance: *mut core::ffi::c_void,
        target: *mut core::ffi::c_void,
        width: u32,
        height: u32,
        colorspace: ColorSpace,
        target_type: WgTargetType,
    ) -> Self {
        Self {
            device,
            instance,
            target,
            width,
            height,
            colorspace,
            target_type,
        }
    }
}
