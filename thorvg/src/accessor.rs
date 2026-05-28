use alloc::ffi::CString;
use alloc::string::String;

use crate::error::{Error, Result};
use crate::paint::Paint;
use thorvg_sys as sys;

/// Scene tree traversal helper.
///
/// Iterates through all descendants of a paint (scene) and invokes a callback on each.
///
/// The lifetime `'eng` ties this accessor to a [`Thorvg`](crate::Thorvg) engine
/// instance. Create accessors via [`Thorvg::accessor()`](crate::Thorvg::accessor).
pub struct Accessor<'eng> {
    raw: sys::Tvg_Accessor,
    _engine: core::marker::PhantomData<&'eng ()>,
}

impl Accessor<'_> {
    /// Creates a new Accessor object.
    pub(crate) fn new() -> Self {
        let raw = unsafe { sys::tvg_accessor_new() };
        assert!(!raw.is_null(), "failed to create Accessor");
        Self {
            raw,
            _engine: core::marker::PhantomData,
        }
    }

    /// Iterates through all descendants of `paint`, calling `func` on each.
    ///
    /// When `func` returns `false`, iteration stops. The `data` pointer is passed
    /// through to every invocation.
    ///
    /// # Safety
    /// `data` must remain valid for the duration of the iteration, and `func` must
    /// be safe to call with the paint handles provided by `ThorVG`.
    pub unsafe fn set<P: Paint>(
        &mut self,
        paint: &P,
        func: unsafe extern "C" fn(sys::Tvg_Paint, *mut core::ffi::c_void) -> bool,
        data: *mut core::ffi::c_void,
    ) -> Result<()> {
        Error::from_raw(unsafe { sys::tvg_accessor_set(self.raw, paint.raw(), Some(func), data) })
    }

    /// Iterates through all descendants, calling a safe Rust closure on each.
    ///
    /// The closure receives the raw `Tvg_Paint` handle and returns `true` to
    /// continue or `false` to stop.
    pub fn for_each<P: Paint, F: FnMut(sys::Tvg_Paint) -> bool>(
        &mut self,
        paint: &P,
        mut func: F,
    ) -> Result<()> {
        unsafe extern "C" fn trampoline<F: FnMut(sys::Tvg_Paint) -> bool>(
            paint: sys::Tvg_Paint,
            data: *mut core::ffi::c_void,
        ) -> bool {
            let f = unsafe { &mut *data.cast::<F>() };
            f(paint)
        }

        let data = core::ptr::from_mut(&mut func).cast::<core::ffi::c_void>();
        Error::from_raw(unsafe {
            sys::tvg_accessor_set(self.raw, paint.raw(), Some(trampoline::<F>), data)
        })
    }

    /// Generates a unique ID (hash key) from a name string.
    ///
    /// Use this to assign or look up paint IDs.
    pub fn generate_id(name: &str) -> Option<u32> {
        let c_name = CString::new(name).ok()?;
        Some(unsafe { sys::tvg_accessor_generate_id(c_name.as_ptr()) })
    }

    /// Retrieves the original name string for a given unique ID.
    ///
    /// Only valid when `Picture::set_accessible(true)` has been called and
    /// this accessor is currently iterating via [`Accessor::set`] or
    /// [`Accessor::for_each`].
    pub fn get_name(&self, id: u32) -> Option<String> {
        let ptr = unsafe { sys::tvg_accessor_get_name(self.raw, id) };
        if ptr.is_null() {
            None
        } else {
            Some(
                unsafe { core::ffi::CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }
    }
}

impl Drop for Accessor<'_> {
    fn drop(&mut self) {
        unsafe {
            sys::tvg_accessor_del(self.raw);
        }
    }
}

impl core::fmt::Debug for Accessor<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Accessor").finish_non_exhaustive()
    }
}
