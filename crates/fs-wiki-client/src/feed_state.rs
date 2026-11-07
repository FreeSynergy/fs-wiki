//! Which remote subscription of a connection runs, and whether the connection can carry one at all.
//!
//! Pattern: State — a generation number for the running subscription (0 = none) and a flag for a
//! connection that cannot carry a feed; plus the list of listeners that were told so.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};

use fs_wiki_core::TextListener;

/// What a listener is told when the connection cannot carry a feed.
pub(crate) const DEAD: &str =
    "the connection to the wiki server is gone, so no feed can be made; connect again";

/// The pause between two such notices: a listener that always subscribes a fresh object in `ended`
/// is told at this pace and not in a busy loop.
pub(crate) const NOTICE_PAUSE: std::time::Duration = std::time::Duration::from_millis(100);

/// Which remote subscription of a connection is the running one (0 = none).
#[derive(Default)]
pub(crate) struct FeedState {
    next: AtomicU64,
    current: AtomicU64,
    /// Set when a remote subscription could not be made: this connection cannot carry a feed.
    dead: AtomicBool,
    /// The listeners that were told so, once each (see `subscribe`).
    told: Mutex<Vec<Weak<dyn TextListener>>>,
    /// The listeners waiting to hear it, and whether a thread is delivering them.
    notices: Mutex<Notices>,
}

#[derive(Default)]
struct Notices {
    waiting: VecDeque<Arc<dyn TextListener>>,
    delivering: bool,
}

impl FeedState {
    /// Whether a remote subscription failed on this connection.
    pub(crate) fn is_dead(&self) -> bool {
        self.dead.load(Ordering::SeqCst)
    }

    /// Take the right to start a remote subscription, if none is running.
    pub(crate) fn claim(&self) -> Option<u64> {
        let generation = self.next.fetch_add(1, Ordering::SeqCst) + 1;
        self.current
            .compare_exchange(0, generation, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| generation)
    }

    /// Remember that no feed can be made on this connection.
    pub(crate) fn bury(&self) {
        self.dead.store(true, Ordering::SeqCst);
    }

    /// Whether `listener` is the first to hear that the connection cannot carry a feed (and
    /// remember it). Listeners that are gone are forgotten on the way.
    pub(crate) fn first_notice(&self, listener: &Arc<dyn TextListener>) -> bool {
        let mut told = self.told.lock().unwrap_or_else(PoisonError::into_inner);
        told.retain(|w| w.strong_count() > 0);
        let me = Arc::downgrade(listener);
        if told.iter().any(|w| w.ptr_eq(&me)) {
            return false;
        }
        told.push(me);
        true
    }

    /// Put `listener` in the queue of those who are to hear that the feed is dead. Returns true if the
    /// caller must start the thread that delivers (none is running); that thread calls
    /// [`next_notice`](Self::next_notice) until it answers `None`.
    pub(crate) fn queue_notice(&self, listener: Arc<dyn TextListener>) -> bool {
        let mut n = self.notices.lock().unwrap_or_else(PoisonError::into_inner);
        n.waiting.push_back(listener);
        !std::mem::replace(&mut n.delivering, true)
    }

    /// The next listener to tell; `None` when the queue is empty (the delivering thread then ends).
    pub(crate) fn next_notice(&self) -> Option<Arc<dyn TextListener>> {
        let mut n = self.notices.lock().unwrap_or_else(PoisonError::into_inner);
        let next = n.waiting.pop_front();
        if next.is_none() {
            n.delivering = false;
        }
        next
    }

    /// Give up `generation`; false if it is not the running one any more.
    pub(crate) fn release(&self, generation: u64) -> bool {
        self.current
            .compare_exchange(generation, 0, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
    }
}
