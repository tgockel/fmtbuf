#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

mod truncated;
mod utf8;

use core::fmt;

pub use truncated::Truncated;

use utf8::rfind_utf8_end;

/// A write buffer pointing to a `&mut [u8]`.
///
/// ```
/// use fmtbuf::WriteBuf;
/// use core::fmt::Write;
///
/// // The buffer to write into. The contents can be uninitialized, but using a
/// // bogus `\xff` sigil for demonstration.
/// let mut buf: [u8; 128] = [0xff; 128];
/// let mut writer = WriteBuf::new(&mut buf);
///
/// // Write data to the buffer.
/// write!(writer, "some data: {}", 0x01a4).unwrap();
///
/// // Finish writing:
/// let written = writer.finish().unwrap();
/// assert_eq!(written, "some data: 420");
/// ```
pub struct WriteBuf<'a> {
    target: &'a mut [u8],
    position: usize,
    reserve: usize,
    truncated: bool,
}

impl<'a> WriteBuf<'a> {
    /// Create an instance that will write to the given `target`. The contents of the target do not need to have been
    /// initialized before this, as they will be overwritten by writing.
    pub fn new(target: &'a mut [u8]) -> Self {
        Self {
            target,
            position: 0,
            reserve: 0,
            truncated: false,
        }
    }

    /// Create an instance that will write to the given `target` and `reserve` bytes at the end that will not be written
    /// be `write_str` operations.
    ///
    /// The use of this constructor is to note that `reserve` bytes will always be written at the end of the buffer (by
    /// the [`WriteBuf::finish_with`] family of functions), so `write_str` should not bother writing to it. This is
    /// useful when you know that you will always `finish_with` a null terminator or other character.
    ///
    /// It is allowed to have `target.len() < reserve`, but this can never be written to.
    ///
    /// ```
    /// use fmtbuf::WriteBuf;
    /// use core::fmt::Write;
    ///
    /// // Reserve one byte at the end for a null terminator.
    /// let mut buf: [u8; 8] = [0xff; 8];
    /// let mut writer = WriteBuf::with_reserve(&mut buf, 1);
    ///
    /// // Only 7 bytes are writable; the rest is held for the suffix.
    /// let _ = write!(writer, "abcdefgh");
    /// let written = writer.finish_with("\0").unwrap_err().take();
    /// assert_eq!(written, "abcdefg\0");
    /// ```
    pub fn with_reserve(target: &'a mut [u8], reserve: usize) -> Self {
        Self {
            target,
            position: 0,
            reserve,
            truncated: false,
        }
    }

    /// Get the position in the target buffer. The value is one past the end of written content and the next position to
    /// be written to.
    pub fn position(&self) -> usize {
        self.position
    }

    /// Get if a truncated write has happened.
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// Get the count of reserved bytes.
    pub fn reserve(&self) -> usize {
        self.reserve
    }

    /// Set the reserve bytes to `count`. If the written section has already encroached on the reserve space, this has
    /// no immediate effect, but it will prevent future writes. If [`WriteBuf::truncated`] has already been triggered,
    /// it will not be reset.
    pub fn set_reserve(&mut self, count: usize) {
        self.reserve = count;
    }

    /// Get the contents that have been written so far.
    pub fn written_bytes(&self) -> &[u8] {
        &self.target[..self.position]
    }

    /// Get the contents that have been written so far.
    pub fn written(&self) -> &str {
        // safety: The only way to write into the buffer is with valid UTF-8, so there is no reason to check the
        // contents for validity.
        unsafe { from_utf8_expect(self.written_bytes()) }
    }

    /// Finish writing to the buffer. This returns control of the target buffer to the caller (it is no longer mutably
    /// borrowed).
    ///
    /// # Returns
    ///
    /// In both the `Ok` and `Err` cases, the successfully-written portion of the output is returned as a `&str`. The
    /// `Ok` case indicates the truncation did not occur, while `Err` indicates that it did.
    pub fn finish(self) -> Result<&'a str, Truncated<&'a str>> {
        self.into_result()
    }

    fn into_result(self) -> Result<&'a str, Truncated<&'a str>> {
        // safety: The only way to write into the buffer is with valid UTF-8
        let written = unsafe { from_utf8_expect(&self.target[..self.position]) };
        if self.truncated {
            Err(Truncated(written))
        } else {
            Ok(written)
        }
    }

    /// Finish the buffer, adding the `suffix` to the end. A common use case for this is to add a null terminator.
    ///
    /// This operates slightly differently than the normal format writing function `write_str` in that the `suffix` is
    /// always put at the end. The only case where this will not happen is when `suffix.len()` exceeds the size of the
    /// buffer originally provided. In this case, the last bit of `suffix` will be copied (starting at a valid UTF-8
    /// sequence start; e.g.: writing `"🚀..."` to a 5 byte buffer will leave you with just `"..."`, no matter what was
    /// written before).
    ///
    /// ```
    /// use fmtbuf::WriteBuf;
    ///
    /// let mut buf: [u8; 4] = [0xff; 4];
    /// let writer = WriteBuf::new(&mut buf);
    ///
    /// // Finish writing with too many bytes:
    /// let written = writer.finish_with("12345").unwrap_err().take();
    /// assert_eq!(written, "2345");
    /// ```
    ///
    /// # Returns
    ///
    /// The returned value has the same meaning as [`WriteBuf::finish`].
    pub fn finish_with(self, suffix: impl AsRef<str>) -> Result<&'a str, Truncated<&'a str>> {
        let suffix = suffix.as_ref();
        self._finish_with(suffix, suffix)
    }

    /// Finish the buffer by adding `normal_suffix` if not truncated or `truncated_suffix` if the buffer will be
    /// truncated. This operates the same as [`WriteBuf::finish_with`] in every other way.
    ///
    /// ```
    /// use fmtbuf::WriteBuf;
    /// use core::fmt::Write;
    ///
    /// // Plenty of room: the normal suffix is appended.
    /// let mut buf: [u8; 8] = [0xff; 8];
    /// let mut writer = WriteBuf::new(&mut buf);
    /// write!(writer, "abc").unwrap();
    /// assert_eq!(writer.finish_with_or("!", "...").unwrap(), "abc!");
    ///
    /// // Doesn't fit: the truncated suffix is used instead.
    /// let mut buf: [u8; 4] = [0xff; 4];
    /// let mut writer = WriteBuf::new(&mut buf);
    /// let _ = write!(writer, "abcdef");
    /// assert_eq!(writer.finish_with_or("!", "...").unwrap_err().take(), "a...");
    /// ```
    pub fn finish_with_or(
        self,
        normal_suffix: impl AsRef<str>,
        truncated_suffix: impl AsRef<str>,
    ) -> Result<&'a str, Truncated<&'a str>> {
        self._finish_with(normal_suffix.as_ref(), truncated_suffix.as_ref())
    }

    fn _finish_with(mut self, normal: &str, truncated: &str) -> Result<&'a str, Truncated<&'a str>> {
        let remaining = self.target.len() - self.position();

        // If the truncated case is shorter than the normal case, then writing it might still work
        for (suffix, should_test) in [(normal, !self.truncated), (truncated, true)] {
            if !should_test {
                continue;
            }

            // enough room in the buffer to write entire suffix, so just write it
            if suffix.len() <= remaining {
                self.target[self.position..self.position + suffix.len()].copy_from_slice(suffix.as_bytes());
                self.position += suffix.len();
                return self.into_result();
            }

            // we attempted to perform a write, but rejected it
            self.truncated = true;
        }

        let suffix = truncated;

        // if the suffix is larger than the entire target buffer, copy the last N
        if self.target.len() < suffix.len() {
            let suffix_bytes = suffix.as_bytes();
            let copyable_suffix = &suffix_bytes[suffix.len() - self.target.len()..];
            let Some(valid_utf8_idx) = copyable_suffix
                .iter()
                .enumerate()
                .find(|(_, cu)| utf8::utf8_char_width(**cu).is_some())
                .map(|(idx, _)| idx)
            else {
                self.position = 0;
                self.truncated = true;
                return self.into_result();
            };
            let copyable_suffix = &copyable_suffix[valid_utf8_idx..];
            self.target[..copyable_suffix.len()].copy_from_slice(copyable_suffix);
            self.position = copyable_suffix.len();
            self.truncated = true;
            return self.into_result();
        }

        // Scan backwards to find the position we should write to (can't interrupt a UTF-8 multibyte sequence)
        let potential_end_idx = self.target.len() - suffix.len();
        let write_idx = rfind_utf8_end(&self.target[..potential_end_idx]);
        self.target[write_idx..write_idx + suffix.len()].copy_from_slice(suffix.as_bytes());
        self.position = write_idx + suffix.len();
        self.truncated = true;
        self.into_result()
    }

    fn _write(&mut self, input: &[u8]) -> fmt::Result {
        if self.truncated() {
            return Err(fmt::Error);
        }

        let remaining = self.target.len() - self.position();
        if remaining < self.reserve() {
            self.truncated = true;
            return Err(fmt::Error);
        }
        let remaining = remaining - self.reserve();

        let (input, result) = if remaining >= input.len() {
            (input, Ok(()))
        } else {
            let to_write = &input[..remaining];
            self.truncated = true;
            (&input[..rfind_utf8_end(to_write)], Err(fmt::Error))
        };

        self.target[self.position..self.position + input.len()].copy_from_slice(input);
        self.position += input.len();

        result
    }
}

impl<'a> fmt::Write for WriteBuf<'a> {
    /// Append `s` to the target buffer.
    ///
    /// # Error
    ///
    /// An error is returned if the entirety of `s` can not fit in the target buffer or if a previous `write_str`
    /// operation failed. If this occurs, as much as `s` that can fit into the buffer will be written up to the last
    /// valid Unicode code point. In other words, if the target buffer have 6 writable bytes left and `s` is the two
    /// code points `"♡🐶"` (a.k.a.: the 7 byte `b"\xe2\x99\xa1\xf0\x9f\x90\xb6"`), then only `♡` will make it to the
    /// output buffer, making the target of your ♡ ambiguous.
    ///
    /// Truncation marks this buffer as truncated, which can be observed with [`WriteBuf::truncated`]. Future write
    /// attempts will immediately return in `Err`. This also affects the behavior of [`WriteBuf::finish`] family of
    /// functions, which will always return the `Err` case to indicate truncation. For [`WriteBuf::finish_with_or`],
    /// the `normal_suffix` will not be attempted.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self._write(s.as_bytes())
    }
}

/// Extract a `&str` from source `&[u8]`, expecting it to be valid UTF-8.
///
/// # Safety
///
/// Under the covers, this calls `str::from_utf8` if debug assertions are enabled, otherwise it calls
/// `str::from_utf8_unchecked`. This means `src` should always be valid UTF-8.
unsafe fn from_utf8_expect(src: &[u8]) -> &str {
    #[cfg(debug_assertions)]
    return core::str::from_utf8(src).expect("buffer should have been valid UTF-8");

    // safety: The contents are valid UTF-8 by construction; debug builds verify the invariant via the
    // `cfg(debug_assertions)` branch above.
    #[cfg(not(debug_assertions))]
    unsafe {
        core::str::from_utf8_unchecked(src)
    }
}

#[cfg(test)]
mod test;
