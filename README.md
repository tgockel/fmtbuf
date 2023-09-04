fmtbuf
======

Write a formatted string into a fixed buffer.
This is useful when you have a user-provided buffer you want to write into, which frequently arises when writing foreign
function interfaces for C.

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
    let written_len = match writer.finish() {
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
pub extern "C" fn mylib_strerror(err: *mut Error, buf: *mut ffi::c_char, buf_len: usize) {
    let mut buf = unsafe {
        // Buffer provided by a users
        let mut buf = std::slice::from_raw_parts_mut(buf as *mut u8, buf_len);
    };
    // Reserve at least 1 byte at the end because we will always write '\0'
    let mut writer = WriteBuf::with_reserve(buf, 1);

    // Use the standard `write!` macro (no error handling for brevity)
    write!(writer, "{}", err.as_ref().unwrap()).unwrap();

    // null-terminate buffer or add "..." if it was truncated
    let _written_len = writer.finish_with_or(b"\0", b"...\0")
        // Err value is also number of bytes written
        .unwrap_or_else(|e| e);
}
```

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

The problem is that `"rocket: 🚀"` is encoded as the 12 byte sequence -- the 🚀 emoji is encoded in UTF-8 as the 4 bytes
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
