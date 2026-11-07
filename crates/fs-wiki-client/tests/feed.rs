//! `TextFeed::subscribe` over the socket: changes arrive, a dropped subscription is silent, the
//! end of the connection is reported.
//!
//! Pattern: Integration tests of the Observer end to end; the listener is the only test double and
//! only records what it is told.

mod common;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use common::{make, ClientOf, Served};
use fs_wiki_core::{TextChange, TextFeed, TextFormat, TextListener, Texts};

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

/// Wait until `ready` holds, at most five seconds.
async fn until(ready: impl Fn() -> bool) {
    let start = Instant::now();
    while !ready() {
        assert!(start.elapsed() < Duration::from_secs(5), "timed out");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn changes_arrive_as_created_updated_and_deleted() {
    let served = Served::in_memory().await;
    let texts = served.client();
    let seen = Arc::new(Seen::default());
    let _subscription = texts.subscribe(seen.clone());
    let a = make(&texts, "a", "one\n").await;
    texts
        .edit(a.id, TextFormat::Markdown.import("two\n"))
        .await
        .unwrap();
    texts.delete(a.id).await.unwrap();
    until(|| seen.changes.lock().unwrap().len() == 3).await;
    let want = [
        TextChange::Created(a.id),
        TextChange::Updated(a.id),
        TextChange::Deleted(a.id),
    ];
    assert_eq!(*seen.changes.lock().unwrap(), want);
}

#[tokio::test(flavor = "multi_thread")]
async fn another_connection_is_told_about_a_write_of_this_one() {
    let served = Served::in_memory().await;
    let (writer, reader) = (served.client(), served.client());
    let seen = Arc::new(Seen::default());
    let _subscription = reader.subscribe(seen.clone());
    let a = make(&writer, "a", "x\n").await;
    until(|| !seen.changes.lock().unwrap().is_empty()).await;
    assert_eq!(*seen.changes.lock().unwrap(), [TextChange::Created(a.id)]);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_dropped_subscription_hears_nothing_more_and_a_second_listener_goes_on() {
    let served = Served::in_memory().await;
    let texts = served.client();
    let (first, second) = (Arc::new(Seen::default()), Arc::new(Seen::default()));
    let gone = texts.subscribe(first.clone());
    let _kept = texts.subscribe(second.clone());
    make(&texts, "a", "x\n").await;
    until(|| first.changes.lock().unwrap().len() == 1).await;
    drop(gone);
    make(&texts, "b", "y\n").await;
    until(|| second.changes.lock().unwrap().len() == 2).await;
    assert_eq!(first.changes.lock().unwrap().len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn the_end_of_the_connection_is_reported_to_the_listener() {
    let mut served = Served::in_memory().await;
    let texts = served.client();
    let seen = Arc::new(Seen::default());
    let _subscription = texts.subscribe(seen.clone());
    make(&texts, "a", "x\n").await;
    until(|| !seen.changes.lock().unwrap().is_empty()).await;
    served.stop();
    until(|| !seen.ended.lock().unwrap().is_empty()).await;
    assert_eq!(seen.ended.lock().unwrap().len(), 1, "told once");
}

#[tokio::test(flavor = "multi_thread")]
async fn subscribing_to_a_server_that_is_gone_is_reported_at_once() {
    let mut served = Served::in_memory().await;
    let texts = served.client();
    served.stop();
    let seen = Arc::new(Seen::default());
    let _subscription = texts.subscribe(seen.clone());
    until(|| !seen.ended.lock().unwrap().is_empty()).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn dropping_the_client_with_a_running_subscription_does_not_hang() {
    let served = Served::in_memory().await;
    let texts = served.client();
    let _subscription = texts.subscribe(Arc::new(Seen::default()));
    let done = tokio::task::spawn_blocking(move || drop(texts));
    tokio::time::timeout(Duration::from_secs(10), done)
        .await
        .expect("no hang")
        .unwrap();
}
