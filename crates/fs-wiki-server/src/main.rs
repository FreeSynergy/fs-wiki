//! `fs-wiki-server`. Usage: `fs-wiki-server [--socket PATH] [--data DIR] [--actor IRI] [--instance DOMAIN]
//! [--hub-socket PATH] [--lease FILE]`. The same server is `fs-wiki serve`.
//!
//! Pattern: none (thin binary) — hands the words to `serve`.

use std::process::ExitCode;

use fs_wiki_server::{serve, ServeError};

fn main() -> ExitCode {
    match serve(std::env::args_os().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(ServeError::Usage(m)) => {
            eprintln!("fs-wiki-server: {m}");
            ExitCode::from(2)
        }
        Err(ServeError::Failed(m)) => {
            eprintln!("fs-wiki-server: {m}");
            ExitCode::FAILURE
        }
    }
}
