//! texts 0.2 through the client, against the real server: the trait `Texts` answers what the core answers.
//!
//! Pattern: Integration tests against the trait; no stand-ins.

mod common;

use common::{make, ClientOf, Served};
use fs_wiki_core::{
    Expect, NodeKind, Outcome, Placement, SearchFilter, SearchQuery, TextFormat, Texts, Tri,
};

fn doc(s: &str) -> fs_text::Document {
    TextFormat::Markdown.import(s)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_catalog_comes_over_the_socket() {
    let served = Served::in_memory().await;
    let registry = served.client().types().await.unwrap();
    assert_eq!(registry.len(), 5);
    assert!(registry
        .get("Page")
        .unwrap()
        .find_command("search")
        .is_some());
    assert!(registry
        .get("Page")
        .unwrap()
        .find_command("share")
        .is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn history_restore_and_purge_of_a_revision() {
    let served = Served::in_memory().await;
    let texts = served.client();
    let a = make(&texts, "a", "one\n").await;
    let b = texts
        .edit_with(a.id, Expect::Revision(a.revision), doc("two\n"), "second")
        .await
        .unwrap();
    let page = texts.history(a.id, None, 10).await.unwrap();
    assert_eq!(
        page.items.iter().map(|i| i.number).collect::<Vec<_>>(),
        [2, 1]
    );
    assert_eq!(page.items[0].summary, "second");
    let older = texts.history(a.id, Some(2), 10).await.unwrap();
    assert_eq!(older.items.len(), 1);
    let first = texts.get_revision(a.id, 1).await.unwrap().unwrap();
    assert_eq!(TextFormat::Markdown.export(&first.document), "one\n");
    assert!(texts.get_revision(a.id, 9).await.unwrap().is_none());
    let back = texts
        .restore_revision(a.id, 1, Expect::Revision(b.revision))
        .await
        .unwrap();
    assert_eq!(TextFormat::Markdown.export(&back.document), "one\n");
    // A purge follows a sync with the hub; the test server has none, and says so (a problem, not a silent success).
    let refused = texts.purge_revision(a.id, 1).await.unwrap_err();
    assert!(refused.to_string().contains("no hub"), "{refused}");
    assert!(texts.get_revision(a.id, 1).await.unwrap().is_some());
}

#[tokio::test(flavor = "multi_thread")]
async fn trash_restore_and_purge() {
    let served = Served::in_memory().await;
    let texts = served.client();
    let a = make(&texts, "a", "x\n").await;
    let item = texts.trash(a.id).await.unwrap();
    assert_eq!(item.path, "a");
    assert!(texts.get(a.id).await.unwrap().is_none());
    let page = texts.list_trash(None, 10).await.unwrap();
    assert_eq!(page.items.len(), 1);
    let after = texts
        .list_trash(Some(&page.items[0].place()), 10)
        .await
        .unwrap();
    assert!(after.items.is_empty());
    let back = texts.restore_from_trash(a.id, "").await.unwrap();
    assert_eq!(back.path, "a");
    texts.trash(a.id).await.unwrap();
    let refused = texts.purge(a.id).await.unwrap_err();
    assert!(refused.to_string().contains("no hub"), "{refused}");
    assert_eq!(texts.list_trash(None, 10).await.unwrap().items.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn folders_order_and_children() {
    let served = Served::in_memory().await;
    let texts = served.client();
    let space = texts.create_folder(None, "Manual", "").await.unwrap();
    assert_eq!(space.kind, NodeKind::Space);
    let sub = texts
        .create_folder(Some(space.id), "Start", "")
        .await
        .unwrap();
    let renamed = texts.rename_folder(sub.id, "Begin").await.unwrap();
    assert_eq!(renamed.name, "Begin");
    let p = make(&texts, &format!("{}/p", space.path), "x\n").await;
    let kids = texts.children(Some(space.id), 0, 10).await.unwrap();
    assert_eq!(kids.items.len(), 2);
    let moved = texts.reorder(p.id, Placement::First, None).await.unwrap();
    assert_eq!(moved.position, 0);
    assert_eq!(
        texts
            .children(Some(space.id), 1, 10)
            .await
            .unwrap()
            .items
            .len(),
        1
    );
    assert!(texts.get_folder(sub.id).await.unwrap().is_some());
    texts.delete_folder(sub.id).await.unwrap();
    assert!(texts.get_folder(sub.id).await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn templates_redirects_and_resolve() {
    let served = Served::in_memory().await;
    let texts = served.client();
    let t = make(&texts, "tpl", "# Form\n").await;
    let marked = texts.set_template(t.id, true, Expect::Any).await.unwrap();
    assert!(marked.template);
    let copy = texts.create_from_template(t.id, "new").await.unwrap();
    assert!(!copy.template && copy.path == "new");
    let red = texts
        .set_redirect(copy.id, "tpl", Expect::Any)
        .await
        .unwrap();
    assert_eq!(red.redirect_to, "tpl");
    let r = texts.resolve("new").await.unwrap();
    assert_eq!(r.outcome, Outcome::Resolved);
    assert_eq!(r.text.unwrap().path, "tpl");
    assert_eq!(
        texts.resolve("nowhere").await.unwrap().outcome,
        Outcome::NotFound
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn search_with_snippets_and_filters() {
    let served = Served::in_memory().await;
    let texts = served.client();
    make(&texts, "a", "the quick brown fox\n").await;
    let t = make(&texts, "b", "the slow brown bear\n").await;
    texts.set_template(t.id, true, Expect::Any).await.unwrap();
    let q = |term: &str, template: Tri| SearchQuery {
        term: term.into(),
        prefix: false,
        filter: SearchFilter {
            template,
            ..SearchFilter::default()
        },
    };
    let all = texts.search(&q("brown", Tri::Any), None, 10).await.unwrap();
    assert_eq!(all.items.len(), 2);
    assert!(!all.items[0].snippets.is_empty());
    let only = texts
        .search(&q("brown", Tri::Only), None, 10)
        .await
        .unwrap();
    assert_eq!(only.items.len(), 1);
    assert_eq!(only.items[0].path, "b");
    let rest = texts
        .search(&q("brown", Tri::Any), Some(&all.items[0].place()), 10)
        .await
        .unwrap();
    assert_eq!(rest.items.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_stale_revision_is_a_typed_conflict_in_every_write_that_takes_one() {
    use fs_wiki_core::Error;
    let served = Served::in_memory().await;
    let texts = served.client();
    let a = make(&texts, "a", "one\n").await;
    let b = texts
        .edit_with(a.id, Expect::Any, doc("two\n"), "")
        .await
        .unwrap();
    let stale = Expect::Revision(a.revision);
    for e in [
        texts.set_template(a.id, true, stale).await.unwrap_err(),
        texts.set_redirect(a.id, "x", stale).await.unwrap_err(),
        texts.restore_revision(a.id, 1, stale).await.unwrap_err(),
        texts
            .edit_with(a.id, stale, doc("three\n"), "")
            .await
            .unwrap_err(),
    ] {
        assert!(
            matches!(e, Error::Conflict { expected, found, .. } if expected == a.revision && found == b.revision),
            "{e:?}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_that_is_not_empty_and_a_page_that_is_not_in_the_trash_have_their_own_errors() {
    use fs_wiki_core::Error;
    let served = Served::in_memory().await;
    let texts = served.client();
    let space = texts.create_folder(None, "Manual", "").await.unwrap();
    make(&texts, &format!("{}/p", space.path), "x\n").await;
    let e = texts.delete_folder(space.id).await.unwrap_err();
    assert!(matches!(e, Error::NotEmpty(id) if id == space.id), "{e:?}");
    let e = texts.get_folder(fs_objects::ObjectId::new()).await;
    assert!(e.is_ok());
    let p = make(&texts, "q", "x\n").await;
    let e = texts.restore_from_trash(p.id, "").await.unwrap_err();
    assert!(matches!(e, Error::NotInTrash(id) if id == p.id), "{e:?}");
}
