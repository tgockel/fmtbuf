#![no_main]

use std::fmt::Write;

use arbitrary::Arbitrary;
use fmtbuf::{TruncatedResultExt, WriteBuf};
use libfuzzer_sys::fuzz_target;

#[derive(Debug, Arbitrary)]
struct Input {
    buf_len: u8,
    reserve: u8,
    input: String,
    finish_with: Option<String>,
    truncate_with: Option<String>,
}

fuzz_target!(|input: Input| {
    let buf_len = usize::from(input.buf_len);
    let reserve = usize::from(input.reserve);

    // Path 1: write then finish.
    {
        let mut buf = vec![0u8; buf_len];
        let mut writer = WriteBuf::with_reserve(&mut buf, reserve);
        let _ = writer.write_str(&input.input);
        let result = writer.finish();
        // The library must produce valid UTF-8 in the written portion.
        assert!(core::str::from_utf8(result.written().as_bytes()).is_ok());
        // And the byte length must be bounded by the buffer.
        assert!(result.written().len() <= buf_len);
    }

    // Path 2: write then finish_with / finish_with_or based on the optional suffixes.
    {
        let mut buf = vec![0u8; buf_len];
        let mut writer = WriteBuf::with_reserve(&mut buf, reserve);
        let _ = writer.write_str(&input.input);
        let result = match (input.finish_with.as_deref(), input.truncate_with.as_deref()) {
            (None, None) => writer.finish(),
            (Some(f), None) => writer.finish_with(f),
            (None, Some(t)) => writer.finish_with_or("", t),
            (Some(f), Some(t)) => writer.finish_with_or(f, t),
        };
        assert!(core::str::from_utf8(result.written().as_bytes()).is_ok());
        assert!(result.written().len() <= buf_len);
    }
});
