use alloc::ffi::CString;
use alloc::string::String;
use alloc::vec::Vec;

use crate::animation::Animation;
use crate::error::{Error, Result};
use thorvg_sys as sys;

/// Marker information for a Lottie animation segment.
#[derive(Debug, Clone)]
pub struct Marker {
    /// The marker name.
    pub name: String,
    /// The starting frame.
    pub begin: f32,
    /// The ending frame.
    pub end: f32,
}

/// Borrowed view of a Lottie audio layer's current playback state.
///
/// Passed to the closure registered with
/// [`LottieAnimation::set_audio_resolver`]. The borrow is valid only for
/// the duration of that callback — the underlying strings are owned by
/// thorvg and must not be retained past the call (copy out what you need).
pub struct AudioInfo<'a> {
    raw: &'a sys::Tvg_Audio_Info,
}

impl<'a> AudioInfo<'a> {
    /// The audio source as a file path or URL.
    ///
    /// Returns `None` for embedded audio (use [`embedded_data`](Self::embedded_data))
    /// or if the source is not valid UTF-8.
    pub fn source(&self) -> Option<&'a str> {
        if self.raw.embedded || self.raw.src.is_null() {
            return None;
        }
        // SAFETY: non-null C string owned by thorvg, valid for the
        // callback's duration (`'a`).
        unsafe { core::ffi::CStr::from_ptr(self.raw.src) }
            .to_str()
            .ok()
    }

    /// The embedded raw audio bytes, of length [`size`](Self::size).
    ///
    /// Returns `None` when the source is a path/URL rather than embedded
    /// data (see [`is_embedded`](Self::is_embedded)).
    pub fn embedded_data(&self) -> Option<&'a [u8]> {
        if !self.raw.embedded || self.raw.src.is_null() {
            return None;
        }
        // SAFETY: thorvg guarantees `src` points to `size` bytes of
        // embedded data when `embedded` is set; valid for `'a`.
        Some(unsafe {
            core::slice::from_raw_parts(self.raw.src.cast::<u8>(), self.raw.size as usize)
        })
    }

    /// MIME type of the embedded audio, when present.
    pub fn mime_type(&self) -> Option<&'a str> {
        if self.raw.mimeType.is_null() {
            return None;
        }
        // SAFETY: non-null C string owned by thorvg, valid for `'a`.
        unsafe { core::ffi::CStr::from_ptr(self.raw.mimeType) }
            .to_str()
            .ok()
    }

    /// Size of the embedded audio data in bytes (valid when [`is_embedded`](Self::is_embedded)).
    pub fn size(&self) -> u32 {
        self.raw.size
    }

    /// Playback position within the audio in seconds (valid when [`is_active`](Self::is_active)).
    pub fn offset(&self) -> f32 {
        self.raw.offset
    }

    /// Volume in the range `[0, 100]` (valid when [`is_active`](Self::is_active)).
    pub fn volume(&self) -> f32 {
        self.raw.volume
    }

    /// `true` while the layer is within its playback range.
    pub fn is_active(&self) -> bool {
        self.raw.active
    }

    /// `true` if the source is embedded raw audio data; `false` if it is
    /// a file path or URL.
    pub fn is_embedded(&self) -> bool {
        self.raw.embedded
    }
}

impl core::fmt::Debug for AudioInfo<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AudioInfo")
            .field("active", &self.is_active())
            .field("embedded", &self.is_embedded())
            .field("offset", &self.offset())
            .field("volume", &self.volume())
            .finish_non_exhaustive()
    }
}

/// A Lottie animation controller with Lottie-specific extensions.
///
/// This wraps [`Animation`] and adds slots, markers, tweening,
/// expression variables, and quality control.
///
/// All base [`Animation`] methods are available via `Deref`.
///
/// # Example
///
/// ```no_run
/// use thorvg::{Thorvg, ColorSpace};
///
/// // `Thorvg::init` takes a thread count under the default `threads` feature
/// // and takes no arguments when that feature is disabled.
/// let engine = Thorvg::init(0).unwrap();
/// let mut canvas = engine.sw_canvas(Default::default()).unwrap();
/// let mut buffer = vec![0u32; 800 * 600];
/// unsafe {
///     canvas
///         .set_target(&mut buffer, 800, 800, 600, ColorSpace::ABGR8888)
///         .unwrap()
/// };
///
/// let mut lottie = engine.lottie_animation().unwrap();
/// let pic = lottie.picture_mut();
/// pic.load_from_str("animation.json").ok();
/// ```
///
/// The lifetime `'eng` ties this animation to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create Lottie animations via
/// [`Thorvg::lottie_animation()`](crate::Thorvg::lottie_animation).
pub struct LottieAnimation<'eng> {
    inner: Animation<'eng>,
    /// Type-erased audio resolver, kept alive for the animation's
    /// lifetime.  The closure lives in its own `Box<F>` on the heap;
    /// `data` is a thin pointer to that allocation, which thorvg stores
    /// verbatim and feeds back to the monomorphized trampoline.  Moving
    /// the `LottieAnimation` does not invalidate the pointer because the
    /// closure stays at its heap address.  Declared **after** `inner` so
    /// the C animation handle is released before this box is freed.
    /// `None` until [`set_audio_resolver`](Self::set_audio_resolver) is called.
    audio_resolver: Option<ErasedAudioResolver>,
}

impl LottieAnimation<'_> {
    /// Creates a new Lottie animation object.
    pub(crate) fn new() -> Result<Self> {
        let raw = unsafe { sys::tvg_lottie_animation_new() };
        if raw.is_null() {
            return Err(Error::FailedAllocation);
        }
        Ok(Self {
            inner: unsafe { Animation::from_raw(raw) },
            audio_resolver: None,
        })
    }

    // ── Convenience loaders ──────────────────────────────────────────

    /// Load a Lottie animation from a JSON byte slice.
    ///
    /// Convenience wrapper around [`Picture::load_data`](crate::Picture::load_data) that uses the
    /// `"lottie"` mimetype and copies the data. For advanced use cases
    /// (e.g. external asset `resource_path`), access the picture
    /// directly via [`Animation::picture`].
    pub fn load_data(&mut self, data: &[u8]) -> Result<()> {
        let pic = self.picture_mut();
        pic.load_data(data, crate::picture::MimeType::Lottie, None)
    }

    /// Load a Lottie animation from a file path string.
    ///
    /// Convenience wrapper around [`Picture::load_from_str`](crate::Picture::load_from_str).
    #[cfg(feature = "std")]
    pub fn load_file(&mut self, path: &str) -> Result<()> {
        let pic = self.picture_mut();
        pic.load_from_str(path)
    }

    /// Set the render size of the animation picture.
    ///
    /// Convenience wrapper around [`Picture::set_size`](crate::Picture::set_size).
    pub fn set_size(&mut self, w: f32, h: f32) -> Result<()> {
        let pic = self.picture_mut();
        pic.set_size(w, h)
    }

    // ── Slots ──────────────────────────────────────────────────────

    /// Generates a new slot from the given Lottie slot data (JSON format).
    ///
    /// Returns the generated slot ID, or `None` on failure.
    pub fn gen_slot(&mut self, slot_json: &str) -> Option<u32> {
        let c_slot = CString::new(slot_json).ok()?;
        let id = unsafe { sys::tvg_lottie_animation_gen_slot(self.inner.raw(), c_slot.as_ptr()) };
        if id == 0 {
            None
        } else {
            Some(id)
        }
    }

    /// Applies a previously generated slot to the animation.
    ///
    /// Pass `0` to reset all slots.
    pub fn apply_slot(&mut self, id: u32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_lottie_animation_apply_slot(self.inner.raw(), id) })
    }

    /// Deletes a previously generated slot.
    pub fn del_slot(&mut self, id: u32) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_lottie_animation_del_slot(self.inner.raw(), id) })
    }

    // ── Markers ────────────────────────────────────────────────────

    /// Gets the number of markers in the animation.
    pub fn markers_count(&self) -> Result<u32> {
        let mut cnt: u32 = 0;
        Error::from_raw(unsafe {
            sys::tvg_lottie_animation_get_markers_cnt(self.inner.raw(), &raw mut cnt)
        })?;
        Ok(cnt)
    }

    /// Gets the marker name at the given index.
    pub fn marker_name(&self, idx: u32) -> Result<String> {
        let mut name_ptr: *const core::ffi::c_char = core::ptr::null();
        Error::from_raw(unsafe {
            sys::tvg_lottie_animation_get_marker(self.inner.raw(), idx, &raw mut name_ptr)
        })?;
        if name_ptr.is_null() {
            return Ok(String::new());
        }
        Ok(unsafe { core::ffi::CStr::from_ptr(name_ptr) }
            .to_string_lossy()
            .into_owned())
    }

    /// Gets full marker information (name, begin frame, end frame) at the given index.
    pub fn marker_info(&self, idx: u32) -> Result<Marker> {
        let mut name_ptr: *const core::ffi::c_char = core::ptr::null();
        let mut begin: f32 = 0.0;
        let mut end: f32 = 0.0;
        Error::from_raw(unsafe {
            sys::tvg_lottie_animation_get_marker_info(
                self.inner.raw(),
                idx,
                &raw mut name_ptr,
                &raw mut begin,
                &raw mut end,
            )
        })?;
        let name = if name_ptr.is_null() {
            String::new()
        } else {
            unsafe { core::ffi::CStr::from_ptr(name_ptr) }
                .to_string_lossy()
                .into_owned()
        };
        Ok(Marker { name, begin, end })
    }

    /// Returns all markers in the animation.
    pub fn markers(&self) -> Result<Vec<Marker>> {
        let count = self.markers_count()?;
        let mut markers = Vec::with_capacity(count as usize);
        for i in 0..count {
            markers.push(self.marker_info(i)?);
        }
        Ok(markers)
    }

    /// Specifies a playback segment by marker name.
    pub fn set_marker(&mut self, marker: &str) -> Result<()> {
        let c_marker = CString::new(marker)?;
        Error::from_raw(unsafe {
            sys::tvg_lottie_animation_set_marker(self.inner.raw(), c_marker.as_ptr())
        })
    }

    // ── Tweening ───────────────────────────────────────────────────

    /// Interpolates between two frames based on a progress value (0.0–1.0).
    pub fn tween(&mut self, from: f32, to: f32, progress: f32) -> Result<()> {
        Error::from_raw(unsafe {
            sys::tvg_lottie_animation_tween(self.inner.raw(), from, to, progress)
        })
    }

    // ── Quality ────────────────────────────────────────────────────

    /// Sets the quality level for Lottie effects (0–100).
    ///
    /// Lower values prioritize performance, higher values prioritize quality.
    pub fn set_quality(&mut self, value: u8) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_lottie_animation_set_quality(self.inner.raw(), value) })
    }

    // ── Audio ──────────────────────────────────────────────────────

    /// Installs an audio resolver for the animation's Lottie audio layers.
    ///
    /// thorvg invokes the closure whenever an audio layer's playback
    /// state changes during rendering, handing it an [`AudioInfo`]
    /// describing the current timeline state. The closure is responsible
    /// for driving an external audio engine — start/seek playback when
    /// [`AudioInfo::is_active`] is `true`, stop it when `false`. thorvg
    /// itself does not play audio.
    ///
    /// The closure is stored inside this `LottieAnimation` and lives for
    /// the animation's lifetime; calling `set_audio_resolver` again
    /// replaces (and drops) the previous closure.
    ///
    /// *Experimental:* the underlying C API is marked experimental by
    /// upstream and may change in a future `ThorVG` release.
    ///
    /// ```ignore
    /// lottie.set_audio_resolver(|info: &thorvg::AudioInfo| {
    ///     if info.is_active() {
    ///         player.play(info.source().unwrap(), info.offset(), info.volume());
    ///     } else {
    ///         player.stop();
    ///     }
    /// })?;
    /// ```
    pub fn set_audio_resolver<F>(&mut self, resolver: F) -> Result<()>
    where
        F: FnMut(&AudioInfo<'_>) + Send + 'static,
    {
        // Detach any previous resolver from the C side first, so the old
        // Box is not freed while thorvg still holds its address.
        if self.audio_resolver.is_some() {
            unsafe {
                sys::tvg_lottie_animation_set_audio_resolver(
                    self.inner.raw(),
                    None,
                    core::ptr::null_mut(),
                );
            }
            self.audio_resolver = None;
        }
        // Heap-allocate the concrete closure and hand thorvg a thin
        // pointer to it.  The monomorphized `audio_trampoline::<F>` casts
        // the pointer back to `*mut F` and calls `F` directly — no `dyn`,
        // no double box.
        let boxed: alloc::boxed::Box<F> = alloc::boxed::Box::new(resolver);
        let raw_f: *mut F = alloc::boxed::Box::into_raw(boxed);
        // SAFETY: `Box::into_raw` never returns null.
        let data = unsafe { core::ptr::NonNull::new_unchecked(raw_f.cast::<()>()) };
        self.audio_resolver = Some(ErasedAudioResolver {
            data,
            drop_fn: drop_audio_resolver::<F>,
        });
        // SAFETY: `data` references a heap allocation owned by
        // `self.audio_resolver`; `Drop` unregisters the resolver before
        // that allocation is freed, so C never dereferences a dangling
        // pointer.
        Error::from_raw(unsafe {
            sys::tvg_lottie_animation_set_audio_resolver(
                self.inner.raw(),
                Some(audio_trampoline::<F>),
                data.as_ptr().cast::<core::ffi::c_void>(),
            )
        })
    }

    /// Removes any previously installed audio resolver.
    pub fn clear_audio_resolver(&mut self) -> Result<()> {
        let r = Error::from_raw(unsafe {
            sys::tvg_lottie_animation_set_audio_resolver(
                self.inner.raw(),
                None,
                core::ptr::null_mut(),
            )
        });
        self.audio_resolver = None;
        r
    }
}

impl<'eng> core::ops::Deref for LottieAnimation<'eng> {
    type Target = Animation<'eng>;

    fn deref(&self) -> &Animation<'eng> {
        &self.inner
    }
}

impl<'eng> core::ops::DerefMut for LottieAnimation<'eng> {
    fn deref_mut(&mut self) -> &mut Animation<'eng> {
        &mut self.inner
    }
}

impl core::fmt::Debug for LottieAnimation<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LottieAnimation").finish_non_exhaustive()
    }
}

impl Drop for LottieAnimation<'_> {
    fn drop(&mut self) {
        // Unregister the resolver from the C side BEFORE the `inner`
        // animation handle is freed (and before our resolver Box drops).
        // thorvg holds a pointer into our Box; letting it drop first
        // would leave thorvg dereferencing freed memory on any further
        // audio resolution.
        if self.audio_resolver.is_some() {
            unsafe {
                sys::tvg_lottie_animation_set_audio_resolver(
                    self.inner.raw(),
                    None,
                    core::ptr::null_mut(),
                );
            }
        }
    }
}

/// Owning, type-erased handle to a heap-allocated audio-resolver closure.
///
/// Mirrors the resolver-storage scheme used by `Picture`: each
/// `set_audio_resolver::<F>` call monomorphizes its own trampoline that
/// casts the thin `data` pointer back to `*mut F` directly, and `drop_fn`
/// reconstructs the original `Box<F>` so the right `Drop` runs — avoiding
/// the `Box<Box<dyn ...>>` double-indirection a `dyn` resolver needs.
struct ErasedAudioResolver {
    /// Thin pointer to a heap-allocated `F` produced by `Box::into_raw`.
    data: core::ptr::NonNull<()>,
    /// Reconstructs and drops the original `Box<F>`.
    drop_fn: unsafe fn(core::ptr::NonNull<()>),
}

impl Drop for ErasedAudioResolver {
    fn drop(&mut self) {
        // SAFETY: `data` was produced by `Box::<F>::into_raw` and
        // `drop_fn` was set to `drop_audio_resolver::<F>` for the same
        // `F` at construction time, so the cast inside is sound.
        unsafe { (self.drop_fn)(self.data) }
    }
}

// SAFETY: `set_audio_resolver` requires `F: Send`, so the boxed closure
// behind `data` is `Send`.  `drop_fn` is a plain function pointer.
unsafe impl Send for ErasedAudioResolver {}

/// Monomorphized destructor used by [`ErasedAudioResolver`] to drop the
/// original `Box<F>` behind the erased `data` pointer.
unsafe fn drop_audio_resolver<F>(data: core::ptr::NonNull<()>) {
    // SAFETY: caller guarantees `data` originated from `Box::<F>::into_raw`
    // for this same `F`.
    drop(unsafe { alloc::boxed::Box::from_raw(data.as_ptr().cast::<F>()) });
}

/// FFI trampoline bridging thorvg's C audio callback to the boxed Rust
/// closure stored in [`LottieAnimation::audio_resolver`].  Monomorphized
/// on the concrete closure type `F`: `data` is the thin `*mut F` produced
/// by [`LottieAnimation::set_audio_resolver`].
unsafe extern "C" fn audio_trampoline<F>(
    info: *const sys::Tvg_Audio_Info,
    data: *mut core::ffi::c_void,
) where
    F: FnMut(&AudioInfo<'_>) + Send + 'static,
{
    if data.is_null() || info.is_null() {
        return;
    }
    let f = unsafe { &mut *data.cast::<F>() };
    // SAFETY: `info` is non-null and valid for the duration of this call.
    let view = AudioInfo {
        raw: unsafe { &*info },
    };
    // A panic here would unwind across the C++ caller above us, which is
    // UB.  Catch it (see `invoke_audio`); in `no_std` builds the crate
    // requires `panic = "abort"`, so termination cannot cross the FFI
    // boundary.
    invoke_audio::<F>(f, &view);
}

#[cfg(feature = "std")]
fn invoke_audio<F>(f: &mut F, info: &AudioInfo<'_>)
where
    F: FnMut(&AudioInfo<'_>) + Send + 'static,
{
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(info)));
}

#[cfg(not(feature = "std"))]
fn invoke_audio<F>(f: &mut F, info: &AudioInfo<'_>)
where
    F: FnMut(&AudioInfo<'_>) + Send + 'static,
{
    // `no_std` users build with `panic = "abort"` (see crate docs); an
    // aborting panic cannot cross the FFI boundary, so no `catch_unwind`.
    f(info);
}
