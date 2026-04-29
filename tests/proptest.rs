//! Property tests for the `fmtbuf` UTF-8 truncation invariants. These run against the public API.
//!
//! For any UTF-8 input and any buffer size, the crate must:
//!
//! 1. Never panic (under any input).
//! 2. Produce valid UTF-8 in the written portion.
//! 3. Keep state consistent: `written()`, `written_bytes()`, `position()`, and `Truncated::written_len()`
//!    all agree on length.
//! 4. For `finish()`: the result is a prefix of the original input (truncated at a valid char boundary
//!    when the input doesn't fit).

use core::fmt::Write;
use fmtbuf::{TruncatedResultExt, WriteBuf};
use proptest::prelude::*;

fn write_then_finish<'a>(buf: &'a mut [u8], input: &str) -> Result<&'a str, fmtbuf::Truncated<'a>> {
    let mut writer = WriteBuf::new(buf);
    let _ = writer.write_str(input);
    writer.finish()
}

fn write_then_finish_with<'a>(buf: &'a mut [u8], input: &str, suffix: &str) -> Result<&'a str, fmtbuf::Truncated<'a>> {
    let mut writer = WriteBuf::new(buf);
    let _ = writer.write_str(input);
    writer.finish_with(suffix)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn finish_never_panics_and_yields_valid_utf8(
        input in ".{0,64}",
        buf_len in 0usize..=128,
    ) {
        let mut buf = vec![0u8; buf_len];
        let result = write_then_finish(&mut buf, &input);
        let written = result.written();
        // valid_utf8: enforced by the type (returning &str), but also assert the bytes form
        // valid UTF-8 if interpreted directly.
        prop_assert!(core::str::from_utf8(written.as_bytes()).is_ok());
    }

    #[test]
    fn finish_result_is_prefix_of_input(
        input in ".{0,64}",
        buf_len in 0usize..=128,
    ) {
        let mut buf = vec![0u8; buf_len];
        let result = write_then_finish(&mut buf, &input);
        let written = result.written();
        prop_assert!(
            input.starts_with(written),
            "written {written:?} is not a prefix of input {input:?}"
        );
        // If we report no truncation, the entire input must have made it through.
        if !result.is_truncated() {
            prop_assert_eq!(written, input.as_str());
        }
    }

    #[test]
    fn lengths_are_consistent_after_finish(
        input in ".{0,64}",
        buf_len in 0usize..=128,
    ) {
        let mut buf = vec![0u8; buf_len];
        let result = write_then_finish(&mut buf, &input);
        let len_via_ext = result.written_len();
        let len_via_str = result.written().len();
        prop_assert_eq!(len_via_ext, len_via_str);
        if let Err(t) = result {
            prop_assert_eq!(t.written_len(), t.written().len());
            prop_assert_eq!(t.written_len(), len_via_ext);
        }
    }

    #[test]
    fn finish_with_never_panics_and_yields_valid_utf8(
        input in ".{0,64}",
        suffix in ".{0,16}",
        buf_len in 0usize..=128,
    ) {
        let mut buf = vec![0u8; buf_len];
        let result = write_then_finish_with(&mut buf, &input, &suffix);
        let written = result.written();
        prop_assert!(core::str::from_utf8(written.as_bytes()).is_ok());
        prop_assert!(written.len() <= buf_len);
    }

    #[test]
    fn finish_with_ok_ends_with_suffix(
        input in ".{0,32}",
        suffix in ".{0,16}",
        buf_len in 0usize..=128,
    ) {
        let mut buf = vec![0u8; buf_len];
        let result = write_then_finish_with(&mut buf, &input, &suffix);
        if let Ok(written) = result {
            prop_assert!(
                written.ends_with(&*suffix),
                "Ok-arm result {written:?} should end with suffix {suffix:?}"
            );
        }
    }

    #[test]
    fn position_matches_written_after_writes(
        inputs in proptest::collection::vec(".{0,16}", 0..4),
        buf_len in 0usize..=64,
    ) {
        let mut buf = vec![0u8; buf_len];
        let mut writer = WriteBuf::new(&mut buf);
        for input in &inputs {
            let _ = writer.write_str(input);
        }
        prop_assert_eq!(writer.position(), writer.written().len());
        prop_assert_eq!(writer.position(), writer.written_bytes().len());
        prop_assert!(core::str::from_utf8(writer.written_bytes()).is_ok());
    }
}
