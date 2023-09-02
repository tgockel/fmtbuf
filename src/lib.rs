//! # `fmtbuf`
//! This library is intended to help write formatted text to fixed buffers.

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
            _ => None, // only hit if the input sequence is not UTF-8 encoded
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

#[cfg(test)]
mod test {
    use super::*;

    static TEST_CASES: &[(&[u8], usize)] = &[
        (b"", 0),
        (b"James", 5),
        ("Søren".as_bytes(), 6),
        (b"_\xc3", 1),
        ("磨".as_bytes(), 3),
        ("here: 见/見".as_bytes(), 13),
        (b"here: \xe8\xa7\x81/\xe8\xa6", 10),
        ("𨉟呐㗂越".as_bytes(), 13),
        ("🚀".as_bytes(), 4),
        ("🚀🚀🚀".as_bytes(), 12),
        (b"rocket: \xf0\x9f\x9a\x80", 12),
        (b"rocket: \xf0\x9f\x9a", 8),
    ];

    #[test]
    fn rfind_utf8_end_test() {
        for (input, last_valid_idx) in TEST_CASES.iter() {
            let result = rfind_utf8_end(input);
            assert_eq!(result, *last_valid_idx, "input={input:?}");
        }
    }
}
