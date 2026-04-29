Security
========

This library does not use any crazy features of Rust.
At the time of writing, there is exactly one `unsafe fn` (`from_utf8_expect` in `src/lib.rs`) that bypasses Rust's
UTF-8 check.
Its safety invariant is upheld by construction: every byte written into the buffer comes from a `&str` validated by
`core::fmt::Write`, and debug builds re-verify the invariant via `core::str::from_utf8` before reading the buffer
back as a `&str`.
The two sibling modules (`truncated` and `utf8`) both have `#![forbid(unsafe_code)]`, so the unsafe surface cannot
spread without an explicit edit to that policy.
Any security vulnerabilities are likely [higher-level concerns](https://www.rust-lang.org/policies/security) than this
little format library.

That said, if you do find a security vulnerability that is specific to this library, please
[email me](mailto:travis@gockelhut.com) directly.
I do not have a specific policy for addressing security concerns because they seem quite unlikely, but I will probably
just follow the Rust security policy.
