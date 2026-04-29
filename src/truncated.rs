//! Contains the [`Truncated`] type error type.

use core::fmt;

/// An error type indicating that a result was truncated.
///
/// This is used from [`WriteBuf`] functions in the `Result::Err` case. It only provides access to the inner value;
/// for the `WriteBuf` finish family, `Truncated<&'_ str>` records the part of the string slice that was validly
/// written before truncation.
pub struct Truncated<T>(pub(crate) T);

impl<T> Truncated<T> {
    /// Get a reference to the inner value.
    pub fn get(&self) -> &T {
        &self.0
    }

    /// Take the inner value out of this wrapper.
    pub fn take(self) -> T {
        self.0
    }
}

impl<T: fmt::Debug> fmt::Debug for Truncated<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Truncated({:?})", self.0)
    }
}

impl<T: fmt::Display> fmt::Display for Truncated<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> core::error::Error for Truncated<T> where T: fmt::Debug + fmt::Display {}
