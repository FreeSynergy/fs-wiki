//! `TextFeed` implemented by `WikiRepo`.
//!
//! Pattern: Observer (subject side) — the repository owns a [`Registry`] and tells it after each
//! successful store write (`write.rs`, `moving.rs`); this file only exposes the subscription.

use std::sync::Arc;

use super::handle::WikiRepo;
use crate::feed::{Subscription, TextFeed, TextListener};

impl TextFeed for WikiRepo {
    fn subscribe(&self, listener: Arc<dyn TextListener>) -> Subscription {
        self.feed.add(listener)
    }
}
