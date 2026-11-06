//! `move_folder`: the text at a path and everything below it move, links follow. Every test runs
//! on a real in-memory `TursoStore`.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use common::{paths, setup, ME};
use fs_store::traits::{ObjectQuery, ObjectRepository};
use fs_store::{
    Activity, Error as StoreError, LogEntry, ObjectId, Result as StoreResult, StoredObject,
    TursoStore,
};
use fs_wiki_core::{Error, Paging, TextEntry, Texts, WikiRepo, MARKDOWN};

/// The standard wiki: a folder `manual` with a text of its own, children, a lookalike, and texts
/// elsewhere that link in. Returns the ids of `[manual, manual/a, manual/deep/b, other, lookalike]`.
async fn fill(w: &WikiRepo) -> Vec<ObjectId> {
    let mut ids = vec![];
    for (path, md) in [
        ("manual", "# Manual\n\nSee [[manual/a]] and [[manual/deep/b#part|b]].\n"),
        ("manual/a", "Back to [[manual]], up [x](manual/deep/b), self [[manual/a#s]].\n"),
        ("manual/deep/b", "Link [[manual/a]] and a dangling [[manual/nothing-here]].\n"),
        ("other", "[[manual]] [[manual#top]] [[manual/a]] [[manual/deep/b#h]] [[manual-old]] [[mine]] [e](https://e.org/manual/a)\n"),
        ("manual-old", "Not part of it: [[manual/a]]\n"),
    ] {
        ids.push(w.create(path, md).await.unwrap().id);
    }
    ids
}

async fn md(w: &WikiRepo, id: ObjectId) -> String {
    w.export_text(id, MARKDOWN).await.unwrap()
}

async fn everything(w: &WikiRepo) -> Vec<TextEntry> {
    w.list("", Paging::default()).await.unwrap()
}

#[tokio::test]
async fn a_folder_moves_with_everything_below_and_links_follow() {
    let w = common::wiki().await;
    let ids = fill(&w).await;
    let before_old = w.get(ids[4]).await.unwrap().unwrap();

    let moved = w.move_folder("manual", "guide").await.unwrap();
    assert_eq!(paths(&moved), ["guide", "guide/a", "guide/deep/b"]);
    assert_eq!(
        moved.iter().map(|t| t.id).collect::<Vec<_>>(),
        ids[..3].to_vec()
    );

    let all = everything(&w).await;
    assert_eq!(
        paths(&all),
        ["guide", "guide/a", "guide/deep/b", "manual-old", "other"]
    );
    assert_eq!(
        md(&w, ids[0]).await,
        "# Manual\n\nSee [[guide/a]] and [[guide/deep/b#part|b]].\n"
    );
    assert_eq!(
        md(&w, ids[1]).await,
        "Back to [[guide]], up [x](guide/deep/b), self [[guide/a#s]].\n"
    );
    assert_eq!(
        md(&w, ids[2]).await,
        "Link [[guide/a]] and a dangling [[guide/nothing-here]].\n"
    );
    assert_eq!(
        md(&w, ids[3]).await,
        "[[guide]] [[guide#top]] [[guide/a]] [[guide/deep/b#h]] [[manual-old]] [[mine]] [e](https://e.org/manual/a)\n"
    );
    // A lookalike does not move; its link into the folder does follow.
    let old = w.get(ids[4]).await.unwrap().unwrap();
    assert_eq!(old.path, "manual-old");
    assert_eq!(md(&w, ids[4]).await, "Not part of it: [[guide/a]]\n");
    assert_ne!(old.document, before_old.document);

    // The link table agrees: nothing points into the old folder any more.
    assert!(w.backlinks("manual/a").await.unwrap().is_empty());
    assert!(w.backlinks("manual").await.unwrap().is_empty());
    assert_eq!(w.backlinks("guide/a").await.unwrap().len(), 5);
}

#[tokio::test]
async fn a_folder_without_a_text_of_its_own_moves_too() {
    let w = common::wiki().await;
    let a = w.create("docs/x/one", "[[docs/x/two]]").await.unwrap();
    w.create("docs/x/two", "t").await.unwrap();
    let moved = w.move_folder("docs/x", "archive/y").await.unwrap();
    assert_eq!(paths(&moved), ["archive/y/one", "archive/y/two"]);
    assert_eq!(md(&w, a.id).await, "[[archive/y/two]]\n");
    // Moving up into an ancestor works while no name repeats.
    let up = w.move_folder("archive/y", "archive").await.unwrap();
    assert_eq!(paths(&up), ["archive/one", "archive/two"]);
}

#[tokio::test]
async fn a_clash_anywhere_below_refuses_everything_before_writing() {
    let (w, store) = setup().await;
    fill(&w).await;
    w.create("guide/deep/b", "taken").await.unwrap();
    let before = everything(&w).await;
    let log_before = store.undo_state(ME).await.unwrap();

    let err = w.move_folder("manual", "guide").await.unwrap_err();
    assert!(
        matches!(&err, Error::PathTaken(p) if p == "guide/deep/b"),
        "{err:?}"
    );
    assert_eq!(everything(&w).await, before, "no text was touched");
    assert_eq!(
        store.undo_state(ME).await.unwrap(),
        log_before,
        "not one write reached the store"
    );

    // The clash may be the folder's own text, too.
    let w2 = common::wiki().await;
    fill(&w2).await;
    w2.create("guide", "taken").await.unwrap();
    assert!(matches!(
        w2.move_folder("manual", "guide").await,
        Err(Error::PathTaken(p)) if p == "guide"
    ));
}

#[tokio::test]
async fn into_itself_below_itself_and_onto_itself() {
    let (w, store) = setup().await;
    fill(&w).await;
    let state = store.undo_state(ME).await.unwrap();
    for target in ["manual/inside", "manual/deep/er"] {
        assert!(matches!(
            w.move_folder("manual", target).await,
            Err(Error::InvalidMove { .. })
        ));
    }
    // A lookalike name is not "inside".
    assert!(w.move_folder("manual/deep", "manual-deep").await.is_ok());
    w.move_folder("manual-deep", "manual/deep").await.unwrap();
    let state2 = store.undo_state(ME).await.unwrap();
    assert_ne!(state, state2);

    let snapshot = everything(&w).await;
    let same = w.move_folder("manual", "manual").await.unwrap();
    assert_eq!(paths(&same), ["manual", "manual/a", "manual/deep/b"]);
    assert_eq!(everything(&w).await, snapshot);
    assert_eq!(
        store.undo_state(ME).await.unwrap(),
        state2,
        "nothing written"
    );

    assert!(matches!(
        w.move_folder("nothing", "x").await,
        Err(Error::NoSuchFolder(_))
    ));
    assert!(matches!(
        w.move_folder("manual", "../x").await,
        Err(Error::InvalidPath { .. })
    ));
}

#[tokio::test]
async fn moving_up_where_a_name_repeats_is_refused_before_writing() {
    // a/b/b/x -> a/b/x would land inside the old folder a/b: an interrupted move of that kind could
    // not be told from a finished one.
    let w = common::wiki().await;
    w.create("a/b", "1").await.unwrap();
    w.create("a/b/b/x", "2").await.unwrap();
    let before = everything(&w).await;
    assert!(matches!(
        w.move_folder("a/b", "a").await,
        Err(Error::InvalidMove { .. })
    ));
    assert_eq!(everything(&w).await, before);
}

/// A store that stops accepting writes after `left` of them (reads always work).
struct Failing {
    inner: Arc<TursoStore>,
    left: AtomicUsize,
}

impl Failing {
    fn gate(&self) -> StoreResult<()> {
        let left = self.left.load(Ordering::SeqCst);
        if left == 0 {
            return Err(StoreError::Backend("injected failure".into()));
        }
        self.left.store(left - 1, Ordering::SeqCst);
        Ok(())
    }
}

#[async_trait]
impl ObjectRepository for Failing {
    async fn put(&self, object: StoredObject, actor: &str) -> StoreResult<Activity> {
        self.gate()?;
        ObjectRepository::put(&*self.inner, object, actor).await
    }
    async fn put_and_log(
        &self,
        object: StoredObject,
        actor: &str,
        extra: Vec<LogEntry>,
    ) -> StoreResult<Vec<Activity>> {
        self.gate()?;
        ObjectRepository::put_and_log(&*self.inner, object, actor, extra).await
    }
    async fn get(&self, id: ObjectId) -> StoreResult<Option<StoredObject>> {
        ObjectRepository::get(&*self.inner, id).await
    }
    async fn delete(&self, id: ObjectId, actor: &str) -> StoreResult<Activity> {
        self.gate()?;
        ObjectRepository::delete(&*self.inner, id, actor).await
    }
}

/// A wiki over `store` whose writes stop after `left`.
fn failing(store: &Arc<TursoStore>, left: usize) -> WikiRepo {
    let objects = Arc::new(Failing {
        inner: store.clone(),
        left: AtomicUsize::new(left),
    });
    let reads: Arc<dyn ObjectQuery> = store.clone();
    WikiRepo::with_backend(objects, reads, ME)
}

fn sorted_ids(texts: &[TextEntry]) -> Vec<ObjectId> {
    let mut ids: Vec<_> = texts.iter().map(|t| t.id).collect();
    ids.sort_by_key(ToString::to_string);
    ids
}

#[tokio::test]
async fn an_interrupted_folder_move_is_finished_by_repeating_it() {
    // Writes of the standard move: `other` and `manual-old` (outside, phase 1), then 3 texts.
    const WRITES: usize = 5;
    // The expected end state, from an undisturbed run.
    let reference = common::wiki().await;
    fill(&reference).await;
    reference.move_folder("manual", "guide").await.unwrap();
    let expected: Vec<(String, String)> = {
        let mut v = vec![];
        for t in everything(&reference).await {
            v.push((t.path.clone(), md(&reference, t.id).await));
        }
        v
    };

    for fail_at in 0..WRITES {
        let store = Arc::new(TursoStore::open_in_memory().await.unwrap());
        let healthy = WikiRepo::new(store.clone(), ME);
        fill(&healthy).await;
        let ids_before = sorted_ids(&everything(&healthy).await);

        let broken = failing(&store, fail_at);
        let err = broken.move_folder("manual", "guide").await.unwrap_err();
        assert!(matches!(err, Error::Store(_)), "at {fail_at}: {err:?}");

        // In between: nothing lost, nothing twice, every path still unique.
        let mid = everything(&healthy).await;
        assert_eq!(sorted_ids(&mid), ids_before, "at {fail_at}");
        let mut p: Vec<_> = paths(&mid);
        p.dedup();
        assert_eq!(p.len(), mid.len(), "at {fail_at}: a path twice");

        // Repeating the same call finishes the job.
        let moved = healthy.move_folder("manual", "guide").await.unwrap();
        let rest = fail_at.saturating_sub(2); // texts of the folder already moved
        assert_eq!(moved.len(), 3 - rest, "at {fail_at}: only what was left");
        let after = everything(&healthy).await;
        assert_eq!(sorted_ids(&after), ids_before, "at {fail_at}");
        let mut got = vec![];
        for t in &after {
            got.push((t.path.clone(), md(&healthy, t.id).await));
        }
        assert_eq!(got, expected, "at {fail_at}");
    }
}

#[tokio::test]
async fn a_folder_move_is_one_undo_step_per_write() {
    let (w, store) = setup().await;
    let ids = fill(&w).await;
    let start = everything(&w).await;
    let mut start_md = vec![];
    for t in &start {
        start_md.push((t.path.clone(), md(&w, t.id).await));
    }
    let steps_before = store.undo_state(ME).await.unwrap();

    w.move_folder("manual", "guide").await.unwrap();
    let steps_after = store.undo_state(ME).await.unwrap();
    assert_ne!(steps_before, steps_after);

    // Undoing the last write puts back the last moved text only: a half moved folder.
    store.undo(ME, 1).await.unwrap();
    let paths_now: Vec<String> = everything(&w).await.into_iter().map(|t| t.path).collect();
    assert_eq!(
        paths_now,
        ["guide", "guide/a", "manual-old", "manual/deep/b", "other"],
        "the deepest text, moved last, is back; the rest stays moved"
    );
    // Repeating the move finishes it again, as after an interruption.
    w.move_folder("manual", "guide").await.unwrap();

    // Undoing every write of the move (2 outside texts + 3 moved) restores the start.
    store.undo(ME, 5).await.unwrap();
    let mut back = vec![];
    for t in everything(&w).await {
        back.push((t.path.clone(), md(&w, t.id).await));
    }
    assert_eq!(back, start_md);
    assert_eq!(sorted_ids(&everything(&w).await), {
        let mut v = ids.clone();
        v.sort_by_key(ToString::to_string);
        v
    });
}

#[tokio::test]
async fn a_large_folder_moves_in_batches() {
    let w = common::wiki().await;
    for i in 0..510 {
        w.create(
            &format!("big/p{i:04}"),
            &format!("[[big/p{:04}]]", (i + 1) % 510),
        )
        .await
        .unwrap();
    }
    w.create("elsewhere/x", "[[big/p0007]]").await.unwrap();
    let moved = w.move_folder("big", "huge").await.unwrap();
    assert_eq!(moved.len(), 510);
    assert_eq!(w.list("big", Paging::default()).await.unwrap().len(), 0);
    assert_eq!(w.list("huge/", Paging::default()).await.unwrap().len(), 510);
    assert_eq!(w.backlinks("huge/p0007").await.unwrap().len(), 2);
}
