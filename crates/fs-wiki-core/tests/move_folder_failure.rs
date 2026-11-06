//! `move_folder` under failure, undo and size: an interrupted move is finished by repeating it,
//! the undo steps are the writes, a large folder is read in batches. Real in-memory `TursoStore`.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use common::{everything, fill, md, paths, setup, ME};
use fs_store::traits::{ObjectQuery, ObjectRepository};
use fs_store::{
    Activity, Error as StoreError, LogEntry, ObjectId, Result as StoreResult, StoredObject,
    TursoStore,
};
use fs_wiki_core::{Error, Paging, TextEntry, Texts, WikiRepo};

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
