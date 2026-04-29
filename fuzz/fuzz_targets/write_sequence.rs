//! Fuzz a random sequence of `write_str` / `clear` / `set_reserve` calls. The interesting bug
//! class here is state corruption when truncation hits midstream and the caller keeps writing,
//! clearing, or reconfiguring the reserve.
//!
//! The companion target `finish_with` covers the suffix variants of the finish family. This one
//! focuses on the multi-call write path, asserting type-wide invariants after every operation.

#![no_main]

use std::fmt::Write;

use arbitrary::Arbitrary;
use fmtbuf::WriteBuf;
use libfuzzer_sys::fuzz_target;

#[derive(Debug, Arbitrary)]
enum Op {
    Write(String),
    Clear,
    SetReserve(u8),
}

#[derive(Debug, Arbitrary)]
struct Input {
    buf_len: u8,
    initial_reserve: u8,
    ops: Vec<Op>,
}

fuzz_target!(|input: Input| {
    let buf_len = usize::from(input.buf_len);
    let initial_reserve = usize::from(input.initial_reserve);

    let mut buf = vec![0u8; buf_len];
    let mut writer = WriteBuf::with_reserve(&mut buf, initial_reserve);
    assert_eq!(writer.capacity(), buf_len);

    for op in &input.ops {
        let was_truncated = writer.truncated();

        match op {
            Op::Write(s) => {
                let result = writer.write_str(s);
                if was_truncated {
                    // Once a write is rejected, every subsequent write must immediately fail. This
                    // is the load-bearing invariant of the truncated flag.
                    assert!(result.is_err(), "write_str succeeded after a previous truncation");
                }
            }
            Op::Clear => {
                writer.clear();
                assert!(!writer.truncated());
                assert_eq!(writer.position(), 0);
                assert_eq!(writer.written(), "");
            }
            Op::SetReserve(n) => {
                writer.set_reserve(usize::from(*n));
                assert_eq!(writer.reserve(), usize::from(*n));
            }
        }

        // Invariants that must hold after every operation, regardless of branch above.
        assert_eq!(writer.capacity(), buf_len, "capacity is fixed");
        assert!(writer.position() <= writer.capacity());
        assert_eq!(writer.position(), writer.written().len());
        assert_eq!(writer.position(), writer.written_bytes().len());
        // `written()` returning `&str` already proves valid UTF-8, but assert the underlying bytes
        // independently to catch any divergence between `written` and `written_bytes`.
        assert!(core::str::from_utf8(writer.written_bytes()).is_ok());

        let expected_remaining = if writer.truncated() {
            0
        } else {
            writer
                .capacity()
                .saturating_sub(writer.position())
                .saturating_sub(writer.reserve())
        };
        assert_eq!(writer.remaining(), expected_remaining);
    }

    // Final finish must respect the same invariants as the standalone `finish_with` target.
    let final_truncated = writer.truncated();
    let result = writer.finish();
    assert_eq!(result.is_err(), final_truncated);
    let written = match result {
        Ok(s) => s,
        Err(t) => t.written(),
    };
    assert!(core::str::from_utf8(written.as_bytes()).is_ok());
    assert!(written.len() <= buf_len);
});
