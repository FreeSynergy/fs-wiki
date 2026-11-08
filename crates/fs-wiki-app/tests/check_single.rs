//! One text that cannot be asked about does not stop the start; a delete whose record cannot be
//! updated says so.
//!
//! Pattern: Integration tests against the real server. The stand-ins are small: a provider in front
//! of the real wiki whose `get` fails for one id, and a records store that cannot write.

mod common;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use common::{temp_dir, write_files};
use fs_objects::ObjectId;
use fs_wiki_app::{FileOrigins, Wiki, import_folder};
use fs_wiki_client::RpcTexts;
use fs_wiki_core::{
    Capabilities, Error, FileOrigin, Origins, Paging, Place, Result, TextEntry, TextPage,
    TextQuery, Texts, WikiRepo,
};
use fs_wiki_testserver::Served;

/// The real wiki; `get` fails for one id.
struct OneBad(Arc<WikiRepo>, Mutex<Option<ObjectId>>);

#[async_trait]
impl Texts for OneBad {
    fn capabilities(&self) -> Capabilities {
        self.0.capabilities()
    }
    async fn get(&self, id: ObjectId) -> Result<Option<TextEntry>> {
        if *self.1.lock().unwrap() == Some(id) {
            return Err(Error::Store(fs_objects::Error::Backend(
                "disk hiccup".into(),
            )));
        }
        self.0.get(id).await
    }
    async fn page(
        &self,
        query: &TextQuery,
        after: Option<&Place>,
        size: usize,
    ) -> Result<TextPage> {
        self.0.page(query, after, size).await
    }
    async fn get_by_path(&self, path: &str) -> Result<Option<TextEntry>> {
        self.0.get_by_path(path).await
    }
    async fn list(&self, prefix: &str, page: Paging) -> Result<Vec<TextEntry>> {
        self.0.list(prefix, page).await
    }
    async fn create(&self, path: &str, markdown: &str) -> Result<TextEntry> {
        self.0.create(path, markdown).await
    }
    async fn import_text(&self, path: &str, media: &str, text: &str) -> Result<TextEntry> {
        self.0.import_text(path, media, text).await
    }
}

#[test]
fn one_failing_read_is_reported_for_its_file_and_does_not_stop_the_import() {
    let dir = temp_dir("cs-onebad");
    let docs = dir.join("docs");
    write_files(&docs, &[("a.md", "# A\n"), ("b.md", "# B\n")]);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let plain = rt.block_on(Served::in_memory());
    let bad = Arc::new(OneBad(plain.repo.clone(), Mutex::new(None)));
    let served = Served::serving(bad.clone(), plain.repo.clone());
    let origins = FileOrigins::open(&dir.join("o.origins")).unwrap();
    let texts = RpcTexts::connect(&served.socket).unwrap();
    let wiki = Wiki::new(Arc::new(texts), Arc::new(origins));
    assert_eq!(rt.block_on(import_folder(&wiki, &docs)).imported, 2);
    let a = rt.block_on(wiki.get_by_path("a.md")).unwrap().unwrap();
    *bad.1.lock().unwrap() = Some(a.id);
    write_files(&docs, &[("c.md", "# C\n")]);
    let report = rt.block_on(import_folder(&wiki, &docs));
    assert_eq!(
        (report.imported, report.unchanged),
        (1, 2),
        "c.md came in, a.md is taken as there"
    );
    let reported: Vec<_> = report.failures.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        reported,
        ["a.md"],
        "and the file whose text could not be asked about is named"
    );
    assert_eq!(report.records_ignored, 0);
}

#[test]
fn the_only_record_whose_read_fails_does_not_stop_the_import_either() {
    let dir = temp_dir("cs-onlybad");
    let docs = dir.join("docs");
    write_files(&docs, &[("a.md", "# A\n")]);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let plain = rt.block_on(Served::in_memory());
    let bad = Arc::new(OneBad(plain.repo.clone(), Mutex::new(None)));
    let served = Served::serving(bad.clone(), plain.repo.clone());
    let origins = FileOrigins::open(&dir.join("o.origins")).unwrap();
    let texts = RpcTexts::connect(&served.socket).unwrap();
    let wiki = Wiki::new(Arc::new(texts), Arc::new(origins));
    assert_eq!(rt.block_on(import_folder(&wiki, &docs)).imported, 1);
    let a = rt.block_on(wiki.get_by_path("a.md")).unwrap().unwrap();
    *bad.1.lock().unwrap() = Some(a.id);
    let report = rt.block_on(import_folder(&wiki, &docs));
    assert_eq!(
        (report.imported, report.unchanged),
        (0, 1),
        "the one file is taken as there, as with two"
    );
    let reported: Vec<_> = report.failures.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        reported,
        ["a.md"],
        "named by its file, not as a failed start"
    );
}

/// Records that name every text, and cannot be written.
struct Unwritable(Mutex<Vec<FileOrigin>>);

#[async_trait]
impl Origins for Unwritable {
    async fn origins(&self) -> Result<Vec<FileOrigin>> {
        Ok(self.0.lock().unwrap().clone())
    }
    async fn set_origin(&self, _: &FileOrigin) -> Result<()> {
        Err(Error::Store(fs_objects::Error::Backend("disk full".into())))
    }
}

#[test]
fn a_text_deleted_whose_record_cannot_be_updated_is_reported_as_deleted() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let served = rt.block_on(Served::in_memory());
    let texts = RpcTexts::connect(&served.socket).unwrap();
    let made = rt.block_on(texts.create("a.md", "# A\n")).unwrap();
    let record = FileOrigin {
        file: "a.md".into(),
        hash: "h".into(),
        text: made.id,
        text_hash: "t".into(),
        deleted: false,
    };
    let wiki = Wiki::new(
        Arc::new(texts),
        Arc::new(Unwritable(Mutex::new(vec![record]))),
    );
    let e = rt.block_on(wiki.delete(made.id)).unwrap_err();
    assert!(
        rt.block_on(wiki.get(made.id)).unwrap().is_none(),
        "the text is gone, whatever the error says"
    );
    let said = e.to_string();
    let words_start =
        fs_wiki_app::words::say(fs_wiki_app::words::DELETED_RECORD_STALE, &[("error", &"x")]);
    let words_start = words_start.split('(').next().unwrap();
    assert!(said.contains(words_start), "{said}");
}

/// The real wiki, with no identity (a provider that has no store).
struct NoIdentity(Arc<WikiRepo>);

#[async_trait]
impl Texts for NoIdentity {
    fn capabilities(&self) -> Capabilities {
        self.0.capabilities()
    }
    async fn page(
        &self,
        query: &TextQuery,
        after: Option<&Place>,
        size: usize,
    ) -> Result<TextPage> {
        self.0.page(query, after, size).await
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
    async fn import_text(&self, path: &str, media: &str, text: &str) -> Result<TextEntry> {
        self.0.import_text(path, media, text).await
    }
}

#[test]
fn records_made_for_a_store_are_not_trusted_by_a_server_that_tells_no_identity() {
    let dir = temp_dir("cs-noid");
    let (docs, records) = (dir.join("docs"), dir.join("o.origins"));
    write_files(&docs, &[("a.md", "# A\\n")]);
    let rt = tokio::runtime::Runtime::new().unwrap();
    {
        let served = rt.block_on(Served::in_memory());
        let origins = FileOrigins::open(&records).unwrap();
        let texts = RpcTexts::connect(&served.socket).unwrap();
        let wiki = Wiki::new(Arc::new(texts), Arc::new(origins));
        assert_eq!(rt.block_on(import_folder(&wiki, &docs)).imported, 1);
    }
    let plain = rt.block_on(Served::in_memory());
    assert!(plain.repo.identity().is_some());
    let wiki = Wiki::new(
        Arc::new(NoIdentity(plain.repo.clone())),
        Arc::new(FileOrigins::open(&records).unwrap()),
    );
    let report = rt.block_on(import_folder(&wiki, &docs));
    assert_eq!(
        (report.imported, report.records_ignored),
        (1, 1),
        "the records were made for a store this provider cannot name"
    );
}
