//! Start time and link health on a COPY of the real documentation.
//!
//! Pattern: Facade consumer — syncs a folder into a store file twice (first start, second start),
//! counts the links that lead nowhere, moves a folder, counts again, and starts once more.
//! Never point it at the live documentation: it only reads the folder, but give it a copy.
//!
//! `cargo run --release -p fs-wiki-app --example real_docs <docs-copy> <folder-to-move> <new-path>`

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use fs_components_workbenches::LinkTarget;
use fs_store::{Store, TursoStore};
use fs_wiki_app::{import_folder, resolve_with_folders};
use fs_wiki_core::{Paging, Texts, WikiRepo};

async fn open(file: &Path) -> Arc<WikiRepo> {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open(file).await.expect("store"));
    Arc::new(WikiRepo::new(store, "https://localhost/actors/real-docs"))
}

async fn start(label: &str, store: &Path, docs: &Path) -> Arc<WikiRepo> {
    let t = Instant::now();
    let wiki = open(store).await;
    let r = import_folder(wiki.as_ref(), docs).await;
    println!(
        "{label}: {:.1?}  new {} updated {} unchanged {} failed {} conflicts {} gone {} skipped {}",
        t.elapsed(),
        r.imported,
        r.updated,
        r.unchanged,
        r.failures.len(),
        r.conflicts.len(),
        r.gone.len(),
        r.skipped.len()
    );
    for f in r.failures.iter().take(3) {
        println!("  failure {}: {}", f.path, f.reason);
    }
    wiki
}

/// (links, links that resolve to nothing) over all texts, resolved as the reader does.
async fn broken(wiki: &WikiRepo) -> (usize, usize) {
    let mut all = vec![];
    while let Ok(page) = wiki.list("", Paging::new(500, all.len())).await {
        if page.is_empty() {
            break;
        }
        all.extend(page);
    }
    let paths: BTreeSet<String> = all.iter().map(|t| t.path.clone()).collect();
    let first = |folder: &str| {
        let prefix = if folder.is_empty() {
            String::new()
        } else {
            format!("{folder}/")
        };
        paths
            .range(prefix.clone()..)
            .next()
            .filter(|p| p.starts_with(&prefix))
            .cloned()
    };
    let (mut total, mut bad) = (0, 0);
    for text in &all {
        for link in wiki.links(text.id).await.expect("links") {
            if !link.internal || link.target.starts_with('#') {
                continue;
            }
            total += 1;
            let r = resolve_with_folders(&text.path, &link.target, &|p| paths.contains(p), &first);
            bad += usize::from(matches!(r, LinkTarget::Missing(_)));
        }
    }
    (total, bad)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [docs, folder, new] = &args[..] else {
        eprintln!("usage: real_docs <docs-copy> <folder-to-move> <new-path>");
        return;
    };
    let docs = Path::new(docs);
    let store = std::env::temp_dir().join(format!("fs-wiki-real-docs-{}.db", std::process::id()));
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        drop(start("first start ", &store, docs).await);
        let wiki = start("second start", &store, docs).await;
        let (total, before) = broken(&wiki).await;
        println!("links {total}, leading nowhere before the move: {before}");
        let t = Instant::now();
        let moved = wiki.move_folder(folder, new).await.expect("move");
        println!("moved {} texts in {:.1?}", moved.len(), t.elapsed());
        let (_, after) = broken(&wiki).await;
        println!("leading nowhere after the move: {after}");
        drop(wiki);
        let wiki = start("start after the move", &store, docs).await;
        println!(
            "texts below the old folder after restart: {}",
            wiki.list(&format!("{folder}/"), Paging::default())
                .await
                .unwrap()
                .len()
        );
        let (_, again) = broken(&wiki).await;
        println!("leading nowhere after restart: {again}");
    });
    let size = std::fs::metadata(&store).map_or(0, |m| m.len());
    println!("store file: {} MB", size / 1_000_000);
    let _ = std::fs::remove_file(&store);
}
