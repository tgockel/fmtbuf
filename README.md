fmtbuf
======

Write a formatted string into a fixed buffer.
This is useful when you have a user-provided buffer you want to write into, which frequently arises when writing foreign
function interfaces for C, where strings are expected to have a null terminator.

Usage
-----

```rust
use fmtbuf::WriteBuf;
use std::fmt::Write;

fn main() {
    let mut buf: [u8; 10] = [0; 10];
    let mut writer = WriteBuf::new(&mut buf);
    if let Err(e) = write!(&mut writer, "🚀🚀🚀") {
        println!("write error: {e:?}");
    }
    let written_len = match writer.finish_with("\0") {
        Ok(len) => len, // <- won't be hit since 🚀🚀🚀 is 12 bytes
        Err(len) => {
            println!("writing was truncated");
            len
        }
    };
    let written = &buf[..written_len];
    println!("wrote {written_len} bytes: {written:?}");
    println!("result: {:?}", std::str::from_utf8(written));
}
```

🚀🚀

The primary use case is for implementing APIs like [`strerror_r`](https://linux.die.net/man/3/strerror_r), where the
user provides the buffer.

```rust
use std::{ffi, fmt::Write, io::Error};
use fmtbuf::WriteBuf;

#[no_mangle]
pub unsafe extern "C" fn mylib_strerror(
    err: *mut Error,
    buf: *mut ffi::c_char,
    buf_len: usize
) {
    let mut buf = unsafe {
        // Buffer provided by a users
        std::slice::from_raw_parts_mut(buf as *mut u8, buf_len)
    };
    // Reserve at least 1 byte at the end because we will always
    // write '\0'
    let mut writer = WriteBuf::with_reserve(buf, 1);

    // Use the standard `write!` macro (no error handling for
    // brevity) -- note that an error here might only indicate
    // write truncation, which is handled gracefully be this
    // library's finish___ functions
    let _ = write!(writer, "{}", err.as_ref().unwrap());

    // null-terminate buffer or add "..." if it was truncated
    let _written_len = writer.finish_with_or(b"\0", b"...\0")
        // Err value is also number of bytes written
        .unwrap_or_else(|e| e);
}
```

Features
--------

### `!#[no_std]`

Support for `!#[no_std]` is enabled by disabling the default features and not re-enabling the `"std"` feature.

```toml
fmtbuf = { version = "*", default_features = false }
```

F.A.Q.
------

### Why not write to `&mut [u8]`?

The Rust Standard Library trait [`std::io::Write`](https://doc.rust-lang.org/stable/std/io/trait.Write.html) is
implemented for [`&mut [u8]`](https://doc.rust-lang.org/stable/std/io/trait.Write.html#impl-Write-for-%26mut+%5Bu8%5D)
which could be used instead of this library.
The problem with this approach is the lack of UTF-8 encoding support (also, it is not available in `#![no_std]`).

```rust
use std::io::{Cursor, Write};

fn main() {
    let mut buf: [u8; 10] = [0; 10];
    let mut writer = Cursor::<&mut [u8]>::new(&mut buf);
    if let Err(e) = write!(&mut writer, "rocket: 🚀") {
        println!("write error: {e:?}");
    }
    let written_len = writer.position() as usize;
    let written = &buf[..written_len];
    println!("wrote {written_len} bytes: {written:?}");
    println!("result: {:?}", std::str::from_utf8(written));
}
```

Running this program will show you the error:

```text
write error: Error { kind: WriteZero, message: "failed to write whole buffer" }
wrote 10 bytes: [114, 111, 99, 107, 101, 116, 58, 32, 240, 159]
result: Err(Utf8Error { valid_up_to: 8, error_len: None })
```

The problem is that `"rocket: 🚀"` is encoded as a 12 byte sequence -- the 🚀 emoji is encoded in UTF-8 as the 4 bytes
`b"\xf0\x9f\x9a\x80"` -- but our target buffer is only 10 bytes long.
The `write!` to the cursor naïvely cuts off the 🚀 mid-encode, making the encoded string invalid UTF-8, even though it
advanced the cursor the entire 10 bytes.
This is expected, since `std::io::Write` comes from `io` and does not know anything about string encoding; it operates
on the `u8` level.

One _could_ use the [`std::str::Utf8Error`](https://doc.rust-lang.org/stable/std/str/struct.Utf8Error.html) to properly
cut off the `buf`.
The only issue with this is performance.
Since `std::str::from_utf8` scans the whole string moving forward, it costs _O(n)_ to test this, whereas `fmtbuf` will
do this in _O(1)_, since it only looks at the final few bytes.

### What about Unicode _weird format characters_?

This library only guarantees that the contents of the target buffer is valid UTF-8.
It does not make any guarantees of semantics resulting from truncation due to the Unicode format characters,
specifically `U+200D`, `U+200E`, and `U+200F`.

**What?**

If you don't know what those are, that's okay.
Suffice it to say that human language is complicated and Unicode has a set a features to make things possible, but when
you run out of space to store that in your fixed-size buffer, things go awry.
If you're looking for details, see the mini sections below.

#### `U+200D`: Zero Width Joiner

Certain graphemes like "🙇‍♀" (which you might see as two separate graphemes) are comprised of three code points:

1. 🙇 [`U+1F647` "Person Bowing Deeply"](https://codepoints.net/U+1F647)
2. [`U+200D` "Zero Width Joiner"](https://codepoints.net/U+200D)
3. ♀ [`U+2640` "Female Sign"](https://codepoints.net/U+2640)

So the single grapheme is the 10 byte sequence `b"\xf0\x9f\x99\x87\xe2\x80\x8d\xe2\x99\x80"`.
The question arises: What should happen if the buffer size is only 9?
**On truncation, this library will discard code points which are meant to be modifiers.**
This library will truncate the last Unicode code point, leaving you with `b"\xf0\x9f\x99\x87\xe2\x80\x8d"`--a person
bowing and a zero-width joiner joining with nothing, as the female modifier can not fit.

#### `U+200E` and `U+200F`: Direction Markers

Consider Arabic, which is a right-to-left language:

> ‏آمل أن يحل ‎Rust‏ محل ‎C++‏ يومًا ما.‎

Depending on how compliant with right-to-left presentation your text editor or browser is, you might see that text any
number of ways (if "آمل" on the right-hand side of the text, then the presentation is working).
But note the borrowed words "Rust" and "C++" are still spelled in a left-to-right manner within the right-to-left text
(or they _should_ be).
This is done by encoding [`U+200E` left-to-right mark](https://codepoints.net/U+200E), then writing the borrowed
text, then [`U+200F` right-to-left mark](https://codepoints.net/U+200F) to continue.

What happens if text is reversed, but there is not enough space in the buffer to flip it back?
**On truncation, this library might leave you in the middle of a text-reversed run.**

The construction of [Egyptian Hieroglyphs](https://codepoints.net/egyptian_hieroglyphs) and other languages of this sort
face a similar issue.
Where should the cutoff be?
This library does not know the difference between "𓁪𓌍𓃻" and "𓁪𓌍".
Figuring that out is the responsibility of a higher-level construct.
