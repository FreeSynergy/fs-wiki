//! Changes of the provider to a subscribed consumer.
//!
//! Pattern: Observer + Adapter — the core calls its listener on the thread that made the change; this
//! one only puts the change into a channel. The shared forwarder adds the envelope and sends it.

use std::sync::Arc;

use fs_interfaces::common_capnp::{change_listener, subscription};
use fs_interfaces::rpc::{change_channel, forward, unsupported, ChangeSender, Notice};
use fs_wiki_core::{TextChange, TextFeed, TextListener, Texts};

struct ToChannel(ChangeSender<TextChange>);

impl TextListener for ToChannel {
    fn changed(&self, change: &TextChange) {
        self.0.send(*change);
    }
}

/// Subscribe `listener` until the returned capability is dropped.
pub(crate) fn subscribe(
    feed: &Arc<dyn TextFeed>,
    texts: Arc<dyn Texts>,
    listener: change_listener::Client,
) -> capnp::Result<subscription::Client> {
    let (tx, rx) = change_channel();
    let core_subscription = feed
        .subscribe(Arc::new(ToChannel(tx)))
        .map_err(|e| unsupported(&e))?;
    Ok(forward(rx, core_subscription, listener, move |change| {
        let texts = texts.clone();
        async move { notice(texts.as_ref(), change).await }
    }))
}

/// A text that is gone again by now has nothing to report but its own `Deleted`, which follows. A
/// failed read is something else: the consumer would miss a change, so it ends the feed.
async fn notice(texts: &dyn Texts, change: TextChange) -> Result<Option<Notice>, String> {
    let envelope = async |id| {
        let entry = texts.get(id).await.map_err(|e| e.to_string())?;
        Ok::<_, String>(entry.map(|e| e.envelope))
    };
    Ok(match change {
        TextChange::Created(id) => envelope(id).await?.map(Notice::Created),
        TextChange::Updated(id) => envelope(id).await?.map(Notice::Updated),
        TextChange::Deleted(id) => Some(Notice::Deleted(id)),
    })
}
