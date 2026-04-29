//! Contains the [`Truncated`] error type and the [`TruncatedResultExt`] helper trait.

use core::fmt;

/// An error type indicating that a result was truncated.
///
/// This is used from [`WriteBuf`](crate::WriteBuf) functions in the `Result::Err` case. It only provides access
/// to the inner value; for the `WriteBuf` finish family, `Truncated<'_>` records the part of the string slice
/// that was validly written before truncation.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Truncated<'a>(pub(crate) &'a str);

impl<'a> Truncated<'a> {
    /// Get the inner string slice.
    #[must_use]
    pub fn get(&self) -> &'a str {
        self.0
    }
}

impl fmt::Debug for Truncated<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Truncated({:?})", self.0)
    }
}

impl fmt::Display for Truncated<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl core::error::Error for Truncated<'_> {}

mod sealed {
    pub trait Sealed {}
}

/// Extension trait for [`Result<&str, Truncated>`] providing uniform access to the written content
/// regardless of whether truncation occurred.
///
/// The [`WriteBuf`](crate::WriteBuf) `finish` family returns `Result<&'a str, Truncated<'a>>` where both arms
/// carry the successfully-written `&str`. This trait exposes that content directly, plus convenience accessors
/// for the byte length and the truncation flag.
///
/// ```
/// use fmtbuf::{TruncatedResultExt, WriteBuf};
/// use core::fmt::Write;
///
/// // Ok arm: plenty of space.
/// let mut buf = [0u8; 16];
/// let mut writer = WriteBuf::new(&mut buf);
/// write!(writer, "hi").unwrap();
/// let result = writer.finish();
/// assert_eq!(result.written(), "hi");
/// assert_eq!(result.written_len(), 2);
/// assert!(!result.is_truncated());
///
/// // Err arm: truncated write.
/// let mut buf = [0u8; 4];
/// let mut writer = WriteBuf::new(&mut buf);
/// let _ = write!(writer, "hello");
/// let result = writer.finish();
/// assert_eq!(result.written(), "hell");
/// assert_eq!(result.written_len(), 4);
/// assert!(result.is_truncated());
/// ```
pub trait TruncatedResultExt<'a>: sealed::Sealed {
    /// Get the successfully-written content, regardless of whether truncation occurred.
    fn written(&self) -> &'a str;

    /// Get the byte length of the successfully-written content.
    fn written_len(&self) -> usize {
        self.written().len()
    }

    /// Get whether truncation occurred during the underlying write.
    fn is_truncated(&self) -> bool;
}

impl<'a> sealed::Sealed for Result<&'a str, Truncated<'a>> {}

impl<'a> TruncatedResultExt<'a> for Result<&'a str, Truncated<'a>> {
    fn written(&self) -> &'a str {
        match self {
            Ok(s) => s,
            Err(t) => t.get(),
        }
    }

    fn is_truncated(&self) -> bool {
        self.is_err()
    }
}
