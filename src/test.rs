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

#[test]
fn zero_length_buffer_does_not_panic() {
    // `write_str` to an empty buffer is rejected (truncated), not a panic.
    let mut buf: [u8; 0] = [];
    let mut writer = WriteBuf::new(&mut buf);
    write!(writer, "anything").unwrap_err();
    assert!(writer.truncated());
    assert_eq!(writer.position(), 0);
    assert_eq!(writer.written(), "");

    // `finish` on a fresh empty buffer reports `Ok("")` because no write was attempted.
    let mut buf: [u8; 0] = [];
    let written = WriteBuf::new(&mut buf).finish().unwrap();
    assert_eq!(written, "");

    // `finish_with` on a fresh empty buffer cannot place the suffix; reports truncated empty.
    let mut buf: [u8; 0] = [];
    let result = WriteBuf::new(&mut buf).finish_with("x");
    assert!(result.is_err());
    assert_eq!(result.written(), "");

    // `finish_with_or` likewise.
    let mut buf: [u8; 0] = [];
    let result = WriteBuf::new(&mut buf).finish_with_or("!", "...");
    assert!(result.is_err());
    assert_eq!(result.written(), "");
}

#[test]
fn reserve_larger_than_buffer_through_finish_with() {
    // `with_reserve` allows `reserve > target.len()`; no write succeeds, and the finish family
    // still does its best to place a suffix (writes ignore reserve once the buffer is being
    // closed out).
    let mut buf: [u8; 4] = [0xff; 4];
    let mut writer = WriteBuf::with_reserve(&mut buf, 10);
    write!(writer, "x").unwrap_err();
    assert!(writer.truncated());
    let result = writer.finish_with("!");
    assert!(result.is_err(), "buffer is marked truncated");
    assert_eq!(result.written(), "!");

    let mut buf: [u8; 4] = [0xff; 4];
    let mut writer = WriteBuf::with_reserve(&mut buf, 10);
    write!(writer, "x").unwrap_err();
    let result = writer.finish_with_or("normal", "trunc");
    assert!(result.is_err());
    // `truncated` (5 bytes) does not fit in the 4-byte buffer; the documented behavior is to
    // keep the last bytes that start at a valid UTF-8 boundary -- here, "runc".
    assert_eq!(result.written(), "runc");
}

#[test]
fn truncation_flag_survives_subsequent_writes() {
    // After a write hits truncation, the next `write_str` must immediately return `Err` and
    // leave the previous `position` unchanged.
    let mut buf: [u8; 4] = [0xff; 4];
    let mut writer = WriteBuf::new(&mut buf);
    write!(writer, "abcdef").unwrap_err();
    assert!(writer.truncated());
    let position_after_first = writer.position();

    write!(writer, "ghi").unwrap_err();
    assert!(writer.truncated());
    assert_eq!(writer.position(), position_after_first);

    let result = writer.finish_with_or("!", "...");
    assert!(result.is_err());
    assert_eq!(result.written(), "a...");
}

#[cfg(feature = "std")]
#[test]
fn debug_format_describes_state() {
    // The `Debug` impl shows the user-visible state but never the raw target bytes (those
    // can be uninitialized sentinels). Gated on `std` because `format!` requires `alloc`.
    let mut buf: [u8; 8] = [0xff; 8];
    let mut writer = WriteBuf::new(&mut buf);
    write!(writer, "hi").unwrap();
    let s = format!("{writer:?}");
    assert!(s.contains("position: 2"), "{s}");
    assert!(s.contains("capacity: 8"), "{s}");
    assert!(s.contains("reserve: 0"), "{s}");
    assert!(s.contains("truncated: false"), "{s}");
    assert!(s.contains("written: \"hi\""), "{s}");
    assert!(!s.contains("0xff"), "raw target bytes leaked: {s}");
}
