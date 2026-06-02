use alloc::ffi::CString;
use alloc::string::String;

use crate::error::{Error, Result};
use crate::paint::{BorrowedPaint, Paint};
use thorvg_sys as sys;

/// Scene tree traversal helper.
///
/// Iterates through all descendants of a paint (scene) and invokes
/// a closure on each.  Create accessors via
/// [`Thorvg::accessor()`](crate::Thorvg::accessor).
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

    /// Iterates through every descendant of `paint`, invoking `func`
    /// on each visited node.
    ///
    /// The closure receives:
    ///   * a [`BorrowedAccessor`] view over `self`, exposing
    ///     `get_name(id)` — only meaningful here, since the C side
    ///     populates the id→name index only for the duration of
    ///     this call;
    ///   * a [`BorrowedPaint`] view over the visited node, exposing
    ///     `id` / `paint_type` / `opacity` / `bounds`.
    ///
    /// Returning `false` from the closure stops iteration early.
    pub fn for_each<P, F>(&mut self, paint: &P, func: F) -> Result<()>
    where
        P: Paint,
        F: FnMut(BorrowedAccessor<'_>, BorrowedPaint<'_>) -> bool,
    {
        struct Ctx<F> {
            func: F,
            acc_raw: sys::Tvg_Accessor,
        }

        unsafe extern "C" fn trampoline<F>(
            paint_raw: sys::Tvg_Paint,
            data: *mut core::ffi::c_void,
        ) -> bool
        where
            F: FnMut(BorrowedAccessor<'_>, BorrowedPaint<'_>) -> bool,
        {
            let ctx = unsafe { &mut *data.cast::<Ctx<F>>() };
            // SAFETY: BorrowedAccessor and BorrowedPaint are
            // synthesised here with lifetimes scoped to this
            // callback invocation — the C side guarantees both
            // handles are live for the duration of the call.
            let acc_view = unsafe { BorrowedAccessor::from_raw(ctx.acc_raw) };
            let paint_view = unsafe { BorrowedPaint::from_raw(paint_raw) };
            (ctx.func)(acc_view, paint_view)
        }

        let mut ctx = Ctx {
            func,
            acc_raw: self.raw,
        };
        let data = core::ptr::from_mut(&mut ctx).cast::<core::ffi::c_void>();
        Error::from_raw(unsafe {
            sys::tvg_accessor_set(self.raw, paint.raw(), Some(trampoline::<F>), data)
        })
    }

    /// Generates a unique ID (hash key) from a name string.
    ///
    /// Pure hashing function — no engine state involved; the name
    /// → id mapping is purely textual.
    pub fn generate_id(name: &str) -> Option<u32> {
        let c_name = CString::new(name).ok()?;
        Some(unsafe { sys::tvg_accessor_generate_id(c_name.as_ptr()) })
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

/// Read-only view of an [`Accessor`] passed into
/// [`for_each`](Accessor::for_each)'s closure.
///
/// `get_name` is only meaningful while iteration is active — the
/// C side populates the id→name index in `tvg_accessor_set` and
/// clears it on return.  Restricting the call to this view's
/// lifetime makes the misuse path uncallable.
pub struct BorrowedAccessor<'a> {
    raw: sys::Tvg_Accessor,
    _life: core::marker::PhantomData<&'a ()>,
}

impl<'a> BorrowedAccessor<'a> {
    /// # Safety
    /// `raw` must be a valid accessor handle currently inside an
    /// active iteration; the returned view borrows for `'a`.
    unsafe fn from_raw(raw: sys::Tvg_Accessor) -> Self {
        Self {
            raw,
            _life: core::marker::PhantomData,
        }
    }

    /// Looks up the original name string for a visited paint's id.
    ///
    /// Requires the iterated picture to have been marked accessible
    /// via `Picture::set_accessible(true)`.  Returns `None` for ids
    /// not present in the active picture's name index.
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

impl core::fmt::Debug for BorrowedAccessor<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BorrowedAccessor").finish_non_exhaustive()
    }
}
