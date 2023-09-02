//! # `fmtbuf`
//! This library is intended to help write formatted text to fixed buffers.

#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt;

/// Find the end of the last valid UTF-8 code point.
///
/// # Parameters
///
/// * `buf`: This should be an almost-valid UTF-8 encoded sequence. The final bytes can be a UTF-8 multi-byte sequence
///   which is incomplete.
///
/// # Returns
///
/// The number of code units which are valid UTF-8 (assuming `buf` adheres to the above specification).
///
/// ```
/// use fmtbuf::rfind_utf8_end;
///
/// assert!(rfind_utf8_end("1234".as_bytes()) == 4);
/// assert!(rfind_utf8_end("🚀".as_bytes()) == 4);
/// assert!(rfind_utf8_end(b"\xf0\x9f\x9a\x80") == 4); // "🚀" with the bytes written out
/// assert!(rfind_utf8_end(b"\xf0\x9f\x9a") == 0);     // "🚀" but missing the last byte
/// ```
pub fn rfind_utf8_end(buf: &[u8]) -> usize {
    let mut position = buf.len();
    // If the end of the string is middle of writing a UTF-8 multibyte sequence, we need to reverse to before the
    // code units for this incomplete code point.
    while position > 0 {
        position -= 1;

        // Keep scanning backwards until we find a code unit that is a valid start of a UTF-8 sequence; if we found one,
        // then `need_more` is the number of code units that multi-byte sequence should have.
        let need_more = match buf[position] {
            cu if cu & 0b1000_0000 == 0b0000_0000 => Some(1), // U+0000 .. U+007f (ASCII)
            cu if cu & 0b1100_0000 == 0b1000_0000 => None,    // UTF-8 continuation character
            cu if cu & 0b1110_0000 == 0b1100_0000 => Some(2), // U+0080 .. U+07ff
            cu if cu & 0b1111_0000 == 0b1110_0000 => Some(3), // U+0800 .. U+ffff
            cu if cu & 0b1111_1000 == 0b1111_0000 => Some(4), // U+10000 .. U+10ffff
            cu if cu & 0b1111_1100 == 0b1111_1000 => Some(5), // U+11000 .. who knows?
            cu if cu & 0b1111_1110 == 0b1111_1100 => Some(6), // Unicode doesn't go this high
            _ => None,                                        // only hit if the input sequence is not UTF-8 encoded
        };

        if let Some(need_more) = need_more {
            if position + need_more <= buf.len() {
                position += need_more;
            }
            break;
        }
    }
    position
}

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
}

impl<'a> fmt::Write for WriteBuf<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.truncated {
            return Err(fmt::Error);
        }

        let input = s.as_bytes();
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

        (&mut self.target[self.position..self.position + input.len()]).copy_from_slice(input);
        self.position += input.len();
        Ok(())
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
}
