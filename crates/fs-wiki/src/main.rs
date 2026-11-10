//! `fs-wiki`: one binary for everything (ADR 0063). Pattern: Facade over the crates that stay libraries.
//!
//! ```text
//! fs-wiki                          the window (feature `gui`, on by default)
//! fs-wiki [--socket F] [--check]   the window, options as in `fs-wiki-app --help`
//! fs-wiki serve                    the server (Cap'n Proto, texts.capnp)
//! fs-wiki --version                the version, without opening anything
//! ```

use std::ffi::OsString;
use std::process::ExitCode;

use fs_wiki_server::ServeError;

fn main() -> ExitCode {
    let raw: Vec<OsString> = std::env::args_os().skip(1).collect();
    let text = |i: usize| raw.get(i).and_then(|w| w.to_str());
    match text(0) {
        Some("--version" | "-V") => {
            println!("fs-wiki {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("serve") => serve(&raw[1..]),
        // `fs-wiki --socket PATH serve`: the option may stand before the word.
        Some("--socket") if text(2) == Some("serve") => {
            let mut words = vec![raw[0].clone(), raw[1].clone()];
            words.extend(raw[3..].iter().cloned());
            serve(&words)
        }
        None | Some("gui") => window(&lossy(raw.get(1..).unwrap_or_default())),
        Some(_) => window(&lossy(&raw)),
    }
}

fn lossy(words: &[OsString]) -> Vec<String> {
    words
        .iter()
        .map(|w| w.to_string_lossy().into_owned())
        .collect()
}

fn serve(words: &[OsString]) -> ExitCode {
    match fs_wiki_server::serve(words.iter().cloned()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(ServeError::Usage(m)) => {
            eprintln!("fs-wiki: {m}");
            ExitCode::from(2)
        }
        Err(ServeError::Failed(m)) => {
            eprintln!("fs-wiki: {m}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(feature = "gui")]
fn window(words: &[String]) -> ExitCode {
    match fs_wiki_app::gui::run(words) {
        Ok(()) => ExitCode::SUCCESS,
        Err(m) => {
            eprintln!("fs-wiki: {m}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(not(feature = "gui"))]
fn window(_words: &[String]) -> ExitCode {
    eprintln!(
        "fs-wiki: this build has no window (built with --no-default-features); use `fs-wiki serve`"
    );
    ExitCode::FAILURE
}
