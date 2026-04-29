use crate::{utf8::rfind_utf8_end, TruncatedResultExt, WriteBuf};
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
    for (input, last_valid_idx_after_cut) in TEST_CASES {
        let result = rfind_utf8_end(input.as_bytes());
        assert_eq!(result, input.len(), "input=\"{input}\"");
        if input.is_empty() {
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
    for (input, _) in TEST_CASES {
        let mut buf: [u8; 128] = [0xff; 128];
        let mut writer = WriteBuf::new(&mut buf);

        writer.write_str(input).unwrap();
        assert_eq!(input.len(), writer.position());
        let written = writer.finish().unwrap();
        assert_eq!(input.len(), written.len());
        assert_eq!(*input, written);
    }
}

#[test]
fn format_enough_space_just_enough_reserved() {
    for (input, _) in TEST_CASES {
        let mut buf: [u8; 128] = [0xff; 128];
        let mut writer = WriteBuf::with_reserve(&mut buf[..=input.len()], 1);

        writer.write_str(input).unwrap();
        assert_eq!(input.len(), writer.position());
        let written = writer.finish().unwrap();
        assert_eq!(input.len(), written.len());
        assert_eq!(*input, written);
    }
}

#[test]
fn format_truncation() {
    for (input, last_valid_idx_after_cut) in TEST_CASES {
        if input.is_empty() {
            continue;
        }

        let mut buf: [u8; 128] = [0xff; 128];
        let mut writer = WriteBuf::new(&mut buf[..input.len() - 1]);

        writer.write_str(input).unwrap_err();
        assert_eq!(*last_valid_idx_after_cut, writer.position());
        assert!(writer.truncated());
        write!(writer, "!!!").expect_err("writes should fail here");

        let written = writer.finish().unwrap_err().written();
        assert_eq!(*last_valid_idx_after_cut, written.len());
    }
}

struct SimpleString {
    storage: [u8; 128],
    size: usize,
}

impl SimpleString {
    fn from_segments(segments: &[&str]) -> Self {
        let mut out = Self {
            storage: [0; 128],
            size: 0,
        };
        for segment in segments {
            out.append(segment);
        }
        out
    }

    fn append(&mut self, value: &str) {
        let value = value.as_bytes();
        self.storage[self.size..self.size + value.len()].copy_from_slice(value);
        self.size += value.len();
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.storage[..self.size]).unwrap()
    }
}

impl From<&str> for SimpleString {
    fn from(value: &str) -> Self {
        Self::from_segments(&[value])
    }
}

#[test]
fn finish_with_enough_space() {
    for (input, _) in TEST_CASES {
        let mut buf: [u8; 128] = [0xff; 128];
        let mut writer = WriteBuf::new(&mut buf);

        writer.write_str(input).unwrap();
        let written = writer.finish_with(".123").unwrap();
        assert_eq!(written.len(), input.len() + 4);
        let expected_written = SimpleString::from_segments(&[input, ".123"]);
        assert_eq!(expected_written.as_str(), written);
    }
}

#[test]
fn finish_with_overwrite() {
    for (input, last_valid_idx_after_cut) in TEST_CASES {
        if input.is_empty() {
            continue;
        }

        let mut buf: [u8; 128] = [0xff; 128];
        let mut writer = WriteBuf::new(&mut buf[..input.len()]);

        writer.write_str(input).unwrap();
        let written = writer.finish_with("?").unwrap_err().written();
        assert_eq!(written.len(), last_valid_idx_after_cut + 1);
        let expected_written = SimpleString::from_segments(&[
            core::str::from_utf8(&input.as_bytes()[..*last_valid_idx_after_cut]).unwrap(),
            "?",
        ]);
        assert_eq!(expected_written.as_str(), written);
    }
}

#[test]
fn finish_with_or_with_longer_normal_closer() {
    let mut buf: [u8; 4] = [0xff; 4];
    let writer = WriteBuf::new(&mut buf);

    let written = writer.finish_with_or("0123456789", "abc").unwrap_err().written();
    assert_eq!(written.len(), 3);
    assert_eq!("abc", written);
}

#[test]
fn finish_with_full_overwrite_utf8() {
    let mut buf: [u8; 4] = [0xff; 4];
    let writer = WriteBuf::new(&mut buf);

    let written = writer.finish_with("🚀12").unwrap_err().written();
    assert_eq!(written.len(), 2);
    assert_eq!("12", written);
}

#[test]
fn finish_with_all_continuation_bytes() {
    // The 2-byte tail of "🚀" (b"\xf0\x9f\x9a\x80") is b"\x9a\x80" — both UTF-8
    // continuation bytes, so no valid character start can be found. The buffer
    // is cleared and the result is reported as truncated.
    let mut buf: [u8; 2] = [0xff; 2];
    let writer = WriteBuf::new(&mut buf);

    let written = writer.finish_with("🚀").unwrap_err().written();
    assert_eq!(written, "");
}

#[test]
fn write_rejected_when_remaining_below_reserve() {
    let mut buf: [u8; 4] = [0xff; 4];
    let mut writer = WriteBuf::with_reserve(&mut buf, 10);

    writer.write_str("a").unwrap_err();
    assert!(writer.truncated());
    assert_eq!(writer.position(), 0);

    let written = writer.finish().unwrap_err().written();
    assert_eq!(written, "");
}

#[test]
fn set_reserve_should_not_change_written() {
    let mut buf: [u8; 10] = [0xff; 10];
    let mut writer = WriteBuf::new(&mut buf);

    write!(writer, "0123456789").unwrap();
    assert_eq!("0123456789", writer.written());

    writer.set_reserve(4);
    assert_eq!("0123456789", writer.written());

    let written = writer.finish_with_or("", "!").unwrap();
    assert_eq!("0123456789", written);
}

#[test]
fn truncated_result_ext_ok() {
    for (input, _) in TEST_CASES {
        let mut buf: [u8; 128] = [0xff; 128];
        let mut writer = WriteBuf::new(&mut buf);

        writer.write_str(input).unwrap();
        let result = writer.finish();
        assert!(result.is_ok(), "input=\"{input}\"");
        assert_eq!(result.written(), *input, "input=\"{input}\"");
        assert_eq!(result.written_len(), input.len(), "input=\"{input}\"");
        assert!(!result.is_truncated(), "input=\"{input}\"");
    }
}

#[test]
fn truncated_result_ext_err() {
    for (input, last_valid_idx_after_cut) in TEST_CASES {
        if input.is_empty() {
            continue;
        }

        let mut buf: [u8; 128] = [0xff; 128];
        let mut writer = WriteBuf::new(&mut buf[..input.len() - 1]);

        let _ = writer.write_str(input);
        let result = writer.finish();
        assert!(result.is_err(), "input=\"{input}\"");
        assert_eq!(result.written_len(), *last_valid_idx_after_cut, "input=\"{input}\"");
        assert_eq!(
            result.written(),
            &input[..*last_valid_idx_after_cut],
            "input=\"{input}\""
        );
        assert!(result.is_truncated(), "input=\"{input}\"");
    }
}

#[test]
fn truncated_result_ext_empty_err() {
    // Mirrors `finish_with_all_continuation_bytes`: the buffer is too small to
    // hold any valid UTF-8 prefix of the suffix, so `Err` carries an empty `&str`.
    let mut buf: [u8; 2] = [0xff; 2];
    let writer = WriteBuf::new(&mut buf);

    let result = writer.finish_with("🚀");
    assert!(result.is_err());
    assert_eq!(result.written(), "");
    assert_eq!(result.written_len(), 0);
    assert!(result.is_truncated());
}

#[test]
fn truncated_result_ext_no_unwrap() {
    // The trait dispatches directly on the raw `Result` from the finish family,
    // without needing an intermediate `.unwrap()` or `.written()`.
    let mut buf: [u8; 4] = [0xff; 4];
    let mut writer = WriteBuf::new(&mut buf);
    let _ = write!(writer, "abcdef");

    let written: &str = writer.finish_with_or("!", "...").written();
    assert_eq!(written, "a...");
}
