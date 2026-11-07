use std::sync::Arc;
use std::time::Instant;
use fs_store::{Store, TursoStore};
use fs_wiki_core::{Texts, WikiRepo, MARKDOWN};

fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let mut es: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    es.sort();
    for p in es { if p.is_dir() { walk(&p, out) } else if p.extension().is_some_and(|e| e == "md") { out.push(p) } }
}

#[tokio::main]
async fn main() {
    let root = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    let mut files = vec![]; walk(&root, &mut files);
    for n in std::env::args().skip(2).map(|s| s.parse::<usize>().unwrap()) {
        let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
        let wiki = WikiRepo::new(store, "https://example.org/a");
        let texts: Vec<(String,String)> = files.iter().take(n).map(|f| (f.strip_prefix(&root).unwrap().to_string_lossy().into_owned(), std::fs::read_to_string(f).unwrap())).collect();
        let t = Instant::now();
        let mut parse = std::time::Duration::ZERO;
        for (p, md) in &texts {
            let a = Instant::now();
            let _ = fs_wiki_core::TextFormat::Markdown.import(md);
            parse += a.elapsed();
            let _ = p;
        }
        let _ = t;
        let t = Instant::now();
        for (p, md) in &texts { wiki.import_text(p.trim_end_matches(".md"), MARKDOWN, md).await.unwrap(); }
        println!("n={n} parse-only {parse:?} import {:?}", t.elapsed());
    }
}
