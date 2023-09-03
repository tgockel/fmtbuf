//! # `fmtbuf`
//! This library is intended to help write formatted text to fixed buffers.
//!
//! ```
//! use fmtbuf::WriteBuf;
//! use std::fmt::Write;
//!
//! let mut buf: [u8; 10] = [0; 10];
//! let mut writer = WriteBuf::new(&mut buf);
//! if let Err(e) = write!(&mut writer, "🚀🚀🚀") {
//!     println!("write error: {e:?}");
//! }
//! let written_len = match writer.finish_with_or("!", "…") {
//!     Ok(len) => len, // <- won't be hit since 🚀🚀🚀 is 12 bytes
//!     Err(len) => {
//!         println!("writing was truncated");
//!         len
//!     }
//! };
//! let written = &buf[..written_len];
//! assert_eq!("🚀…", std::str::from_utf8(written).unwrap());
//! ```
//!
//! A few things happened in that example:
//!
//! 1. We stared with a 10 byte buffer
//! 2. Tried to write `"🚀🚀🚀"` to it, which is encoded as 3 `b"\xf0\x9f\x9a\x80"`s (12 bytes)
//! 3. This can't fit into 10 bytes, so only `"🚀🚀"` is stored and the `writer` is noted as having truncated writes
//! 4. We finish the buffer with `"!"` on success or `"…"` (a.k.a. `b"\xe2\x80\xa6"`) on truncation
//! 5. Since we noted truncation in step #3, we try to write `"…"`, but this can not fit into the buffer either, since
//!    8 (`"🚀🚀".len()`) + 3 (`"…".len()`) > 12 (`buf.len()`)
//! 6. Roll the buffer back to the end of the first 🚀, then add …, leaving us with `"🚀…"`

#![cfg_attr(not(feature = "std"), no_std)]

mod utf8;

use core::fmt;

#[deprecated]
pub use utf8::rfind_utf8_end;

/// A write buffer pointing to a `&mut [u8]`.
///
/// ```
/// use fmtbuf::WriteBuf;
/// use std::fmt::Write;
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
/// let write_len = writer.finish().unwrap();
/// let written = std::str::from_utf8(&buf[..write_len]).unwrap();
/// assert_eq!(written, "some data: 420");
/// ```
pub struct WriteBuf<'a> {
    target: &'a mut [u8],
    position: usize,
    truncated: bool,
}

impl<'a> WriteBuf<'a> {
    /// Create an instance that will write to the given `target`. The contents of the target do not need to have been
    /// initialized before this, as they will be overwritten by writing.
    pub fn new(target: &'a mut [u8]) -> Self {
        Self {
            target,
            position: 0,
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

    /// # Returns
    ///
    /// In both the `Ok` and `Err` cases, the [`WriteBuf::position`] is returned. The `Ok` case indicates the truncation
    /// did not occur, while `Err` indicates that it did.
    pub fn finish(self) -> Result<usize, usize> {
        if self.truncated {
            Err(self.position)
        } else {
            Ok(self.position)
        }
    }

    /// Finish the buffer, adding the `suffix` to the end. A common use case for this is to add a null terminator.
    ///
    /// This operates slightly differently than the normal format writing function `write_str` in that the `suffix` is
    /// always put at the end. The only case where this will not happen is when `suffix.len()` is less than the size of
    /// the buffer originally provided. In this case, the last bit of `suffix` will be copied (starting at a valid UTF-8
    /// sequence start; e.g.: writing `"🚀..."` to a 5 byte buffer will leave you with just `"..."`, no matter what was
    /// written before).
    ///
    /// ```
    /// use fmtbuf::WriteBuf;
    ///
    /// let mut buf: [u8; 4] = [0xff; 4];
    /// let mut writer = WriteBuf::new(&mut buf);
    ///
    /// // Finish writing with too many bytes:
    /// let write_len = writer.finish_with("12345").unwrap_err();
    /// assert_eq!(write_len, 4);
    /// let buf_str = std::str::from_utf8(&buf).unwrap();
    /// assert_eq!(buf_str, "2345");
    /// ```
    ///
    /// # Returns
    ///
    /// The returned value has the same meaning as [`WriteBuf::finish`].
    pub fn finish_with(self, suffix: impl AsRef<[u8]>) -> Result<usize, usize> {
        let suffix = suffix.as_ref();
        self._finish_with(suffix, suffix)
    }

    /// Finish the buffer by adding `normal_suffix` if not truncated or `truncated_suffix` if the buffer will be
    /// truncated. This operates the same as [`WriteBuf::finish_with`] in every other way.
    pub fn finish_with_or(
        self,
        normal_suffix: impl AsRef<[u8]>,
        truncated_suffix: impl AsRef<[u8]>,
    ) -> Result<usize, usize> {
        self._finish_with(normal_suffix.as_ref(), truncated_suffix.as_ref())
    }

    fn _finish_with(mut self, normal: &[u8], truncated: &[u8]) -> Result<usize, usize> {
        let remaining = self.target.len() - self.position;

        // If the truncated case is shorter than the normal case, then writing it might still work
        for (suffix, should_test) in [(normal, !self.truncated), (truncated, true)] {
            if !should_test {
                continue;
            }

            // enough room in the buffer to write entire suffix, so just write it
            if suffix.len() <= remaining {
                self.target[self.position..self.position + suffix.len()].copy_from_slice(suffix);
                self.position += suffix.len();
                return if self.truncated {
                    Err(self.position)
                } else {
                    Ok(self.position)
                };
            }

            // we attempted to perform a write, but rejected it
            self.truncated = true;
        }

        let suffix = truncated;

        // if the suffix is larger than the entire target buffer, copy the last N
        if self.target.len() < suffix.len() {
            let copyable_suffix = &suffix[suffix.len() - self.target.len()..];
            let Some(valid_utf8_idx) = copyable_suffix
                .iter()
                .enumerate()
                .find(|(_, cu)| utf8::utf8_char_width(**cu).is_some())
                .map(|(idx, _)| idx)
            else {
                return Err(0);
            };
            let copyable_suffix = &copyable_suffix[valid_utf8_idx..];
            self.target[..copyable_suffix.len()].copy_from_slice(copyable_suffix);
            return Err(copyable_suffix.len());
        }

        // Scan backwards to find the position we should write to (can't interrupt a UTF-8 multibyte sequence)
        let potential_end_idx = self.target.len() - suffix.len();
        let write_idx = rfind_utf8_end(&self.target[..potential_end_idx]);
        self.target[write_idx..write_idx + suffix.len()].copy_from_slice(suffix);
        Err(write_idx + suffix.len())
    }

    fn append(&mut self, input: &[u8]) -> fmt::Result {
        let remaining = self.target.len() - self.position;
        if remaining == 0 {
            return Err(fmt::Error);
        }

        let input = if remaining >= input.len() {
            input
        } else {
            let to_write = &input[..remaining];
            self.truncated = true;
            &input[..rfind_utf8_end(to_write)]
        };

        self.target[self.position..self.position + input.len()].copy_from_slice(input);
        self.position += input.len();
        Ok(())
    }
}

impl<'a> fmt::Write for WriteBuf<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.truncated {
            return Err(fmt::Error);
        }

        self.append(s.as_bytes())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use core::fmt::Write;

    /// * `.0`: Input string
    /// * `.1`: The end position if the last byte was chopped off
    static TEST_CASES: &[(&str, usize)] = &[
        ("", 0),
        ("James", 4),
        ("_ø", 1),
        ("磨", 0),
        ("here: 见/見", 10),
        ("𨉟呐㗂越", 10),
        ("🚀", 0),
        ("🚀🚀🚀", 8),
        ("rocket: 🚀", 8),
    ];

    #[test]
    fn rfind_utf8_end_test() {
        for (input, last_valid_idx_after_cut) in TEST_CASES.iter() {
            let result = rfind_utf8_end(input.as_bytes());
            assert_eq!(result, input.len(), "input=\"{input}\"");
            if input.len() == 0 {
                continue;
            }
            let input_truncated = &input.as_bytes()[..input.len() - 1];
            let result = rfind_utf8_end(input_truncated);
            assert_eq!(
                result, *last_valid_idx_after_cut,
                "input=\"{input}\" truncated={input_truncated:?}"
            );
        }
    }

    #[test]
    fn format_enough_space() {
        for (input, _) in TEST_CASES.iter() {
            let mut buf: [u8; 128] = [0xff; 128];
            let mut writer = WriteBuf::new(&mut buf);

            writer.write_str(input).unwrap();
            assert_eq!(input.len(), writer.position());
            let last_idx = writer.finish().unwrap();
            assert_eq!(input.len(), last_idx);
        }
    }

    #[test]
    fn format_truncation() {
        for (input, last_valid_idx_after_cut) in TEST_CASES.iter() {
            if input.len() == 0 {
                continue;
            }

            let mut buf: [u8; 128] = [0xff; 128];
            let mut writer = WriteBuf::new(&mut buf[..input.len() - 1]);

            writer.write_str(input).unwrap();
            assert_eq!(*last_valid_idx_after_cut, writer.position());
            let last_idx = writer.finish().unwrap_err();
            assert_eq!(*last_valid_idx_after_cut, last_idx);
        }
    }

    struct SimpleString {
        storage: [u8; 128],
        size: usize,
    }

    impl SimpleString {
        pub fn from_segments(segments: &[&str]) -> Self {
            let mut out = Self {
                storage: [0; 128],
                size: 0,
            };
            for segment in segments {
                out.append(segment);
            }
            out
        }

        pub fn append(&mut self, value: &str) {
            let value = value.as_bytes();
            self.storage[self.size..self.size + value.len()].copy_from_slice(value);
            self.size += value.len();
        }

        pub fn as_str(&self) -> &str {
            core::str::from_utf8(&self.storage[..self.size]).unwrap()
        }
    }

    impl From<&str> for SimpleString {
        fn from(value: &str) -> Self {
            let value = value.as_bytes();
            let mut storage = [0; 128];
            storage[..value.len()].copy_from_slice(value);
            Self {
                storage,
                size: value.len(),
            }
        }
    }

    #[test]
    fn finish_with_enough_space() {
        for (input, _) in TEST_CASES.iter() {
            let mut buf: [u8; 128] = [0xff; 128];
            let mut writer = WriteBuf::new(&mut buf);

            writer.write_str(input).unwrap();
            let position = writer.finish_with(b".123").unwrap();
            assert_eq!(position, input.len() + 4);
            let expected_written = SimpleString::from_segments(&[input, ".123"]);
            let actually_wriiten = core::str::from_utf8(&buf[..position]).unwrap();
            assert_eq!(expected_written.as_str(), actually_wriiten);
        }
    }

    #[test]
    fn finish_with_overwrite() {
        for (input, last_valid_idx_after_cut) in TEST_CASES.iter() {
            if input.len() == 0 {
                continue;
            }

            let mut buf: [u8; 128] = [0xff; 128];
            let mut writer = WriteBuf::new(&mut buf[..input.len()]);

            writer.write_str(input).unwrap();
            let position = writer.finish_with("?").unwrap_err();
            assert_eq!(position, last_valid_idx_after_cut + 1);
            let expected_written = SimpleString::from_segments(&[
                core::str::from_utf8(&input.as_bytes()[..*last_valid_idx_after_cut]).unwrap(),
                "?",
            ]);
            let actually_wriiten = core::str::from_utf8(&buf[..position]).unwrap();
            assert_eq!(expected_written.as_str(), actually_wriiten);
        }
    }

    #[test]
    fn finish_with_or_with_longer_normal_closer() {
        let mut buf: [u8; 4] = [0xff; 4];
        let writer = WriteBuf::new(&mut buf);

        let written = writer.finish_with_or("0123456789", "abc").unwrap_err();
        assert_eq!(written, 3);
        assert_eq!("abc", core::str::from_utf8(&buf[..written]).unwrap());
    }

    #[test]
    fn finish_with_full_overwrite_utf8() {
        let mut buf: [u8; 4] = [0xff; 4];
        let writer = WriteBuf::new(&mut buf);

        let written = writer.finish_with("🚀12").unwrap_err();
        assert_eq!(written, 2);
        assert_eq!("12", core::str::from_utf8(&buf[..written]).unwrap());
    }
}
