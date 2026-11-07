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
        let again = self.texts.get().unwrap().subscribe(me);
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
    // A new listener on the dead connection hears it once, at once.
    let newcomer = Arc::new(Seen::default());
    let _n = texts.subscribe(newcomer.clone());
    assert_eq!(newcomer.ended.lock().unwrap().len(), 1);
}
