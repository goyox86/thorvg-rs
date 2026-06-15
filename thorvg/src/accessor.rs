//! Scene-tree traversal for inspecting node structure and names.
//!
//! Wraps the [`ThorVG` C API](https://www.thorvg.org/c-native).

use alloc::ffi::CString;
use alloc::string::String;

use crate::error::{Error, Result};
use crate::paint::{BorrowedPaint, Paint};
use thorvg_sys as sys;

/// Scene-tree traversal helper.
///
/// Walks every descendant of a scene paint and invokes a closure on each,
/// for inspecting structure and node properties. Create accessors via
/// [`Thorvg::accessor()`](crate::Thorvg::accessor).
///
/// The lifetime `'eng` ties the accessor to a [`Thorvg`](crate::Thorvg)
/// engine instance.
pub struct Accessor<'eng> {
    raw: sys::Tvg_Accessor,
    _engine: core::marker::PhantomData<&'eng ()>,
}

impl Accessor<'_> {
    /// Creates a new Accessor object.
    pub(crate) fn new() -> Result<Self> {
        let raw = unsafe { sys::tvg_accessor_new() };
        if raw.is_null() {
            return Err(Error::FailedAllocation);
        }
        Ok(Self {
            raw,
            _engine: core::marker::PhantomData,
        })
    }

    /// Visits every descendant of `paint`, invoking `func` on each node.
    ///
    /// The closure receives:
    ///   * a [`BorrowedAccessor`] view over `self`, exposing
    ///     [`get_name`](BorrowedAccessor::get_name) — meaningful only
    ///     here, since the C side populates the id-to-name index for the
    ///     duration of this call;
    ///   * a [`BorrowedPaint`] view over the visited node, exposing
    ///     `id` / `paint_type` / `opacity` / `bounds`.
    ///
    /// Returning `false` from the closure stops iteration early; `true`
    /// continues to the next node.
    ///
    /// # Panics
    ///
    /// Does not panic. A panic inside `func` is caught at the FFI
    /// boundary (crossing it would be undefined behavior) and treated as
    /// a `false` return, stopping iteration; under `no_std` the crate
    /// requires `panic = "abort"`.
    ///
    /// # Errors
    ///
    /// Returns the [`Error`] mapped from `ThorVG`'s status if it rejects
    /// the traversal request; succeeds with `Ok(())` once the walk
    /// completes (including early stop).
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
            // SAFETY: panic across the C++ caller is UB.  Catch and
            // stop iteration (return `false`).  In `no_std` builds the
            // crate requires `panic = "abort"`, which terminates the
            // process before any unwinding could reach the FFI edge.
            invoke_user(&mut ctx.func, acc_view, paint_view)
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
    /// A pure hashing function: the name-to-id mapping is purely textual
    /// and involves no engine state. Use it to match a node's `id`
    /// against a known name during traversal. Returns `None` if `name`
    /// contains an interior NUL byte.
    pub fn generate_id(name: &str) -> Option<u32> {
        let c_name = CString::new(name).ok()?;
        Some(unsafe { sys::tvg_accessor_generate_id(c_name.as_ptr()) })
    }
}

#[cfg(feature = "std")]
fn invoke_user<F>(f: &mut F, acc: BorrowedAccessor<'_>, paint: BorrowedPaint<'_>) -> bool
where
    F: FnMut(BorrowedAccessor<'_>, BorrowedPaint<'_>) -> bool,
{
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(acc, paint))).unwrap_or(false)
}

#[cfg(not(feature = "std"))]
fn invoke_user<F>(f: &mut F, acc: BorrowedAccessor<'_>, paint: BorrowedPaint<'_>) -> bool
where
    F: FnMut(BorrowedAccessor<'_>, BorrowedPaint<'_>) -> bool,
{
    f(acc, paint)
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

impl BorrowedAccessor<'_> {
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
    /// Requires the iterated picture to have been marked accessible via
    /// [`Picture::set_accessible`](crate::Picture::set_accessible) with
    /// `true`. Returns `None` for ids not present in the active picture's
    /// name index, or when the name is unavailable. Only valid inside an
    /// [`Accessor::for_each`] callback.
    ///
    /// *Experimental in `ThorVG`; the API may change.*
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
