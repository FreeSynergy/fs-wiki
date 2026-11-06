use fs_text::*;
fn main() {
    let root = std::env::args().nth(1).unwrap();
    let (mut n, mut ident, mut unstable) = (0, 0, 0);
    let mut stack = vec![std::path::PathBuf::from(root)];
    while let Some(p) = stack.pop() {
        for e in std::fs::read_dir(&p).unwrap() {
            let path = e.unwrap().path();
            if path.is_dir() { stack.push(path); continue; }
            if path.extension().map(|x| x != "md").unwrap_or(true) { continue; }
            let src = std::fs::read_to_string(&path).unwrap();
            let d1 = import_markdown(&src);
            let out = export_markdown(&d1, ExportOptions::default());
            let d2 = import_markdown(&out);
            n += 1;
            let adm = export_markdown(&d1, ExportOptions { admonitions: true, ..Default::default() });
            if out == src || adm == src { ident += 1; }
            if !d1.same_content(&d2) { unstable += 1; if unstable < 6 { println!("UNSTABLE {}", path.display()); } }
        }
    }
    println!("files {n} identical {ident} unstable {unstable}");
}
