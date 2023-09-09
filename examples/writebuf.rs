//! This utility program is useful for testing [`fmtbuf::WriteBuf`] behavior with various buffer sizes, inputs and
//! `finish_with` parameters.

use clap::Parser;
use fmtbuf::WriteBuf;
use std::fmt::Write;

#[derive(Parser, Debug)]
#[command(author, long_about = Some("Utility for testing behavior of `fmtbuf::WriteBuf`"))]
struct Cli {
    /// The size of the target buffer to write to.
    #[arg(long)]
    pub buffer_size: usize,

    /// The number of bytes at the end of the buffer to reserve for finishing. See `WriteBuf::with_reserve`.
    #[arg(long, default_value("0"))]
    pub reserve: usize,

    /// The string to add to the end of the buffer to finish it. See `WriteBuf::finish_with`. If this is specified, but
    /// `--truncate-with` is not, then this string will be used as the truncation string (this behaves as if
    /// `finish_with` was called).
    #[arg(long)]
    pub finish_with: Option<String>,

    /// The string to add to the end of the buffer to finish it if truncation happens. See `WriteBuf::finish_with_or`.
    /// If this is specified, but `--finish-with` is not, this behaves as if `--finish-with` was specified as `""`.
    #[arg(long)]
    pub truncate_with: Option<String>,

    /// Print the debugging information as well. This is used when generating error reports.
    #[arg(long)]
    pub debug: bool,

    /// The string to input. This is passed directly to `write!`.
    pub input: String,
}

fn main() {
    let cli = Cli::parse();
    if cli.debug {
        println!("{cli:?}");
    }

    let mut buf = vec![0; cli.buffer_size];
    let mut writer = WriteBuf::with_reserve(buf.as_mut_slice(), cli.reserve);
    let _ = writer.write_str(&cli.input);
    let result = match (cli.finish_with, cli.truncate_with) {
        (None, None) => writer.finish(),
        (Some(finish), None) => writer.finish_with(finish),
        (None, Some(truncate)) => writer.finish_with_or("", truncate),
        (Some(finish), Some(truncate)) => writer.finish_with_or(finish, truncate),
    };
    let (written_len, truncated) = match result {
        Ok(len) => (len, false),
        Err(len) => (len, true),
    };

    let contents = match std::str::from_utf8(&buf[..written_len]) {
        Ok(contents) => {
            println!("{contents}");
            contents.as_bytes()
        },
        Err(e) => {
            println!("! error: {e:?}");
            &buf[..written_len]
        },
    };
    if cli.debug {
        println!("+ version: {}", env!("CARGO_PKG_VERSION"));
        println!("+ written_len: {written_len}");
        println!("+ truncated: {truncated}");
        println!("+ output_bytes: {contents:?}");
        println!("+ input: {}", cli.input);
        println!("+ input_bytes: {:?}", cli.input.as_bytes());
    }
}
