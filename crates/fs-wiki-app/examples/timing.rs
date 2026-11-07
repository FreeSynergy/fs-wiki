//! How long the calls the reader makes on the interface thread take, on the real documentation.
//!
//! Pattern: Facade consumer — imports a folder, then times `DocumentSource` calls.
//!
//! Needs a running `fs-wiki-server` (it keeps the texts; start one on an empty data folder):
//!
//! `cargo run --release -p fs-wiki-app --example timing <socket> [docs-folder]`

use std::sync::Arc;
use std::time::Instant;

use fs_components_workbenches::DocumentSource;
use fs_wiki_app::server::connect;
use fs_wiki_app::{FileOrigins, Wiki, WikiSource, import_folder};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(socket) = args.next() else {
        eprintln!("usage: timing <socket of a running fs-wiki-server> [docs-folder]");
        return;
    };
    let docs = args
        .next()
        .unwrap_or_else(|| fs_wiki_app::args::DEFAULT_DOCS.into());
    let rt = tokio::runtime::Runtime::new().unwrap();
    let records =
        std::env::temp_dir().join(format!("fs-wiki-timing-{}.origins", std::process::id()));
    let texts = connect(std::path::Path::new(&socket)).unwrap_or_else(|e| panic!("{e}"));
    let origins = FileOrigins::open(&records).expect("records");
    let wiki = Arc::new(Wiki::new(Arc::new(texts), Arc::new(origins)));
    let t = Instant::now();
    let report = rt.block_on(import_folder(wiki.as_ref(), std::path::Path::new(&docs)));
    println!(
        "import: {} pages, {} failed, {:.1?}",
        report.imported,
        report.failures.len(),
        t.elapsed()
    );
    let source = WikiSource::new(wiki.texts(), rt.handle().clone(), report.failures);
    let time = |what: &str, f: &dyn Fn() -> usize| {
        let t = Instant::now();
        let n = f();
        println!("{what}: {n} results, {:.1?}", t.elapsed());
    };
    time("entries (all)", &|| {
        source.entries("").map_or(0, |e| e.len())
    });
    time("read index.md", &|| {
        source.read("index.md").map_or(0, |d| d.blocks.len())
    });
    time("read components/planned.md", &|| {
        source
            .read("components/planned.md")
            .map_or(0, |d| d.blocks.len())
    });
    for term in ["kanban", "the", "zzzzqq"] {
        time(&format!("find {term:?}"), &|| {
            source.find(term).map_or(0, |h| h.len())
        });
    }
    time("resolve link", &|| {
        usize::from(matches!(
            source.resolve("index.md", "manifesto.md"),
            fs_components_workbenches::LinkTarget::Document(_)
        ))
    });
    let _ = std::fs::remove_file(&records);
}
