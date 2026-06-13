use core::fmt;
use thorvg_sys as sys;

/// Result type for `ThorVG` operations.
pub type Result<T> = core::result::Result<T, Error>;

/// Errors returned by `ThorVG` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Error {
    /// Invalid arguments were provided.
    InvalidArguments,
    /// The request cannot be processed due to insufficient conditions.
    InsufficientCondition,
    /// Memory allocation failed.
    FailedAllocation,
    /// Memory corruption detected.
    MemoryCorruption,
    /// The requested feature is not supported.
    NotSupported,
    /// An unknown error occurred.
    Unknown,
}

impl Error {
    /// Convert a raw `Tvg_Result` into a `Result<()>`.
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
    /// An interior NUL byte makes the string unusable as a C string,
    /// so it is reported as [`Error::InvalidArguments`].  This lets
    /// `CString::new(s)?` propagate directly without a manual
    /// `map_err`.
    fn from(_: alloc::ffi::NulError) -> Self {
        Error::InvalidArguments
    }
}

impl core::error::Error for Error {}
