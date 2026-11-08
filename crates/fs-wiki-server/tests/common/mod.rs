//! Shared helpers of the integration tests: a real server on a real Unix socket over a real store.

#![allow(dead_code)] // each test file uses a different subset

pub mod counting;
pub mod ports;
pub mod seen;
pub mod v2;

use std::future::Future;
use std::sync::Arc;

use fs_interfaces::rpc::{bind_socket, connect, read_envelope, serve, write_uuid, Problem};
use fs_interfaces::texts_capnp::{text_entry, texts};
use fs_objects::{Store, TursoStore};
use fs_wiki_core::{TextFeed, Texts, WikiRepo};
use fs_wiki_server::TextsServer;
use uuid::Uuid;

pub const ME: &str = "https://local.invalid/users/owner";
pub const PROBLEMS: &str = "https://freesynergy.net/problems/";

/// A text as a consumer reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct Got {
    pub id: Uuid,
    pub name: String,
    pub context: Option<String>,
    pub iri: Option<String>,
    pub path: String,
    pub document: String,
    pub revision: u64,
    pub template: bool,
    pub redirect_to: String,
    pub updated_by: String,
}

pub fn read_text(t: text_entry::Reader<'_>) -> Got {
    let env = read_envelope(t.get_envelope().unwrap()).unwrap();
    Got {
        id: env.id.as_uuid(),
        name: env.name,
        context: env.context,
        iri: env.iri,
        path: t.get_path().unwrap().to_string().unwrap(),
        document: t.get_document().unwrap().to_string().unwrap(),
        revision: t.get_revision(),
        template: t.get_template(),
        redirect_to: t.get_redirect_to().unwrap().to_string().unwrap(),
        updated_by: t.get_updated_by().unwrap().to_string().unwrap(),
    }
}

/// A real server in front of a fresh in-memory wiki.
pub async fn with_server<F, Fut>(test: F)
where
    F: FnOnce(texts::Client) -> Fut,
    Fut: Future<Output = ()>,
{
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let repo =
        Arc::new(WikiRepo::new(store.clone(), ME).with_log_purge(Arc::new(ports::Direct(store))));
    with_provider(repo.clone(), repo, test).await;
}

/// A real server in front of this provider.
pub async fn with_provider<F, Fut>(texts: Arc<dyn Texts>, feed: Arc<dyn TextFeed>, test: F)
where
    F: FnOnce(texts::Client) -> Fut,
    Fut: Future<Output = ()>,
{
    with_page_size(TextsServer::new(texts, feed), test).await;
}

/// A real server, as configured (a small page size, for example).
pub async fn with_page_size<F, Fut>(server: TextsServer, test: F)
where
    F: FnOnce(texts::Client) -> Fut,
    Fut: Future<Output = ()>,
{
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("wiki.sock");
            let root = server.into_client();
            let bound = bind_socket(&path).await.unwrap();
            let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
            let serving = tokio::task::spawn_local(serve(bound, root.client, async {
                let _ = stopped.await;
            }));
            let connection = connect::<texts::Client>(&path).await.unwrap();
            test(connection.client.clone()).await;
            drop(connection);
            let _ = stop.send(());
            serving.await.unwrap().unwrap();
        })
        .await;
}

pub fn set_id(out: fs_interfaces::common_capnp::uuid::Builder<'_>, id: Uuid) {
    write_uuid(out, Some(id));
}

/// Create a text and answer it.
pub async fn create(root: &texts::Client, path: &str, markdown: &str) -> Got {
    try_create(root, path, markdown).await.unwrap()
}

pub async fn try_create(
    root: &texts::Client,
    path: &str,
    markdown: &str,
) -> Result<Got, capnp::Error> {
    let mut req = root.create_request();
    req.get().set_path(path);
    req.get().set_markdown(markdown);
    let answer = req.send().promise.await?;
    Ok(read_text(answer.get()?.get_text()?))
}

/// The text with this id, if there is one.
pub async fn get(root: &texts::Client, id: Uuid) -> Option<Got> {
    let mut req = root.get_request();
    set_id(req.get().init_id(), id);
    let answer = req.send().promise.await.unwrap();
    let r = answer.get().unwrap();
    r.get_found().then(|| read_text(r.get_text().unwrap()))
}

/// The Markdown of a text.
pub async fn export(root: &texts::Client, id: Uuid) -> String {
    let mut req = root.export_text_request();
    set_id(req.get().init_id(), id);
    req.get().set_media_type("text/markdown");
    let answer = req.send().promise.await.unwrap();
    answer
        .get()
        .unwrap()
        .get_text()
        .unwrap()
        .to_string()
        .unwrap()
}

/// All paths below `prefix`, following the cursors.
pub async fn list_paths(root: &texts::Client, prefix: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cursor: Vec<u8> = vec![];
    loop {
        let mut req = root.list_request();
        req.get().set_prefix(prefix);
        req.get().init_cursor().set_opaque(&cursor);
        let answer = req.send().promise.await.unwrap();
        let page = answer.get().unwrap().get_page().unwrap();
        out.extend(page.get_items().unwrap().iter().map(|t| read_text(t).path));
        cursor = page.get_next().unwrap().get_opaque().unwrap().to_vec();
        if cursor.is_empty() {
            return out;
        }
    }
}

/// The typed problem inside an error: its type below `PROBLEMS`, its status and its detail.
pub fn problem(error: &capnp::Error) -> (String, u16, String) {
    let p = Problem::from_error(error).expect("a typed problem");
    (
        p.problem_type.strip_prefix(PROBLEMS).unwrap().to_string(),
        p.status,
        p.detail,
    )
}

pub fn kind(error: &capnp::Error) -> (String, u16) {
    let (t, s, _) = problem(error);
    (t, s)
}
