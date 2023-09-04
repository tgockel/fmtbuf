const CODE_UNIT_INDICATE_WIDTH: [u8; 256] = [
    // low order nibble
    // 1, 2, 3, 4, 5, 6, 7, 8, 9, a, b, c, d, e, f
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0 h
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 1 i
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 2 g
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 3 h
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 4 o
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 5 r
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 6 d
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 7 e
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 8 r
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 9
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // a n
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // b i
    0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // c b
    2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // d b
    3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // e l
    4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // f e
];

/// If `code_unit` is a UTF-8 starting character, then return `Some(len)`, where `len` is the number of code units the
/// encoded run represents. If `code_unit` is a continuation character (a value seen in the middle of an encoded run),
/// then return `None`.
pub const fn utf8_char_width(code_unit: u8) -> Option<usize> {
    let x = CODE_UNIT_INDICATE_WIDTH[code_unit as usize];
    if x == 0 {
        None
    } else {
        Some(x as usize)
    }
}

/// Find the end of the last valid UTF-8 code point.
///
/// # Deprecated
///
/// This function will not be part of the public API in a future release.
///
/// # Parameters
///
/// * `buf`: This should be an almost-valid UTF-8 encoded sequence. The final bytes can be a UTF-8 multi-byte sequence
///   which is incomplete.
///
/// # Returns
///
/// The number of code units which are valid UTF-8 (assuming `buf` adheres to the above specification).
pub fn rfind_utf8_end(buf: &[u8]) -> usize {
    let mut position = buf.len();
    // If the end of the string is middle of writing a UTF-8 multibyte sequence, we need to reverse to before the
    // code units for this incomplete code point.
    while position > 0 {
        position -= 1;

        // Keep scanning backwards until we find a code unit that is a valid start of a UTF-8 sequence; if we found one,
        // then `need_more` is the number of code units that multi-byte sequence should have.
        if let Some(need_more) = utf8_char_width(buf[position]) {
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

    #[test]
    fn rfind_utf8_end_examples() {
        assert_eq!(rfind_utf8_end("1234".as_bytes()), 4);
        assert_eq!(rfind_utf8_end("🚀".as_bytes()), 4);
        assert_eq!(rfind_utf8_end(b"\xf0\x9f\x9a\x80"), 4); // "🚀" with the bytes written out
        assert_eq!(rfind_utf8_end(b"\xf0\x9f\x9a"), 0); // "🚀" but missing the last byte
    }
}
