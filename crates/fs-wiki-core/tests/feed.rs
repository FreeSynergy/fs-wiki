//! `text.subscribe`: a listener hears every write once it is done, and nothing else.
//!
//! Pattern: Example-based integration tests of the Observer, through `dyn TextFeed`.

mod common;

use std::sync::{Arc, Mutex};

use common::{failing, setup, wiki};
use fs_objects::TursoStore;
use fs_text::Document;
use fs_wiki_core::{TextChange, TextFeed, TextListener, Texts, MARKDOWN};

#[derive(Default)]
struct Log(Mutex<Vec<TextChange>>);

impl TextListener for Log {
    fn changed(&self, change: &TextChange) {
        self.0.lock().unwrap().push(*change);
    }
}

impl Log {
    fn take(&self) -> Vec<TextChange> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

#[tokio::test]
async fn create_edit_and_delete_are_told_in_order() {
    let w = wiki().await;
    let feed: &dyn TextFeed = &w;
    let log = Arc::new(Log::default());
    let _sub = feed.subscribe(log.clone());

    let t = w.create("a", "one\n").await.unwrap();
    let mut doc = t.document.clone();
    doc.id = t.document.id;
    let again = w.edit(t.id, doc.clone()).await.unwrap();
    w.edit_if(t.id, again.revision, doc).await.unwrap();
    w.delete(t.id).await.unwrap();
    assert_eq!(
        log.take(),
        [
            TextChange::Created(t.id),
            TextChange::Updated(t.id),
            TextChange::Updated(t.id),
            TextChange::Deleted(t.id)
        ]
    );
}

#[tokio::test]
async fn what_is_refused_is_not_told() {
    let w = wiki().await;
    let log = Arc::new(Log::default());
    let _sub = w.subscribe(log.clone());
    let t = w.create("a", "x\n").await.unwrap();
    log.take();
    assert!(w.create("a", "again\n").await.is_err(), "path taken");
    assert!(w
        .edit_if(t.id, t.revision + 1, Document::new())
        .await
        .is_err());
    assert!(w.move_to(t.id, "bad//path").await.is_err());
    assert!(w.delete(fs_objects::ObjectId::new()).await.is_err());
    assert_eq!(log.take(), [], "nothing was written");
}

#[tokio::test]
async fn a_write_the_store_refuses_is_not_told() {
    let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let w = failing(&store, 1);
    let log = Arc::new(Log::default());
    let _sub = w.subscribe(log.clone());
    let t = w.create("a", "x\n").await.unwrap();
    assert!(w.edit(t.id, t.document.clone()).await.is_err());
    assert_eq!(log.take(), [TextChange::Created(t.id)]);
}

#[tokio::test]
async fn an_import_is_a_create_an_update_or_nothing() {
    let w = wiki().await;
    let log = Arc::new(Log::default());
    let _sub = w.subscribe(log.clone());
    let t = w.import_text("p", MARKDOWN, "one\n").await.unwrap();
    w.import_text("p", MARKDOWN, "one\n").await.unwrap();
    w.import_text("p", MARKDOWN, "two\n").await.unwrap();
    assert_eq!(
        log.take(),
        [TextChange::Created(t.id), TextChange::Updated(t.id)]
    );
}

#[tokio::test]
async fn a_move_tells_every_text_it_wrote_the_moved_one_last() {
    let (w, _) = setup().await;
    let a = w.create("a", "x\n").await.unwrap();
    let b = w.create("b", "see [[a]]\n").await.unwrap();
    let c = w.create("c", "no link\n").await.unwrap();
    let log = Arc::new(Log::default());
    let _sub = w.subscribe(log.clone());
    w.move_to(a.id, "z").await.unwrap();
    assert_eq!(
        log.take(),
        [TextChange::Updated(b.id), TextChange::Updated(a.id)]
    );
    let _ = c;
    let folder = common::fill(&w).await;
    log.take();
    w.move_folder("manual", "guide").await.unwrap();
    let told = log.take();
    for id in &folder[..3] {
        assert!(told.contains(&TextChange::Updated(*id)), "{id}");
    }
    assert!(told.iter().all(|c| matches!(c, TextChange::Updated(_))));
}

#[tokio::test]
async fn dropping_the_subscription_ends_it_and_listeners_are_independent() {
    let w = wiki().await;
    let (one, two) = (Arc::new(Log::default()), Arc::new(Log::default()));
    let first = w.subscribe(one.clone());
    let _second = w.subscribe(two.clone());
    let t = w.create("a", "x\n").await.unwrap();
    drop(first);
    w.delete(t.id).await.unwrap();
    assert_eq!(one.take(), [TextChange::Created(t.id)]);
    assert_eq!(
        two.take(),
        [TextChange::Created(t.id), TextChange::Deleted(t.id)]
    );
}

#[derive(Default)]
struct Ends(Mutex<Vec<String>>);

impl TextListener for Ends {
    fn changed(&self, _: &TextChange) {}
    fn ended(&self, reason: &str) {
        self.0.lock().unwrap().push(reason.to_string());
    }
}

#[test]
fn ending_the_feed_tells_each_listener_once_and_forgets_it() {
    let registry = Arc::new(fs_wiki_core::Registry::default());
    let (a, b) = (Arc::new(Ends::default()), Arc::new(Ends::default()));
    let (_kept, dropped) = (registry.add(a.clone()), registry.add(b.clone()));
    drop(dropped);
    registry.end("gone");
    registry.end("again");
    assert_eq!(*a.0.lock().unwrap(), ["gone"]);
    assert_eq!(b.0.lock().unwrap().len(), 0, "it had left already");
    // A listener that does not care keeps the default: nothing happens.
    let _plain = registry.add(Arc::new(Log::default()));
    registry.end("ignored");
}
