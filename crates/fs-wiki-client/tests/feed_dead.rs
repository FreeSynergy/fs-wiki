//! A feed on a connection that is gone: a listener that subscribes again in `ended`, as the doc of
//! `TextListener::ended` allows, must not make the connection thread loop.
//!
//! Pattern: Integration test of the Observer against the real server, which is stopped.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use common::{ClientOf, Served};
use fs_wiki_client::RpcTexts;
use fs_wiki_core::{TextChange, TextFeed, TextListener};

#[derive(Default)]
struct Seen {
    ended: Mutex<Vec<String>>,
}

impl TextListener for Seen {
    fn changed(&self, _: &TextChange) {}
    fn ended(&self, reason: &str) {
        self.ended.lock().unwrap().push(reason.to_string());
    }
}

/// Always subscribes again from inside `ended`, as the doc of `TextListener::ended` allows.
struct Always {
    texts: OnceLock<Arc<RpcTexts>>,
    me: OnceLock<std::sync::Weak<Always>>,
    ends: AtomicUsize,
    kept: Mutex<Vec<fs_wiki_core::Subscription>>,
}

impl TextListener for Always {
    fn changed(&self, _: &TextChange) {}
    fn ended(&self, _: &str) {
        self.ends.fetch_add(1, Ordering::SeqCst);
        let me = self.me.get().unwrap().upgrade().unwrap();
        let again = self.texts.get().unwrap().subscribe(me).unwrap();
        let mut kept = self.kept.lock().unwrap();
        if kept.len() > 4 {
            drop(kept.remove(0));
        }
        kept.push(again);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_listener_that_subscribes_again_in_ended_does_not_loop_on_a_dead_connection() {
    let mut served = Served::in_memory().await;
    let texts = Arc::new(served.client());
    let listener = Arc::new(Always {
        texts: OnceLock::new(),
        me: OnceLock::new(),
        ends: AtomicUsize::new(0),
        kept: Mutex::default(),
    });
    listener.texts.set(texts.clone()).ok();
    listener.me.set(Arc::downgrade(&listener)).ok();
    let _first = texts.subscribe(listener.clone());
    served.stop();
    tokio::time::sleep(Duration::from_secs(2)).await;
    let early = listener.ends.load(Ordering::SeqCst);
    tokio::time::sleep(Duration::from_secs(1)).await;
    let late = listener.ends.load(Ordering::SeqCst);
    assert!(early >= 1, "it was told");
    assert_eq!(early, late, "and it stopped being told");
    assert!(late < 10, "{late}");
    // A new listener on the dead connection hears it once, not inside `subscribe` but soon after.
    let newcomer = Arc::new(Seen::default());
    let _n = texts.subscribe(newcomer.clone());
    assert!(
        newcomer.ended.lock().unwrap().is_empty(),
        "not synchronously"
    );
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(newcomer.ended.lock().unwrap().len(), 1);
}

/// Subscribes a NEW object each time it is told, as an adapter made afresh would.
struct Fresh {
    texts: Arc<RpcTexts>,
    calls: Arc<AtomicUsize>,
    kept: Arc<Mutex<Vec<fs_wiki_core::Subscription>>>,
}

impl TextListener for Fresh {
    fn changed(&self, _: &TextChange) {}
    fn ended(&self, _: &str) {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let next = Arc::new(Fresh {
            texts: self.texts.clone(),
            calls: self.calls.clone(),
            kept: self.kept.clone(),
        });
        let again = self.texts.subscribe(next).unwrap();
        self.kept.lock().unwrap().push(again);
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn fresh_listener_objects_on_a_dead_connection_neither_overflow_the_stack_nor_spin() {
    let mut served = Served::in_memory().await;
    let texts = Arc::new(served.client());
    served.stop();
    let calls = Arc::new(AtomicUsize::new(0));
    let (t, c) = (texts.clone(), calls.clone());
    // A small stack: a recursion through `subscribe` and `ended` would not survive it.
    let small = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || {
            let first = Arc::new(Fresh {
                texts: t.clone(),
                calls: c,
                kept: Arc::default(),
            });
            t.subscribe(first)
        })
        .unwrap();
    let _kept = small.join().unwrap();
    tokio::time::sleep(Duration::from_millis(1500)).await;
    let n = calls.load(Ordering::SeqCst);
    assert!((1..=30).contains(&n), "told {n} times in 1.5 s");
}

#[tokio::test(flavor = "multi_thread")]
async fn dropping_the_client_while_notices_are_queued_still_tells_each_listener_once() {
    let mut served = Served::in_memory().await;
    let texts = RpcTexts::connect(&served.socket).unwrap();
    served.stop();
    tokio::time::sleep(Duration::from_millis(300)).await;
    let first = Arc::new(Seen::default());
    let (texts, _first_sub) = tokio::task::spawn_blocking({
        let first = first.clone();
        move || {
            let sub = texts.subscribe(first);
            (texts, sub)
        }
    })
    .await
    .unwrap();
    let many: Vec<Arc<Seen>> = (0..20).map(|_| Arc::new(Seen::default())).collect();
    let subs: Vec<_> = many.iter().map(|l| texts.subscribe(l.clone())).collect();
    drop(texts);
    let started = std::time::Instant::now();
    while many.iter().any(|l| l.ended.lock().unwrap().is_empty()) {
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "not all were told"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(many.iter().all(|l| l.ended.lock().unwrap().len() == 1));
    drop(subs);
}
