//! Texts stored by 0.2.0 have no place in the order of a page: the first page says so, loudly, and
//! `upgrade_index` repairs them without touching what they say.

use std::path::Path;
use std::sync::Arc;

use fs_store::{Store, TursoStore};
use fs_wiki_core::{Error, Paging, TextQuery, Texts, WikiRepo};

/// A copy of the database written by `fs-wiki-core` 0.2.0 (five texts), opened by this version.
async fn old_wiki(dir: &Path) -> WikiRepo {
    for name in ["v020.db", "v020.db-wal"] {
        let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name);
        std::fs::copy(from, dir.join(name)).unwrap();
    }
    let store: Arc<dyn Store> = Arc::new(TursoStore::open(dir.join("v020.db")).await.unwrap());
    WikiRepo::new(store, "https://example.org/a")
}

#[tokio::test]
async fn the_first_page_of_old_texts_fails_loudly_and_upgrade_index_repairs_them() {
    let dir = tempfile::tempdir().unwrap();
    let wiki = old_wiki(dir.path()).await;
    let before = wiki.list("", Paging::default()).await.unwrap();
    assert_eq!(before.len(), 5);

    let all = TextQuery::Prefix(String::new());
    let error = wiki.page(&all, None, 2).await.unwrap_err();
    assert!(matches!(&error, Error::Stored(m) if m.contains("upgrade_index")), "{error}");

    let report = wiki.upgrade_index().await.unwrap();
    assert_eq!((report.rewritten, report.failed.len()), (5, 0), "{report:?}");
    assert_eq!(wiki.upgrade_index().await.unwrap().rewritten, 0, "nothing left to do");

    let (mut seen, mut place) = (vec![], None);
    loop {
        let page = wiki.page(&all, place.as_ref(), 2).await.unwrap();
        seen.extend(page.items.iter().map(|p| p.entry.path.clone()));
        place = page.items.last().map(|p| p.place.clone());
        if !page.more {
            break;
        }
    }
    let listed: Vec<_> = before.iter().map(|t| t.path.clone()).collect();
    assert_eq!(seen, listed, "the same order as list, every text once");
    // What the texts say did not change, and neither did their revision (only `updated` moves).
    let after = wiki.list("", Paging::default()).await.unwrap();
    let said = |texts: &[fs_wiki_core::TextEntry]| -> Vec<_> {
        texts
            .iter()
            .map(|t| (t.id, t.path.clone(), t.revision, t.envelope.name.clone(), t.document.clone()))
            .collect()
    };
    assert!(said(&after) == said(&before), "the texts changed");
}
