//! Start time and link health on a COPY of the real documentation.
//!
//! Pattern: Facade consumer — syncs a folder into the wiki of a running server twice (first start,
//! second start: a new connection and the records read again), counts the links that lead nowhere,
//! moves a folder, counts again, and starts once more. Give it a server on an empty data folder and a
//! COPY of the documentation: it writes into that server's wiki.
//!
//! `cargo run --release -p fs-wiki-app --example real_docs <socket> <docs-copy> <folder-to-move> <new-path>`

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use fs_components_workbenches::LinkTarget;
use fs_wiki_app::server::connect;
use fs_wiki_app::{FileOrigins, Wiki, import_folder, resolve_with_folders};
use fs_wiki_core::{Paging, Texts};

fn open(socket: &Path, records: &Path) -> Arc<Wiki> {
    let texts = connect(socket).unwrap_or_else(|e| panic!("{e}"));
    let origins = FileOrigins::open(records).expect("records");
    Arc::new(Wiki::new(Arc::new(texts), Arc::new(origins)))
}

async fn start(label: &str, socket: &Path, records: &Path, docs: &Path) -> Arc<Wiki> {
    let t = Instant::now();
    let wiki = open(socket, records);
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
async fn broken(wiki: &Wiki) -> (usize, usize) {
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
    let [socket, docs, folder, new] = &args[..] else {
        eprintln!("usage: real_docs <socket> <docs-copy> <folder-to-move> <new-path>");
        return;
    };
    let (socket, docs) = (Path::new(socket), Path::new(docs));
    let records =
        std::env::temp_dir().join(format!("fs-wiki-real-docs-{}.origins", std::process::id()));
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        drop(start("first start ", socket, &records, docs).await);
        let wiki = start("second start", socket, &records, docs).await;
        let (total, before) = broken(&wiki).await;
        println!("links {total}, leading nowhere before the move: {before}");
        let t = Instant::now();
        let moved = wiki.move_folder(folder, new).await.expect("move");
        println!("moved {} texts in {:.1?}", moved.len(), t.elapsed());
        let (_, after) = broken(&wiki).await;
        println!("leading nowhere after the move: {after}");
        drop(wiki);
        let wiki = start("start after the move", socket, &records, docs).await;
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
    let _ = std::fs::remove_file(&records);
}
