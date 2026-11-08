//! A feed that the server ends (`ChangeListener.ended`): the old end never touches a newer
//! subscription, and a listener may subscribe again from inside `ended`.
//!
//! Pattern: Integration tests of the Observer against the real server. The only stand-in is the
//! provider in front of it, which delegates to the real wiki and fails a read when told to: that is
//! the one way the real server ends a feed (a change it cannot tell).

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use common::{ClientOf, Served};
use fs_interfaces::Capabilities;
use fs_objects::ObjectId;
use fs_wiki_client::RpcTexts;
use fs_wiki_core::{
    Paging, Place, Result, TextChange, TextEntry, TextFeed, TextListener, TextPage, TextQuery,
    Texts, WikiRepo,
};

/// The real wiki, whose `get` fails while `fail` is set.
struct Flaky {
    repo: Arc<WikiRepo>,
    fail: Arc<AtomicBool>,
}

#[async_trait]
impl Texts for Flaky {
    fn capabilities(&self) -> Capabilities {
        self.repo.capabilities()
    }
    async fn get(&self, id: ObjectId) -> Result<Option<TextEntry>> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(fs_objects::Error::Backend("the read failed".into()).into());
        }
        self.repo.get(id).await
    }
    async fn page(
        &self,
        query: &TextQuery,
        after: Option<&Place>,
        size: usize,
    ) -> Result<TextPage> {
        self.repo.page(query, after, size).await
    }
    async fn get_by_path(&self, path: &str) -> Result<Option<TextEntry>> {
        self.repo.get_by_path(path).await
    }
    async fn list(&self, prefix: &str, page: Paging) -> Result<Vec<TextEntry>> {
        self.repo.list(prefix, page).await
    }
}

async fn flaky() -> (Served, Arc<AtomicBool>) {
    let plain = Served::in_memory().await;
    let fail = Arc::new(AtomicBool::new(false));
    let texts = Arc::new(Flaky {
        repo: plain.repo.clone(),
        fail: fail.clone(),
    });
    (Served::serving(texts, plain.repo.clone()), fail)
}

#[derive(Default)]
struct Seen {
    changes: Mutex<Vec<TextChange>>,
    ended: Mutex<Vec<String>>,
}

impl TextListener for Seen {
    fn changed(&self, change: &TextChange) {
        self.changes.lock().unwrap().push(*change);
    }
    fn ended(&self, reason: &str) {
        self.ended.lock().unwrap().push(reason.to_string());
    }
}

async fn until(ready: impl Fn() -> bool) {
    let start = Instant::now();
    while !ready() {
        assert!(start.elapsed() < Duration::from_secs(5), "timed out");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

async fn pause() {
    tokio::time::sleep(Duration::from_millis(400)).await;
}

/// Tells a channel in `ended`, then holds the connection thread a moment, so that another thread can
/// subscribe before the old listener object of the connection is released.
struct Slow {
    tx: Mutex<Option<std::sync::mpsc::Sender<()>>>,
}

impl TextListener for Slow {
    fn changed(&self, _: &TextChange) {}
    fn ended(&self, _: &str) {
        if let Some(tx) = self.tx.lock().unwrap().take() {
            tx.send(()).unwrap();
        }
        std::thread::sleep(Duration::from_millis(300));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_end_of_an_old_feed_leaves_a_new_subscription_alone() {
    let (served, fail) = flaky().await;
    let texts = Arc::new(served.client());
    let (tx, rx) = std::sync::mpsc::channel();
    let _old = texts.subscribe(Arc::new(Slow {
        tx: Mutex::new(Some(tx)),
    }));
    // Another thread subscribes while the old listener holds the connection thread in `ended`.
    let fresh = Arc::new(Seen::default());
    let (t2, f2) = (texts.clone(), fresh.clone());
    let joiner = std::thread::spawn(move || {
        rx.recv().unwrap();
        t2.subscribe(f2)
    });
    fail.store(true, Ordering::SeqCst);
    served.repo.create("a", "x\n").await.unwrap();
    let _kept = tokio::task::spawn_blocking(move || joiner.join().unwrap())
        .await
        .unwrap();
    fail.store(false, Ordering::SeqCst);
    pause().await;
    assert!(
        fresh.ended.lock().unwrap().is_empty(),
        "the old end ended the new one"
    );
    let b = served.repo.create("b", "y\n").await.unwrap();
    pause().await;
    // And a third listener joins the running feed: no second remote subscription, no doubles.
    let third = Arc::new(Seen::default());
    let _third = texts.subscribe(third.clone());
    let c = served.repo.create("c", "z\n").await.unwrap();
    pause().await;
    assert_eq!(*third.changes.lock().unwrap(), [TextChange::Created(c.id)]);
    assert_eq!(
        *fresh.changes.lock().unwrap(),
        [TextChange::Created(b.id), TextChange::Created(c.id)]
    );
}

/// Subscribes again from inside `ended`, on the connection's own thread.
struct Resubscribes {
    texts: OnceLock<Arc<RpcTexts>>,
    next: Arc<Seen>,
    kept: Mutex<Vec<fs_wiki_core::Subscription>>,
}

impl TextListener for Resubscribes {
    fn changed(&self, _: &TextChange) {}
    fn ended(&self, _: &str) {
        let texts = self.texts.get().unwrap();
        let again = texts.subscribe(self.next.clone()).unwrap();
        self.kept.lock().unwrap().push(again);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn subscribing_from_inside_ended_does_not_block_the_connection() {
    let (served, fail) = flaky().await;
    let texts = Arc::new(served.client());
    let next = Arc::new(Seen::default());
    let listener = Arc::new(Resubscribes {
        texts: OnceLock::new(),
        next: next.clone(),
        kept: Mutex::default(),
    });
    listener.texts.set(texts.clone()).ok();
    let _first = texts.subscribe(listener.clone());
    fail.store(true, Ordering::SeqCst);
    served.repo.create("a", "x\n").await.unwrap();
    until(|| !listener.kept.lock().unwrap().is_empty()).await;
    fail.store(false, Ordering::SeqCst);
    let used = tokio::time::timeout(Duration::from_secs(5), texts.get_by_path("a"))
        .await
        .expect("the connection still answers");
    assert!(used.unwrap().is_some());
    served.repo.create("b", "y\n").await.unwrap();
    until(|| next.changes.lock().unwrap().len() == 1).await;
}
