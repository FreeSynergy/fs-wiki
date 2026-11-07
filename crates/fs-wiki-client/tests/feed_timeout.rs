//! A subscription that fails once (the server is busy and the deadline runs out) fails that one
//! `subscribe`; the connection lives, and the next `subscribe` works.
//!
//! Pattern: Integration test of the Observer against the real server. The only stand-in is the
//! provider in front of it, which delegates to the real wiki and holds the server thread for a while
//! on one `list` when told to (a busy server).

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use common::Served;
use fs_interfaces::Capabilities;
use fs_objects::ObjectId;
use fs_wiki_client::{RpcTexts, Timeouts};
use fs_wiki_core::{
    Paging, Result, TextChange, TextEntry, TextFeed, TextListener, Texts, WikiRepo,
};

/// The real wiki; `list` holds the server thread for 3 s once, while `slow` is set.
struct Busy {
    repo: Arc<WikiRepo>,
    slow: Arc<AtomicBool>,
}

#[async_trait]
impl Texts for Busy {
    fn capabilities(&self) -> Capabilities {
        self.repo.capabilities()
    }
    async fn get(&self, id: ObjectId) -> Result<Option<TextEntry>> {
        self.repo.get(id).await
    }
    async fn get_by_path(&self, path: &str) -> Result<Option<TextEntry>> {
        self.repo.get_by_path(path).await
    }
    async fn list(&self, prefix: &str, page: Paging) -> Result<Vec<TextEntry>> {
        if self.slow.swap(false, Ordering::SeqCst) {
            std::thread::sleep(Duration::from_secs(3));
        }
        self.repo.list(prefix, page).await
    }
}

#[derive(Default)]
struct Seen {
    changes: Mutex<Vec<TextChange>>,
    ended: Mutex<Vec<String>>,
}

impl TextListener for Seen {
    fn changed(&self, c: &TextChange) {
        self.changes.lock().unwrap().push(*c);
    }
    fn ended(&self, reason: &str) {
        self.ended.lock().unwrap().push(reason.to_string());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn one_subscription_that_runs_out_of_time_does_not_kill_the_feed_of_the_connection() {
    let plain = Served::in_memory().await;
    let slow = Arc::new(AtomicBool::new(false));
    let busy = Arc::new(Busy {
        repo: plain.repo.clone(),
        slow: slow.clone(),
    });
    let served = Served::serving(busy, plain.repo.clone());
    let short = Timeouts {
        call: Duration::from_secs(1),
        ..Timeouts::default()
    };
    let texts = Arc::new(RpcTexts::connect_with(&served.socket, short).unwrap());

    // The server is busy for 3 s; the subscription meanwhile times out after 1 s.
    slow.store(true, Ordering::SeqCst);
    let lister = {
        let t = texts.clone();
        tokio::spawn(async move {
            let _ = t.list("", Paging::default()).await;
        })
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    let first = Arc::new(Seen::default());
    let (t, l) = (texts.clone(), first.clone());
    let _first = tokio::task::spawn_blocking(move || t.subscribe(l))
        .await
        .unwrap();
    let said = first.ended.lock().unwrap().clone();
    assert_eq!(said.len(), 1, "told once: {said:?}");
    assert!(said[0].starts_with("subscription failed"), "{said:?}");
    assert!(
        !said[0].contains("gone"),
        "the connection is not gone: {said:?}"
    );
    let _ = lister.await;
    tokio::time::sleep(Duration::from_secs(3)).await;

    // The connection lives, and a new subscribe works and gets the changes.
    assert!(texts.get_by_path("nothing").await.unwrap().is_none());
    let newcomer = Arc::new(Seen::default());
    let (t, l) = (texts.clone(), newcomer.clone());
    let _n = tokio::task::spawn_blocking(move || t.subscribe(l))
        .await
        .unwrap();
    let a = served.repo.create("a", "x\n").await.unwrap();
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(newcomer.ended.lock().unwrap().is_empty());
    assert_eq!(
        *newcomer.changes.lock().unwrap(),
        [TextChange::Created(a.id)]
    );
}
