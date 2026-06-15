//! Rendering canvases (software, OpenGL, WebGPU) and the shared
//! [`Canvas`] trait.
//!
//! Wraps the [`ThorVG` C API](https://www.thorvg.org/c-native).

use crate::color::ColorSpace;
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

/// Engine rendering option, selected per canvas at creation.
///
/// Mirrors the C++ `EngineOption` enum.  The C header gives the values
/// power-of-two literals (`NONE = 0`, `DEFAULT = 1 << 0`,
/// `SMART_RENDER = 1 << 1`), but the engine compares the option by
/// *exact value*, not as a bitmask — so the variants are mutually
/// exclusive and are not meant to be combined.
///
/// # Per-backend behaviour
///
/// * [`SwCanvas`]: [`None`](Self::None) disables partial (dirty-region)
///   redraw, which is otherwise enabled.  [`Default`](Self::Default)
///   and [`SmartRender`](Self::SmartRender) both leave it enabled, so
///   they are equivalent today.
/// * [`GlCanvas`] / [`WgCanvas`]: the option is ignored entirely — the
///   GPU renderers take it but do nothing with it.
///
/// `Default::default()` is [`Default`](Self::Default).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum EngineOption {
    /// Disable all optional behaviour.  On [`SwCanvas`] this turns off
    /// partial (dirty-region) redraw.
    None,
    /// Default rendering mode.  Partial (dirty-region) redraw stays
    /// enabled on [`SwCanvas`].
    #[default]
    Default,
    /// Requests partial (smart) rendering.  Identical to
    /// [`Default`](Self::Default) on [`SwCanvas`] today (partial redraw
    /// is already on there) and ignored on the GPU backends.
    SmartRender,
    /// Disables anti-aliased rendering.  Re-introduced upstream in
    /// `ThorVG` 1.0.6 (it had been dropped in 1.0.5).
    ///
    /// *Experimental:* upstream marks this option experimental, so its
    /// behaviour may change in a future `ThorVG` release.
    Aliased,
}

impl EngineOption {
    pub(crate) fn to_raw(self) -> sys::Tvg_Engine_Option {
        match self {
            EngineOption::None => sys::Tvg_Engine_Option::TVG_ENGINE_OPTION_NONE,
            EngineOption::Default => sys::Tvg_Engine_Option::TVG_ENGINE_OPTION_DEFAULT,
            EngineOption::SmartRender => sys::Tvg_Engine_Option::TVG_ENGINE_OPTION_SMART_RENDER,
            EngineOption::Aliased => sys::Tvg_Engine_Option::TVG_ENGINE_OPTION_ALIASED,
        }
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
    /// Adds a paint object to the canvas for rendering, appending it
    /// after any previously added paints.
    ///
    /// Ownership of the paint is transferred to the canvas; it is
    /// released when the canvas is dropped or the paint is removed.
    /// Paints render in the order they are added.
    ///
    /// See [`SwCanvas::add`] for details.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InsufficientCondition`] if the canvas is not in
    /// a valid state to accept new paints.
    fn add<P: Paint>(&mut self, paint: P) -> Result<()>;

    /// Inserts a paint object immediately before another existing paint
    /// in the canvas.
    ///
    /// Ownership of `target` is transferred to the canvas. Paints
    /// render in scene order, so `target` will render before `at`.
    ///
    /// See [`SwCanvas::insert`] for details.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if `at` is not a paint
    /// currently held by this canvas.
    fn insert<P: Paint, Q: Paint>(&mut self, target: P, at: &Q) -> Result<()>;

    /// Removes a paint object from the canvas, dropping the canvas'
    /// reference to it.
    ///
    /// See [`SwCanvas::remove`] for details.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if `paint` is not held by
    /// this canvas.
    fn remove<P: Paint>(&mut self, paint: &P) -> Result<()>;

    /// Removes all paint objects from the canvas.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the underlying engine reports a failure.
    fn clear(&mut self) -> Result<()>;

    /// Updates all modified paint objects in preparation for rendering.
    ///
    /// See [`SwCanvas::update`] for details.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InsufficientCondition`] if the canvas is not
    /// prepared — for instance when no target has been set, or while a
    /// previous [`draw`](Self::draw) has not been followed by
    /// [`sync`](Self::sync).
    fn update(&mut self) -> Result<()>;

    /// Renders all paint objects on the canvas.
    ///
    /// See [`SwCanvas::draw`] for details.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InsufficientCondition`] if the canvas is not
    /// prepared — for instance when no target has been set, or while a
    /// previous draw has not been followed by [`sync`](Self::sync).
    fn draw(&mut self, clear: bool) -> Result<()>;

    /// Waits for the rendering started by [`draw`](Self::draw) to
    /// finish.
    ///
    /// See [`SwCanvas::sync`] for details.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the underlying engine reports a failure.
    fn sync(&mut self) -> Result<()>;

    /// Sets the drawing viewport (clipping region).
    ///
    /// See [`SwCanvas::set_viewport`] for the ordering and synced-state
    /// requirements.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InsufficientCondition`] if the canvas is not in
    /// a synced state, or if the viewport is changed after
    /// [`add`](Self::add), `remove`, [`update`](Self::update), or
    /// [`draw`](Self::draw).
    fn set_viewport(&mut self, x: i32, y: i32, w: i32, h: i32) -> Result<()>;

    /// Updates, draws (clearing the buffer), and syncs in one call.
    ///
    /// Equivalent to [`update`](Self::update),
    /// [`draw(true)`](Self::draw), then [`sync`](Self::sync).
    ///
    /// # Errors
    ///
    /// Returns the first [`Error`] produced by the wrapped
    /// [`update`](Self::update), [`draw`](Self::draw), or
    /// [`sync`](Self::sync) call.
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

            /// Adds a paint object to the canvas for rendering,
            /// appending it after any previously added paints.
            ///
            /// Ownership of the paint is transferred to the canvas:
            /// the paint is released when the canvas is dropped, when
            /// the canvas is [`clear`](Self::clear)ed, or when the
            /// paint is [`remove`](Self::remove)d. Paints render in the
            /// order they are added, so later additions draw on top.
            ///
            /// # Errors
            ///
            /// Returns [`Error::InsufficientCondition`] if the canvas
            /// is not in a valid state to accept new paints.
            pub fn add<P: Paint>(&mut self, paint: P) -> Result<()> {
                let raw_paint = paint.into_raw();
                Error::from_raw(unsafe { sys::tvg_canvas_add(self.raw, raw_paint) })
            }

            /// Inserts a paint object immediately before another
            /// existing paint in the canvas.
            ///
            /// Ownership of `target` is transferred to the canvas (see
            /// [`add`](Self::add)). Because paints render in scene
            /// order, `target` draws *before* `at`.
            ///
            /// # Errors
            ///
            /// Returns [`Error::InvalidArguments`] if `at` is not a
            /// paint currently held by this canvas. (The wrapper always
            /// passes a non-null `at`; the C API would otherwise append
            /// when `at` is null.)
            pub fn insert<P: Paint, Q: Paint>(&mut self, target: P, at: &Q) -> Result<()> {
                Error::from_raw(unsafe {
                    sys::tvg_canvas_insert(self.raw, target.into_raw(), at.raw())
                })
            }

            /// Removes a paint object from the canvas, dropping the
            /// canvas' reference to it.
            ///
            /// # Errors
            ///
            /// Returns [`Error::InvalidArguments`] if `paint` is not
            /// held by this canvas.
            pub fn remove<P: Paint>(&mut self, paint: &P) -> Result<()> {
                Error::from_raw(unsafe { sys::tvg_canvas_remove(self.raw, paint.raw()) })
            }

            /// Removes all paint objects from the canvas.
            ///
            /// Forwards to `tvg_canvas_remove` with a null paint, which
            /// the C API treats as "remove everything".
            ///
            /// # Errors
            ///
            /// Returns an [`Error`] if the underlying engine reports a
            /// failure.
            pub fn clear(&mut self) -> Result<()> {
                Error::from_raw(unsafe { sys::tvg_canvas_remove(self.raw, core::ptr::null_mut()) })
            }

            /// Updates all modified paint objects in preparation for
            /// rendering.
            ///
            /// Only paints changed since the last update are
            /// reprocessed. [`draw`](Self::draw) performs this
            /// implicitly if the canvas has not been updated, so an
            /// explicit call is only needed when you want updating and
            /// drawing separated.
            ///
            /// # Errors
            ///
            /// Returns [`Error::InsufficientCondition`] if the canvas
            /// is not prepared — for instance when no target has been
            /// set, or while a previous [`draw`](Self::draw) has not
            /// been followed by [`sync`](Self::sync).
            pub fn update(&mut self) -> Result<()> {
                Error::from_raw(unsafe { sys::tvg_canvas_update(self.raw) })
            }

            /// Renders all paint objects on the canvas.
            ///
            /// If `clear` is `true`, the target buffer is zeroed before
            /// drawing; pass `false` to skip the clear when the canvas
            /// will be fully covered by opaque content. Rendering may
            /// run asynchronously, so call [`sync`](Self::sync)
            /// afterwards to guarantee completion before reading the
            /// target.
            ///
            /// # Errors
            ///
            /// Returns [`Error::InsufficientCondition`] if the canvas
            /// is not prepared — for instance when no target has been
            /// set, or while a previous draw has not been followed by
            /// [`sync`](Self::sync).
            pub fn draw(&mut self, clear: bool) -> Result<()> {
                Error::from_raw(unsafe { sys::tvg_canvas_draw(self.raw, clear) })
            }

            /// Waits for the rendering started by [`draw`](Self::draw)
            /// to finish.
            ///
            /// Must be called after every [`draw`](Self::draw)
            /// regardless of threading. Until it returns, the target
            /// buffer is being written by the engine and must not be
            /// accessed.
            ///
            /// # Errors
            ///
            /// Returns an [`Error`] if the underlying engine reports a
            /// failure.
            pub fn sync(&mut self) -> Result<()> {
                Error::from_raw(unsafe { sys::tvg_canvas_sync(self.raw) })
            }

            /// Sets the drawing viewport (clipping region).
            ///
            /// Must be called **before** adding paints or drawing — changing
            /// the viewport after [`add`](Self::add), `remove`,
            /// [`update`](Self::update), or [`draw`](Self::draw) is rejected
            /// with [`Error::InsufficientCondition`]. The canvas must be in a
            /// synced state. Resetting the target also resets the viewport to
            /// the target size.
            ///
            /// # Errors
            ///
            /// Returns [`Error::InsufficientCondition`] if the canvas
            /// is not in a synced state, or if the viewport is changed
            /// after [`add`](Self::add), `remove`,
            /// [`update`](Self::update), or [`draw`](Self::draw).
            pub fn set_viewport(&mut self, x: i32, y: i32, w: i32, h: i32) -> Result<()> {
                Error::from_raw(unsafe { sys::tvg_canvas_set_viewport(self.raw, x, y, w, h) })
            }

            /// Updates, draws (clearing the buffer), and syncs in one
            /// call.
            ///
            /// Equivalent to calling [`update`](Self::update), [`draw(true)`](Self::draw),
            /// and [`sync`](Self::sync) in sequence.
            ///
            /// # Errors
            ///
            /// Returns the first [`Error`] produced by the wrapped
            /// [`update`](Self::update), [`draw`](Self::draw), or
            /// [`sync`](Self::sync) call.
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
    /// `stride` is the row pitch in pixels (`u32` elements) and is
    /// usually equal to `width`; `width`/`height` are the visible
    /// raster dimensions. `ThorVG` does not allocate the output buffer
    /// itself — the caller owns it and it must hold at least
    /// `stride * height` `u32` pixels. `colorspace` selects how those
    /// 32-bit values are interpreted; `ThorVG` accepts only the four
    /// 8888 spaces ([`ColorSpace::ABGR8888`], [`ColorSpace::ARGB8888`],
    /// [`ColorSpace::ABGR8888S`], [`ColorSpace::ARGB8888S`]).
    ///
    /// The canvas must be in a synced state; if a previous
    /// [`draw`](Self::draw) is still in flight, call [`sync`](Self::sync)
    /// first. Resetting the target also resets the viewport to the new
    /// target size.
    ///
    /// # Safety
    /// The caller must ensure that `buffer` remains valid and is not moved,
    /// reallocated, or dropped for the entire lifetime of the canvas (or until
    /// `set_target` is called again with a different buffer). The canvas stores
    /// the pointer internally and writes to it during [`draw`](Self::draw) and
    /// [`sync`](Self::sync).
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidArguments`] if `stride < width`, if
    /// `stride * height` overflows `u64`, or if `buffer` is shorter
    /// than `stride * height` — these are checked by the wrapper before
    /// the FFI call. `ThorVG` itself additionally returns
    /// [`Error::InvalidArguments`] if any of `stride`, `width`, or
    /// `height` is zero, [`Error::InsufficientCondition`] if the canvas
    /// is currently rendering (not synced), and [`Error::NotSupported`]
    /// if the software engine is unavailable.
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
    ///
    /// # Errors
    ///
    /// Returns [`Error::InsufficientCondition`] if the canvas is
    /// currently rendering (call [`sync`](Self::sync) first), or
    /// [`Error::NotSupported`] if the GL engine is unavailable.
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
    ///
    /// # Errors
    ///
    /// Returns [`Error::InsufficientCondition`] if the canvas is
    /// currently rendering (call [`sync`](Self::sync) first), or
    /// [`Error::NotSupported`] if the WebGPU engine is unavailable.
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
