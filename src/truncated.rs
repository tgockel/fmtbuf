//! Contains the [`Truncated`] type error type.

use core::fmt;

/// An error type indicating that a result was truncated.
///
/// This is used from [`WriteBuf`] functions in the `Result::Err` case. It only provides access to the inner value;
/// for the `WriteBuf` finish family, `Truncated<'_>` records the part of the string slice that was validly
/// written before truncation.
pub struct Truncated<'a>(pub(crate) &'a str);

impl<'a> Truncated<'a> {
    /// Get the inner string slice.
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
