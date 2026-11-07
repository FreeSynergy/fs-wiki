//! `Texts` root interface: every method of `texts.capnp`, each one call of the provider.
//!
//! Pattern: Adapter + Facade — wire request in, `Texts` call, wire response out; what a provider
//! cannot do arrives as its explicit `Unsupported` and leaves as a typed problem. [`TextsServer`] is
//! what a program serves; it depends on the two core traits, never on `WikiRepo` (ADR 0043). Every
//! write goes through `run_to_end`, so a client that goes away cannot leave a move half done.

use std::rc::Rc;
use std::sync::Arc;

use capnp_rpc::new_client;
use fs_interfaces::rpc::{check_answer, read_object_id, run_to_end};
use fs_interfaces::texts_capnp::texts;
use fs_wiki_core::{Paging, TextEntry, TextFeed, TextFormat, Texts};

use crate::errors::{rpc_error, rpc_error_for_format};
use crate::feed::subscribe;
use crate::links::{internal, write_links};
use crate::wire::{
    count, fits, read_document, required, source_arg, text_arg, write_all, write_entry, write_page,
    Sent,
};

/// The contract version a report names.
pub const CONTRACT_VERSION: &str = "texts 0.1 (draft)";

/// The root capability of the wiki program.
pub struct TextsServer {
    texts: Arc<dyn Texts>,
    feed: Arc<dyn TextFeed>,
}

impl TextsServer {
    /// Serve this provider.
    pub fn new(texts: Arc<dyn Texts>, feed: Arc<dyn TextFeed>) -> Self {
        Self { texts, feed }
    }

    /// The capability to hand to `fs_interfaces::rpc::serve`.
    #[must_use]
    pub fn into_client(self) -> texts::Client {
        new_client(self)
    }
}

type Done = Result<(), capnp::Error>;

/// One text as an answer.
fn answer(out: fs_interfaces::texts_capnp::text_entry::Builder<'_>, entry: TextEntry) -> Done {
    write_entry(out, &Sent::new(entry)?)
}

impl texts::Server for TextsServer {
    async fn support_report(
        self: Rc<Self>,
        _: texts::SupportReportParams,
        mut results: texts::SupportReportResults,
    ) -> Done {
        let report = self.texts.capabilities();
        report.write(CONTRACT_VERSION, results.get().init_report());
        Ok(())
    }

    async fn create(
        self: Rc<Self>,
        params: texts::CreateParams,
        mut results: texts::CreateResults,
    ) -> Done {
        let p = params.get()?;
        let (path, markdown) = (
            required(p.get_path()?, "path")?,
            source_arg(p.get_markdown()?)?,
        );
        fits(&TextFormat::Markdown.import(markdown))?;
        let (texts, path, markdown) = (self.texts.clone(), path.to_string(), markdown.to_string());
        let entry = run_to_end(async move { texts.create(&path, &markdown).await })
            .await?
            .map_err(rpc_error)?;
        answer(results.get().init_text(), entry)
    }

    async fn edit(
        self: Rc<Self>,
        params: texts::EditParams,
        mut results: texts::EditResults,
    ) -> Done {
        let p = params.get()?;
        let id = read_object_id(p.get_id()?)?;
        let document = read_document(p.get_document()?)?;
        let texts = self.texts.clone();
        let entry = run_to_end(async move { texts.edit(id, document).await })
            .await?
            .map_err(rpc_error)?;
        answer(results.get().init_text(), entry)
    }

    async fn edit_if(
        self: Rc<Self>,
        params: texts::EditIfParams,
        mut results: texts::EditIfResults,
    ) -> Done {
        let p = params.get()?;
        let id = read_object_id(p.get_id()?)?;
        let revision = p.get_revision();
        let document = read_document(p.get_document()?)?;
        let texts = self.texts.clone();
        let entry = run_to_end(async move { texts.edit_if(id, revision, document).await })
            .await?
            .map_err(rpc_error)?;
        answer(results.get().init_text(), entry)
    }

    async fn move_(
        self: Rc<Self>,
        params: texts::MoveParams,
        mut results: texts::MoveResults,
    ) -> Done {
        let p = params.get()?;
        let id = read_object_id(p.get_id()?)?;
        let path = required(p.get_path()?, "path")?.to_string();
        let texts = self.texts.clone();
        let entry = run_to_end(async move { texts.move_to(id, &path).await })
            .await?
            .map_err(rpc_error)?;
        answer(results.get().init_text(), entry)
    }

    async fn move_folder(
        self: Rc<Self>,
        params: texts::MoveFolderParams,
        mut results: texts::MoveFolderResults,
    ) -> Done {
        let p = params.get()?;
        let prefix = required(p.get_prefix()?, "prefix")?.to_string();
        let path = required(p.get_path()?, "path")?.to_string();
        let texts = self.texts.clone();
        let moved = run_to_end(async move { texts.move_folder(&prefix, &path).await })
            .await?
            .map_err(rpc_error)?;
        let list = results.get().init_moved(count(moved.len())?);
        write_all(list, moved)
    }

    async fn delete(self: Rc<Self>, params: texts::DeleteParams, _: texts::DeleteResults) -> Done {
        let id = read_object_id(params.get()?.get_id()?)?;
        let texts = self.texts.clone();
        run_to_end(async move { texts.delete(id).await })
            .await?
            .map_err(rpc_error)
    }

    async fn get(self: Rc<Self>, params: texts::GetParams, mut results: texts::GetResults) -> Done {
        let id = read_object_id(params.get()?.get_id()?)?;
        let found = self.texts.get(id).await.map_err(rpc_error)?;
        let mut r = results.get();
        r.set_found(found.is_some());
        found.map_or(Ok(()), |entry| answer(r.init_text(), entry))
    }

    async fn get_by_path(
        self: Rc<Self>,
        params: texts::GetByPathParams,
        mut results: texts::GetByPathResults,
    ) -> Done {
        let path = required(params.get()?.get_path()?, "path")?.to_string();
        let found = self.texts.get_by_path(&path).await.map_err(rpc_error)?;
        let mut r = results.get();
        r.set_found(found.is_some());
        found.map_or(Ok(()), |entry| answer(r.init_text(), entry))
    }

    async fn list(
        self: Rc<Self>,
        params: texts::ListParams,
        mut results: texts::ListResults,
    ) -> Done {
        let p = params.get()?;
        let prefix = text_arg(p.get_prefix()?)?.to_string();
        let all = self
            .texts
            .list(&prefix, Paging::default())
            .await
            .map_err(rpc_error)?;
        write_page(results.get().init_page(), all, p.get_cursor())
    }

    async fn find(
        self: Rc<Self>,
        params: texts::FindParams,
        mut results: texts::FindResults,
    ) -> Done {
        let p = params.get()?;
        let term = required(p.get_term()?, "term")?.to_string();
        let all = self
            .texts
            .find(&term, Paging::default())
            .await
            .map_err(rpc_error)?;
        write_page(results.get().init_page(), all, p.get_cursor())
    }

    async fn links(
        self: Rc<Self>,
        params: texts::LinksParams,
        mut results: texts::LinksResults,
    ) -> Done {
        let id = read_object_id(params.get()?.get_id()?)?;
        let links = self.texts.links(id).await.map_err(rpc_error)?;
        let rows = internal(&links)?;
        write_links(results.get().init_items(count(rows.len())?), &rows);
        Ok(())
    }

    async fn backlinks(
        self: Rc<Self>,
        params: texts::BacklinksParams,
        mut results: texts::BacklinksResults,
    ) -> Done {
        let path = required(params.get()?.get_path()?, "path")?.to_string();
        let links = self.texts.backlinks(&path).await.map_err(rpc_error)?;
        let rows = internal(&links)?;
        write_links(results.get().init_items(count(rows.len())?), &rows);
        Ok(())
    }

    async fn import_text(
        self: Rc<Self>,
        params: texts::ImportTextParams,
        mut results: texts::ImportTextResults,
    ) -> Done {
        let p = params.get()?;
        let path = required(p.get_path()?, "path")?.to_string();
        let media = text_arg(p.get_media_type()?)?.to_string();
        let text = source_arg(p.get_text()?)?;
        if let Ok(format) = TextFormat::from_media_type(&media) {
            fits(&format.import(text))?;
        }
        let (texts, text) = (self.texts.clone(), text.to_string());
        let entry = run_to_end(async move { texts.import_text(&path, &media, &text).await })
            .await?
            .map_err(|e| rpc_error_for_format("text.import", e))?;
        answer(results.get().init_entry(), entry)
    }

    async fn export_text(
        self: Rc<Self>,
        params: texts::ExportTextParams,
        mut results: texts::ExportTextResults,
    ) -> Done {
        let p = params.get()?;
        let id = read_object_id(p.get_id()?)?;
        let media = text_arg(p.get_media_type()?)?.to_string();
        let text = self
            .texts
            .export_text(id, &media)
            .await
            .map_err(|e| rpc_error_for_format("text.export", e))?;
        check_answer(text.len())?;
        results.get().set_text(text.as_str());
        Ok(())
    }

    async fn subscribe(
        self: Rc<Self>,
        params: texts::SubscribeParams,
        mut results: texts::SubscribeResults,
    ) -> Done {
        let listener = params.get()?.get_listener()?;
        let subscription = subscribe(&self.feed, self.texts.clone(), listener);
        results.get().set_subscription(subscription);
        Ok(())
    }
}
