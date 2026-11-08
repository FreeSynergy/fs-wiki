//! `Texts` root interface: every method of `texts.capnp`, each one call of the provider.
//!
//! Pattern: Adapter + Facade — wire request in, `Texts` call, wire response out; what a provider
//! cannot do arrives as its explicit `Unsupported` and leaves as a typed problem. [`TextsServer`] is
//! what a program serves; it depends on the two core traits, never on `WikiRepo` (ADR 0043). Every
//! write goes through `run_to_end`, so a client that goes away cannot leave a move half done.

use std::rc::Rc;
use std::sync::Arc;

use capnp_rpc::new_client;
use fs_interfaces::rpc::{write_uuid, DEFAULT_PAGE_SIZE};
use fs_interfaces::texts_capnp::texts;
use fs_wiki_core::{TextFeed, Texts};

use crate::feed::subscribe;
use crate::{reads, writes};

/// The contract version a report names.
pub const CONTRACT_VERSION: &str = "texts 0.1 (draft)";

/// The root capability of the wiki program.
pub struct TextsServer {
    texts: Arc<dyn Texts>,
    feed: Arc<dyn TextFeed>,
    pub(crate) page_size: usize,
}

impl TextsServer {
    /// Serve this provider.
    pub fn new(texts: Arc<dyn Texts>, feed: Arc<dyn TextFeed>) -> Self {
        Self {
            texts,
            feed,
            page_size: DEFAULT_PAGE_SIZE,
        }
    }

    /// Texts per page (default [`DEFAULT_PAGE_SIZE`]); a page also stops where an answer would not fit
    /// one message.
    #[must_use]
    pub fn with_page_size(mut self, size: usize) -> Self {
        self.page_size = size.max(1);
        self
    }

    /// The capability to hand to `fs_interfaces::rpc::serve`.
    #[must_use]
    pub fn into_client(self) -> texts::Client {
        new_client(self)
    }
}

type Done = Result<(), capnp::Error>;

impl texts::Server for TextsServer {
    async fn support_report(
        self: Rc<Self>,
        _: texts::SupportReportParams,
        mut results: texts::SupportReportResults,
    ) -> Done {
        let report = self.texts.capabilities();
        let mut out = results.get().init_report();
        report.write(CONTRACT_VERSION, out.reborrow());
        // The store's id, as the store says it: empty where the provider has no store.
        write_uuid(
            out.init_instance(),
            self.texts.identity().map(|id| id.as_uuid()),
        );
        Ok(())
    }

    async fn get(self: Rc<Self>, params: texts::GetParams, results: texts::GetResults) -> Done {
        reads::get(&self.texts, params, results).await
    }

    async fn get_by_path(
        self: Rc<Self>,
        params: texts::GetByPathParams,
        results: texts::GetByPathResults,
    ) -> Done {
        reads::get_by_path(&self.texts, params, results).await
    }

    async fn list(self: Rc<Self>, params: texts::ListParams, results: texts::ListResults) -> Done {
        reads::list(&self.texts, self.page_size, params, results).await
    }

    async fn find(self: Rc<Self>, params: texts::FindParams, results: texts::FindResults) -> Done {
        reads::find(&self.texts, self.page_size, params, results).await
    }

    async fn links(
        self: Rc<Self>,
        params: texts::LinksParams,
        results: texts::LinksResults,
    ) -> Done {
        reads::links(&self.texts, params, results).await
    }

    async fn backlinks(
        self: Rc<Self>,
        params: texts::BacklinksParams,
        results: texts::BacklinksResults,
    ) -> Done {
        reads::backlinks(&self.texts, params, results).await
    }

    async fn export_text(
        self: Rc<Self>,
        params: texts::ExportTextParams,
        results: texts::ExportTextResults,
    ) -> Done {
        reads::export_text(&self.texts, params, results).await
    }

    async fn create(
        self: Rc<Self>,
        params: texts::CreateParams,
        results: texts::CreateResults,
    ) -> Done {
        writes::create(&self.texts, params, results).await
    }

    async fn edit(self: Rc<Self>, params: texts::EditParams, results: texts::EditResults) -> Done {
        writes::edit(&self.texts, params, results).await
    }

    async fn edit_if(
        self: Rc<Self>,
        params: texts::EditIfParams,
        results: texts::EditIfResults,
    ) -> Done {
        writes::edit_if(&self.texts, params, results).await
    }

    async fn move_(self: Rc<Self>, params: texts::MoveParams, results: texts::MoveResults) -> Done {
        writes::move_(&self.texts, params, results).await
    }

    async fn move_folder(
        self: Rc<Self>,
        params: texts::MoveFolderParams,
        results: texts::MoveFolderResults,
    ) -> Done {
        writes::move_folder(&self.texts, params, results).await
    }

    async fn delete(
        self: Rc<Self>,
        params: texts::DeleteParams,
        results: texts::DeleteResults,
    ) -> Done {
        writes::delete(&self.texts, params, results).await
    }

    async fn import_text(
        self: Rc<Self>,
        params: texts::ImportTextParams,
        results: texts::ImportTextResults,
    ) -> Done {
        writes::import_text(&self.texts, params, results).await
    }

    async fn subscribe(
        self: Rc<Self>,
        params: texts::SubscribeParams,
        mut results: texts::SubscribeResults,
    ) -> Done {
        let listener = params.get()?.get_listener()?;
        let subscription = subscribe(&self.feed, self.texts.clone(), listener)?;
        results.get().set_subscription(subscription);
        Ok(())
    }
}
