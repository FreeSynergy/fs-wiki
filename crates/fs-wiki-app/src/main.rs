//! The binary: wiring only.
//!
//! Pattern: Facade — the one entry point over the app: parse arguments, then the Admin window (default), the headless
//! check (`--check`) or the reader (`--reader`).

mod admin_mode;
mod reader;

use fs_wiki_app::args::{HELP, parse};

fn main() {
    if let Err(message) = run() {
        eprintln!("fs-wiki-app: {message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let given: Vec<String> = std::env::args().skip(1).collect();
    let args = parse(&given)?;
    if args.help {
        println!("{HELP}");
        return Ok(());
    }
    if args.reader { reader::run(&args) } else { admin_mode::run(&args) }
}
