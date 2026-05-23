use alloc::ffi::CString;
use alloc::string::String;

use crate::error::{Error, Result};
use crate::paint::Paint;
use thorvg_sys as ffi;

/// Scene tree traversal helper.
///
/// Iterates through all descendants of a paint (scene) and invokes a callback on each.
pub struct Accessor {
    raw: ffi::Tvg_Accessor,
}

impl Accessor {
    /// Creates a new Accessor object.
    pub fn new() -> Self {
        let raw = unsafe { ffi::tvg_accessor_new() };
        assert!(!raw.is_null(), "failed to create Accessor");
        Self { raw }
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
        func: unsafe extern "C" fn(ffi::Tvg_Paint, *mut core::ffi::c_void) -> bool,
        data: *mut core::ffi::c_void,
    ) -> Result<()> {
        Error::from_raw(unsafe { ffi::tvg_accessor_set(self.raw, paint.raw(), Some(func), data) })
    }

    /// Iterates through all descendants, calling a safe Rust closure on each.
    ///
    /// The closure receives the raw `Tvg_Paint` handle and returns `true` to
    /// continue or `false` to stop.
    pub fn for_each<P: Paint, F: FnMut(ffi::Tvg_Paint) -> bool>(
        &mut self,
        paint: &P,
        mut func: F,
    ) -> Result<()> {
        unsafe extern "C" fn trampoline<F: FnMut(ffi::Tvg_Paint) -> bool>(
            paint: ffi::Tvg_Paint,
            data: *mut core::ffi::c_void,
        ) -> bool {
            let f = unsafe { &mut *data.cast::<F>() };
            f(paint)
        }

        let data = core::ptr::from_mut(&mut func).cast::<core::ffi::c_void>();
        Error::from_raw(unsafe {
            ffi::tvg_accessor_set(self.raw, paint.raw(), Some(trampoline::<F>), data)
        })
    }

    /// Generates a unique ID (hash key) from a name string.
    ///
    /// Use this to assign or look up paint IDs.
    pub fn generate_id(name: &str) -> Option<u32> {
        let c_name = CString::new(name).ok()?;
        Some(unsafe { ffi::tvg_accessor_generate_id(c_name.as_ptr()) })
    }

    /// Retrieves the original name string for a given unique ID.
    ///
    /// Only valid when `Picture::set_accessible(true)` has been called and
    /// this accessor is currently iterating via [`Accessor::set`] or
    /// [`Accessor::for_each`].
    pub fn get_name(&self, id: u32) -> Option<String> {
        let ptr = unsafe { ffi::tvg_accessor_get_name(self.raw, id) };
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

impl Default for Accessor {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Accessor {
    fn drop(&mut self) {
        unsafe {
            ffi::tvg_accessor_del(self.raw);
        }
    }
}
