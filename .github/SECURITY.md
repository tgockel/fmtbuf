Security
========

This library does not use any crazy features of Rust.
At the time of writing, there is only one `unsafe` block, which bypasses a UTF-8 check which should not be needed if
everything going into `write!` follows Rust's UTF-8 policy.
Any security vulnerabilities are likely [higher-level concerns](https://www.rust-lang.org/policies/security) than this
little format library.

That said, if you do find a security vulnerability that is specific to this library, please
[email me](mailto:travis@gockelhut.com) directly.
I do not have a specific policy for addressing security concerns because they seem quite unlikely, but I will probably
just follow the Rust security policy.
