Contributing Guide
==================

This little open-source project is full Rust with no external dependencies, so everything should be easy to do right out
of the box.
Just grab [Rust](https://www.rust-lang.org/) from [Rustup](https://rustup.rs/) and you're good to go.

Building
--------

```shell
cargo test
```

You can also test the `#![no_std]` configuration by disabling the default features:

```shell
cargo test --no-default-features
```

Format with `fmt` and listen to Clippy:

```shell
cargo fmt
cargo clippy
```

Column width is 120 because 80 is way too small and 100 does not really feel like a huge improvement.
140 characters would be right out.

Changes
-------

Before you begin, check the [Issue Tracker](https://github.com/tgockel/fmtbuf/issues) to see if your problem has been
encountered by someone else.
If you can't find an issue, open a new one with a descriptive title and a descriptive description.
Follow the issue template, write a failing unit test, or use the example program
[`writebuf`](https://github.com/tgockel/fmtbuf/blob/trunk/examples/writebuf.rs) to demonstrate what is going on.

After that, this project follows the [Fork and Pull model](https://en.wikipedia.org/wiki/Fork_and_pull_model).

1. Fork the repository
2. Branch in your fork
3. Write your code
4. Commit your code (mention `issue #N` in the commit message)
5. Watch your tests pass the [workflows](https://github.com/tgockel/fmtbuf/tree/trunk/.github/workflows)
6. Issue a pull request

### Sign Your Commits

When committing code, please [sign commits with GPG](https://docs.github.com/en/authentication/managing-commit-signature-verification).
This lets others know that work submitted by you was really created by you.
If you always want to sign commits instead of specifying `-S` on the command line every time, add it to your global
configuration:

```shell
git config --global user.signingkey ${YOUR_KEY_ID}
git config --global commit.gpgsign true
```

Also, this setting is for maintainers, but it tells `git` to sign annotated tags and should probably be a default, but
isn't:

```shell
git config --global tag.forceSignAnnotated true
```
