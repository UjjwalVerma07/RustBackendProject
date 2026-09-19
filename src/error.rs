//! Error type for the safe string-utilities API.
//!
//! One small, isolated enum captures the two ways the safe pipeline can fail: an
//! interior-NUL conversion failure (detected before the FFI layer is ever called) and a
//! null result from a string-producing C function (allocation failure, detected without
//! dereferencing the pointer).

use std::ffi::NulError;
use std::fmt;

/// The two failure modes of the safe API.
///
/// Both variants are constructed by the safe API: `Conversion` on the `?`-based
/// interior-NUL conversion path in `to_cstring`, and `NullResult` on the null-result check
/// after a string-producing FFI call.
#[derive(Debug)]
pub enum Error {
    /// The input `&str` contained an interior NUL byte and could not be converted to a C
    /// string. The FFI layer was not called. (Req 5.5)
    Conversion(NulError),
    /// A C_Library string-producing function returned a null pointer, indicating
    /// allocation failure. The pointer was not dereferenced. (Req 5.10)
    NullResult,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Conversion(e) => write!(f, "input contains an interior NUL byte: {e}"),
            Error::NullResult => write!(f, "C library returned null (allocation failure)"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Conversion(e) => Some(e),
            Error::NullResult => None,
        }
    }
}

impl From<NulError> for Error {
    fn from(e: NulError) -> Self {
        Error::Conversion(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;
    use std::ffi::CString;

    #[test]
    fn from_nul_error_produces_conversion_and_displays() {
        let nul_err = CString::new("a\0b").unwrap_err();
        let err = Error::from(nul_err);
        assert!(matches!(err, Error::Conversion(_)));
        let msg = err.to_string();
        assert!(
            msg.contains("interior NUL byte"),
            "unexpected Display output: {msg}"
        );
    }

    #[test]
    fn null_result_display_and_no_source() {
        let err = Error::NullResult;
        assert_eq!(
            err.to_string(),
            "C library returned null (allocation failure)"
        );
        assert!(err.source().is_none());
    }

    #[test]
    fn conversion_has_source() {
        let nul_err = CString::new("a\0b").unwrap_err();
        let err = Error::Conversion(nul_err);
        assert!(err.source().is_some());
    }
}
