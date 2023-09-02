fmtbuf
======

Format into a fixed buffer.

Usage
-----

```rust
let mut buf: [u8; 128] = [0; 128];
```

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
    let mut writer = WriteBuf::new(buf);

    // Use the standard `write!` macro (no error handling for brevity)
    write!(writer, "{}", err.as_ref().unwrap()).unwrap();

    let _ =
        if writer.truncated() {
            // the message was truncated, let the caller know by adding "..."
            writer.finish_with(b"...\0")
        } else {
            // just null-terminate the buffer
            writer.finish_with(b"\0")
        };
}
```
