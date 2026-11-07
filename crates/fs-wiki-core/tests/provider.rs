//! The provider rule (ADR 0045): every provider has every method; what it does not offer answers
//! an explicit "not possible", and its report says the same.

mod common;

use std::sync::Arc;

use async_trait::async_trait;
use common::wiki;
use fs_store::ObjectId;
use fs_text::Document;
use fs_wiki_core::{
    Capabilities, Error, Paging, Place, Result, TextEntry, TextPage, TextQuery, Texts,
};

/// A read-only provider, for example a help library shipped with a program.
struct ReadOnly;

#[async_trait]
impl Texts for ReadOnly {
    fn capabilities(&self) -> Capabilities {
        let mut c = Capabilities::new("read-only");
        c.supported("text.read");
        c.unsupported("text.create", "this library is shipped, not written");
        c
    }
    async fn get(&self, _: ObjectId) -> Result<Option<TextEntry>> {
        Ok(None)
    }
    async fn get_by_path(&self, _: &str) -> Result<Option<TextEntry>> {
        Ok(None)
    }
    async fn list(&self, _: &str, _: Paging) -> Result<Vec<TextEntry>> {
        Ok(vec![])
    }
    async fn page(&self, _: &TextQuery, _: Option<&Place>, _: usize) -> Result<TextPage> {
        Ok(TextPage {
            items: vec![],
            more: false,
        })
    }
}

fn capability<T: std::fmt::Debug>(r: Result<T>) -> String {
    match r {
        Err(Error::Unsupported(u)) => u.capability().to_string(),
        other => panic!("expected an explicit not possible, got {other:?}"),
    }
}

#[tokio::test]
async fn a_missing_method_says_not_possible() {
    let p: Arc<dyn Texts> = Arc::new(ReadOnly);
    let id = ObjectId::new();
    assert!(p.list("", Paging::default()).await.unwrap().is_empty());
    assert!(p.identity().is_none(), "a provider without a store has no identity");
    assert_eq!(capability(p.create("a", "x").await), "text.create");
    assert_eq!(capability(p.edit(id, Document::new()).await), "text.edit");
    assert_eq!(capability(p.move_to(id, "b").await), "text.move");
    assert_eq!(capability(p.move_folder("a", "b").await), "text.move");
    assert_eq!(capability(p.delete(id).await), "text.delete");
    assert_eq!(
        capability(p.find("x", Paging::default()).await),
        "text.find"
    );
    assert_eq!(capability(p.links(id).await), "link.read");
    assert_eq!(capability(p.backlinks("a").await), "link.read");
    assert_eq!(
        capability(p.import_text("a", "text/markdown", "x").await),
        "text.import"
    );
    assert_eq!(
        capability(p.export_text(id, "text/markdown").await),
        "text.export"
    );
    assert!(!p.capabilities().is_supported("text.create"));
}

#[tokio::test]
async fn the_store_backed_provider_reports_what_it_can() {
    let p: Arc<dyn Texts> = Arc::new(wiki().await);
    let caps = p.capabilities();
    for id in [
        "text.create",
        "text.edit",
        "text.move",
        "text.delete",
        "text.read",
        "text.find",
        "link.read",
        "text.import",
        "text.export",
        "text.subscribe",
    ] {
        assert!(caps.is_supported(id), "{id}");
    }
    assert_eq!(
        caps.entries().len(),
        10,
        "one entry per capability, none twice"
    );
}
