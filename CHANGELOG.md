Changelog
=========

All notable changes to this project will be documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] -- Unreleased

## [0.2.0] -- 2026-04-29

### Added

- `WriteBuf::finish()` and related functions now return validated `&str`s instead of `&[u8]`s you have to validate
  yourself. This makes this library useful for the simple case of writing to a fixed-size/preallocated buffer.
- `WriteBuf::capacity()` returning the size of the target buffer.
- `WriteBuf::remaining()` returning the number of bytes still available for `write_str` operations
  (truncation-aware; saturates on `reserve > capacity - position`).
- `WriteBuf::clear()` resetting `position` and the `truncated` flag for buffer reuse. The configured `reserve` is
  preserved.
- `Truncated<'_>` error type, returned by the `WriteBuf::finish*` family. Carries the portion of the output that was
  successfully written so callers don't have to track it separately. Implements `Debug`, `Display`, `Error`, and
  derives `Clone`, `Copy`, `PartialEq`, `Eq`, and `Hash` so it can be compared, hashed, and stored.
- `Truncated::written()` and `Truncated::written_len()` -- inherent accessors that mirror the methods on
  `TruncatedResultExt` so the same vocabulary works whether you hold the `Truncated` directly or a
  `Result<&str, Truncated<'_>>`.
- `WriteBuf::written()` returns the written portion as a `&str` (companion to the existing `written_bytes()` accessor).
- `fmt::Debug` impl on `WriteBuf` for diagnostic logging. Shows position, capacity, reserve, the truncated flag, and
  the validly-written `&str`; deliberately omits the raw target bytes (which may be uninitialized).

### Changed

- **Breaking:** `WriteBuf::finish`, `WriteBuf::finish_with`, and `WriteBuf::finish_with_or` now return
  `Result<&str, Truncated<'_>>` instead of `Result<usize, usize>`. The new shape gives callers the validated
  string slice directly without a separate `from_utf8` step on the underlying buffer.
- **Breaking:** `finish_with` and `finish_with_or` now require `impl AsRef<str>` for the suffix arguments
  (previously `impl AsRef<[u8]>`). This keeps the UTF-8 invariant at the type level rather than relying on a
  runtime check.
- **Breaking:** Minimum supported Rust version raised to 1.81 (for stable `core::error::Error`); edition is now 2021.

### Removed

- **Breaking:** `rfind_utf8_end` is no longer re-exported from the crate root. It remains as a crate-internal helper.

### Migration from 0.1.x

- Replace `let n = writer.finish().unwrap();` with `let s = writer.finish().unwrap();` -- the success value is now
  `&str` rather than the byte count.
- For the error path, use `e.written()` to access the partially-written `&str`:
  `writer.finish().unwrap_or_else(|e| e.written())`.
- If a `finish_with*` call passed a non-`str` buffer, convert via `core::str::from_utf8` first.

## [0.1.2] -- 2023-09-10

- Documentation additions describing the lack of semantic Unicode support on truncation.

## [0.1.1] -- 2023-09-03

- Added `WriteBuf` accessors to query and alter the reserve byte count.

## [0.1.0] -- 2023-09-02

- Initial release.
