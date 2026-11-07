//! The provider rule over the wire: the report, and a provider that does not offer a method.
//!
//! Pattern: Example-based integration tests; the read-only provider is a second, real implementation
//! of `Texts` beside `WikiRepo`, only for what the store-backed one cannot show.

mod common;

use std::sync::Arc;

use async_trait::async_trait;
use common::{problem, set_id, with_provider, with_server, ME};
use fs_interfaces::rpc::Problem;
use fs_interfaces::texts_capnp::texts;
use fs_store::{ObjectId, Store, TursoStore};
use fs_text::Document;
use fs_wiki_core::{Capabilities, Paging, Result, TextEntry, Texts, WikiRepo};
use fs_wiki_server::CONTRACT_VERSION;

const ALL: [&str; 10] = [
    "text.create",
    "text.edit",
    "text.move",
    "text.delete",
    "text.read",
    "text.find",
    "link.read",
    "text.import",
    "text.export",
    "text.subscribe",
];

async fn report(root: &texts::Client) -> (String, String, Vec<(String, bool, String)>) {
    let answer = root.support_report_request().send().promise.await.unwrap();
    let r = answer.get().unwrap().get_report().unwrap();
    let entries = r
        .get_entries()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e.get_capability().unwrap().to_string().unwrap(),
                e.get_supported(),
                e.get_reason().unwrap().to_string().unwrap(),
            )
        })
        .collect();
    (
        r.get_contract_version().unwrap().to_string().unwrap(),
        r.get_provider().unwrap().to_string().unwrap(),
        entries,
    )
}

#[tokio::test(flavor = "current_thread")]
async fn the_wiki_reports_every_capability_of_the_contract_as_offered() {
    with_server(|root| async move {
        let (version, provider, entries) = report(&root).await;
        assert_eq!(
            (version.as_str(), provider.as_str()),
            (CONTRACT_VERSION, "fs-wiki")
        );
        let got: Vec<_> = entries.iter().map(|e| (e.0.as_str(), e.1)).collect();
        let want: Vec<_> = ALL.iter().map(|id| (*id, true)).collect();
        assert_eq!(got, want);
    })
    .await;
}

/// A read-only provider, for example a help library shipped with a program.
struct ReadOnly(WikiRepo);

#[async_trait]
impl Texts for ReadOnly {
    fn capabilities(&self) -> Capabilities {
        let mut c = Capabilities::new("read-only");
        c.supported("text.read");
        c.unsupported("text.create", "this library is shipped, not written");
        c
    }
    async fn get(&self, id: ObjectId) -> Result<Option<TextEntry>> {
        self.0.get(id).await
    }
    async fn get_by_path(&self, path: &str) -> Result<Option<TextEntry>> {
        self.0.get_by_path(path).await
    }
    async fn list(&self, prefix: &str, page: Paging) -> Result<Vec<TextEntry>> {
        self.0.list(prefix, page).await
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_provider_without_a_method_answers_not_possible_with_its_capability() {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let behind = WikiRepo::new(store.clone(), ME);
    let id = behind.create("lib/a", "shipped\n").await.unwrap().id;
    let read_only = Arc::new(ReadOnly(WikiRepo::new(store.clone(), ME)));
    let feed = Arc::new(WikiRepo::new(store, ME));
    with_provider(read_only, feed, |root| async move {
        let (_, provider, entries) = report(&root).await;
        assert_eq!(provider, "read-only");
        assert!(entries.contains(&(
            "text.create".into(),
            false,
            "this library is shipped, not written".into()
        )));

        let not_possible = |error: capnp::Error, capability: &str| {
            assert!(Problem::from_error(&error).unwrap().is_unsupported());
            let (_, status, detail) = problem(&error);
            assert_eq!(status, 501);
            assert!(detail.starts_with(capability), "{detail}");
        };
        let mut req = root.create_request();
        req.get().set_path("lib/b");
        req.get().set_markdown("x");
        not_possible(req.send().promise.await.err().unwrap(), "text.create");
        let mut req = root.delete_request();
        set_id(req.get().init_id(), id.as_uuid());
        not_possible(req.send().promise.await.err().unwrap(), "text.delete");
        let mut req = root.edit_request();
        set_id(req.get().init_id(), id.as_uuid());
        req.get()
            .set_document(serde_json::to_string(&Document::new()).unwrap().as_str());
        not_possible(req.send().promise.await.err().unwrap(), "text.edit");
        let mut req = root.links_request();
        set_id(req.get().init_id(), id.as_uuid());
        not_possible(req.send().promise.await.err().unwrap(), "link.read");
        let mut req = root.move_folder_request();
        req.get().set_prefix("lib");
        req.get().set_path("lib2");
        not_possible(req.send().promise.await.err().unwrap(), "text.move");
        assert_eq!(
            common::get(&root, id.as_uuid()).await.unwrap().path,
            "lib/a",
            "reading works"
        );
    })
    .await;
}
