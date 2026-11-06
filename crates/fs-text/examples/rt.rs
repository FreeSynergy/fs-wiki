use fs_text::*;
fn main() {
    let p = std::env::args().nth(1).unwrap();
    let s = std::fs::read_to_string(p).unwrap();
    print!("{}", export_markdown(&import_markdown(&s), ExportOptions::default()));
}
