//! A feed that cannot go on says so (`ended`); a provider without a feed says "not possible".
//!
//! Pattern: Example-based integration tests with two small providers.

mod common;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use capnp_rpc::new_client;
use common::seen::{watch, Seen};
use common::with_provider;
use fs_interfaces::common_capnp::change_listener;
use fs_interfaces::rpc::Problem;
use fs_store::{ObjectId, Store, TursoStore};
use fs_wiki_core::{
    Capabilities, Paging, Place, Subscription, TextEntry, TextFeed, TextListener, TextPage,
    TextQuery, Texts, Unsupported, WikiRepo,
};

/// A provider whose reads fail on demand, to end a feed: the change cannot be told.
struct Flaky {
    inner: Arc<WikiRepo>,
    broken: AtomicBool,
}

#[async_trait]
impl Texts for Flaky {
    fn capabilities(&self) -> Capabilities {
        self.inner.capabilities()
    }
    async fn get(&self, id: ObjectId) -> fs_wiki_core::Result<Option<TextEntry>> {
        if self.broken.load(Ordering::SeqCst) {
            return Err(fs_wiki_core::Error::Stored(
                "the store does not answer".into(),
            ));
        }
        self.inner.get(id).await
    }
    async fn get_by_path(&self, path: &str) -> fs_wiki_core::Result<Option<TextEntry>> {
        self.inner.get_by_path(path).await
    }
    async fn list(&self, prefix: &str, page: Paging) -> fs_wiki_core::Result<Vec<TextEntry>> {
        self.inner.list(prefix, page).await
    }
    async fn page(
        &self,
        q: &TextQuery,
        after: Option<&Place>,
        size: usize,
    ) -> fs_wiki_core::Result<TextPage> {
        self.inner.page(q, after, size).await
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_change_that_cannot_be_told_ends_the_feed_with_ended() {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let repo = Arc::new(WikiRepo::new(store, common::ME));
    let flaky = Arc::new(Flaky {
        inner: repo.clone(),
        broken: AtomicBool::new(false),
    });
    let writer = repo.clone();
    with_provider(flaky.clone(), repo, |root| async move {
        let seen = Seen::default();
        let _answer = watch(&root, &seen).await;
        writer.create("a", "# One\n").await.unwrap();
        seen.wait_for(1).await;
        flaky.broken.store(true, Ordering::SeqCst);
        writer.create("b", "# Two\n").await.unwrap();
        seen.wait_for(2).await;
        assert_eq!(seen.0.borrow()[0], "created One");
        assert!(
            seen.0.borrow()[1].starts_with("ended "),
            "{:?}",
            seen.0.borrow()
        );
    })
    .await;
}

/// A provider whose report does not carry `text.subscribe`.
struct NoFeed;

impl TextFeed for NoFeed {
    fn subscribe(&self, _: Arc<dyn TextListener>) -> Result<Subscription, Unsupported> {
        Err(Unsupported::new(
            "text.subscribe",
            "this library never changes",
        ))
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_provider_without_a_feed_answers_not_possible_for_subscribe() {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let repo = Arc::new(WikiRepo::new(store, common::ME));
    with_provider(repo, Arc::new(NoFeed), |root| async move {
        let listener: change_listener::Client = new_client(Seen::default());
        let mut sub = root.subscribe_request();
        sub.get().set_listener(listener);
        let error = sub.send().promise.await.err().expect("not possible");
        let p = Problem::from_error(&error).unwrap();
        assert!(p.is_unsupported());
        assert!(p.detail.starts_with("text.subscribe"), "{}", p.detail);
    })
    .await;
}
