//! How long the calls the reader makes on the interface thread take, on the real documentation.
//!
//! Pattern: Facade consumer — imports a folder, then times `DocumentSource` calls.
//!
//! `cargo run --release -p fs-wiki-app --example timing [docs-folder]`

use std::sync::Arc;
use std::time::Instant;

use fs_components_workbenches::DocumentSource;
use fs_objects::{Store, TursoStore};
use fs_wiki_app::{WikiSource, import_folder};
use fs_wiki_core::WikiRepo;

fn main() {
    let docs = std::env::args()
        .nth(1)
        .unwrap_or_else(|| fs_wiki_app::args::DEFAULT_DOCS.into());
    let rt = tokio::runtime::Runtime::new().unwrap();
    let store: Arc<dyn Store> = Arc::new(rt.block_on(TursoStore::open_in_memory()).expect("store"));
    let wiki = Arc::new(WikiRepo::new(store, "https://localhost/actors/timing"));
    let t = Instant::now();
    let report = rt.block_on(import_folder(wiki.as_ref(), std::path::Path::new(&docs)));
    println!(
        "import: {} pages, {} failed, {:.1?}",
        report.imported,
        report.failures.len(),
        t.elapsed()
    );
    let source = WikiSource::new(wiki, rt.handle().clone(), report.failures);
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
}
