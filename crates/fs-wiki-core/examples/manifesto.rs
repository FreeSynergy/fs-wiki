//! Import the manifesto from a Markdown file, read it back and list its links.
//!
//! Pattern: Facade consumer — everything goes through the `Texts` trait.
//!
//! `cargo run -p fs-wiki-core --example manifesto [path/to/file.md]`

use std::sync::Arc;

use fs_objects::{Store, TursoStore};
use fs_wiki_core::{Texts, WikiRepo, MARKDOWN};

const DEFAULT: &str = "/srv/FreeSynergy/fs-documentation/docs/en/manifesto.md";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file = std::env::args().nth(1).unwrap_or_else(|| DEFAULT.into());
    let markdown = std::fs::read_to_string(&file)?;

    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await?);
    let wiki = WikiRepo::new(store, "https://example.org/actors/me");

    let text = wiki
        .import_text("docs/manifesto", MARKDOWN, &markdown)
        .await?;
    println!("imported {file}");
    println!("  name: {}", text.envelope.name);
    println!("  iri:  {}", text.envelope.iri.as_deref().unwrap_or("-"));
    println!("  blocks: {}", text.document.blocks.len());

    let back = wiki.export_text(text.id, MARKDOWN).await?;
    let same = back == markdown;
    println!(
        "  read back: {} bytes, identical to the file: {same}",
        back.len()
    );

    println!("links:");
    for link in wiki.links(text.id).await? {
        let kind = if link.internal {
            "internal"
        } else {
            "external"
        };
        println!("  [{kind}] {:?} -> {}", link.title, link.target);
    }
    Ok(())
}
