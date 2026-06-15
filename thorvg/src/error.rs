//! Error and result types for `ThorVG` operations.
//!
//! Wraps the [`ThorVG` C API](https://www.thorvg.org/c-native).

use core::fmt;
use thorvg_sys as sys;

/// Specialized [`Result`](core::result::Result) for `ThorVG` operations.
///
/// Aliases `core::result::Result<T, `[`Error`]`>`; every fallible
/// operation in this crate returns it.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors returned by `ThorVG` operations.
///
/// Each variant maps one-to-one onto a `ThorVG` `Tvg_Result` status
/// code; the per-variant docs describe the conditions `ThorVG` reports
/// it under. The enum is `#[non_exhaustive]`: match with a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Error {
    /// An argument was rejected, e.g. an empty path or a null pointer.
    InvalidArguments,
    /// The request cannot be processed in the object's current state,
    /// e.g. querying a property that has not been set.
    InsufficientCondition,
    /// A memory allocation failed.
    FailedAllocation,
    /// Bad memory handling was detected, e.g. a failed pointer release
    /// or cast.
    MemoryCorruption,
    /// An unsupported engine feature or option was requested.
    NotSupported,
    /// `ThorVG` reported a failure that does not map to a more specific
    /// variant.
    Unknown,
}

impl Error {
    /// Converts a raw `Tvg_Result` status code into a [`Result`].
    pub(crate) fn from_raw(result: sys::Tvg_Result) -> Result<()> {
        match result {
            sys::Tvg_Result::TVG_RESULT_SUCCESS => Ok(()),
            sys::Tvg_Result::TVG_RESULT_INVALID_ARGUMENT => Err(Error::InvalidArguments),
            sys::Tvg_Result::TVG_RESULT_INSUFFICIENT_CONDITION => Err(Error::InsufficientCondition),
            sys::Tvg_Result::TVG_RESULT_FAILED_ALLOCATION => Err(Error::FailedAllocation),
            sys::Tvg_Result::TVG_RESULT_MEMORY_CORRUPTION => Err(Error::MemoryCorruption),
            sys::Tvg_Result::TVG_RESULT_NOT_SUPPORTED => Err(Error::NotSupported),
            sys::Tvg_Result::TVG_RESULT_UNKNOWN => Err(Error::Unknown),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidArguments => write!(f, "invalid arguments"),
            Error::InsufficientCondition => write!(f, "insufficient condition"),
            Error::FailedAllocation => write!(f, "failed allocation"),
            Error::MemoryCorruption => write!(f, "memory corruption"),
            Error::NotSupported => write!(f, "not supported"),
            Error::Unknown => write!(f, "unknown error"),
        }
    }
}

impl From<alloc::ffi::NulError> for Error {
    /// Maps a [`NulError`](alloc::ffi::NulError) to
    /// [`Error::InvalidArguments`].
    ///
    /// An interior NUL byte makes a string unusable as a C string, so
    /// it is reported as an invalid argument. This lets
    /// `CString::new(s)?` propagate directly without a manual
    /// `map_err`.
    fn from(_: alloc::ffi::NulError) -> Self {
        Error::InvalidArguments
    }
}

impl core::error::Error for Error {}
