//! The binary of the window alone (`fs-wiki` holds window and server in one).
//!
//! Pattern: none (thin binary) — hands the words to `gui::run`.

fn main() {
    let words: Vec<String> = std::env::args().skip(1).collect();
    if let Err(message) = fs_wiki_app::gui::run(&words) {
        eprintln!("fs-wiki-app: {message}");
        std::process::exit(1);
    }
}
