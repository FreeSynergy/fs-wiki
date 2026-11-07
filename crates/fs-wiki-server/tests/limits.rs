//! What does not fit one message, as a consumer sees it: refused or cut, and the connection stays.
//!
//! Pattern: Example-based integration tests.

mod common;

use std::sync::Arc;

use common::{create, get, kind, list_paths, read_text, set_id, try_create, with_provider, ME};
use fs_interfaces::rpc::{MAX_INTAKE_BYTES, MAX_PAGE_BYTES};
use fs_store::{Store, TursoStore};
use fs_wiki_core::{Texts, WikiRepo};
use fs_wiki_server::MAX_SOURCE_BYTES;

const TOO_LARGE: &str = "https://freesynergy.net/problems/too-large";

fn is_too_large(error: &capnp::Error) {
    let p = fs_interfaces::rpc::Problem::from_error(error).expect("a typed problem");
    assert_eq!((p.problem_type.as_str(), p.status), (TOO_LARGE, 413));
}

#[tokio::test(flavor = "current_thread")]
async fn a_request_above_the_intake_limit_is_refused_and_the_connection_stays() {
    with_server_ok(|root| async move {
        let source = "x".repeat(MAX_SOURCE_BYTES + 1);
        is_too_large(&try_create(&root, "big", &source).await.err().unwrap());
        let big = "x".repeat(MAX_INTAKE_BYTES + 1);
        let mut req = root.edit_request();
        set_id(req.get().init_id(), create(&root, "a", "x\n").await.id);
        req.get().set_document(big.as_str());
        is_too_large(&req.send().promise.await.err().unwrap());
        assert_eq!(list_paths(&root, "").await, ["a"]);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn markdown_whose_document_would_not_fit_an_answer_is_refused_before_it_is_stored() {
    with_server_ok(|root| async move {
        // Short paragraphs: the document is about 100 times larger than the text.
        let blown = "a\n\n".repeat(MAX_SOURCE_BYTES / 6);
        is_too_large(&try_create(&root, "blown", &blown).await.err().unwrap());
        assert!(list_paths(&root, "").await.is_empty(), "nothing was stored");
        assert!(try_create(&root, "fine", "small\n").await.is_ok());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_list_that_would_not_fit_one_message_is_cut_into_pages_by_weight() {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let repo = Arc::new(WikiRepo::new(store, ME));
    // Three texts of about 14 MB as a document: no 32 MiB page can hold them all.
    let body = format!("{}\n\n", "word ".repeat(200)).repeat(6_000);
    for i in 0..3 {
        repo.create(&format!("big/{i}"), &format!("{body}\n"))
            .await
            .unwrap();
    }
    with_provider(repo.clone(), repo, |root| async move {
        let mut cursor: Vec<u8> = vec![];
        let (mut pages, mut seen) = (0, vec![]);
        loop {
            let mut req = root.list_request();
            req.get().init_cursor().set_opaque(&cursor);
            let answer = req.send().promise.await.unwrap();
            let page = answer.get().unwrap().get_page().unwrap();
            let items = page.get_items().unwrap();
            let bytes: usize = items.iter().map(|t| read_text(t).document.len()).sum();
            assert!(bytes <= MAX_PAGE_BYTES, "{bytes}");
            seen.extend(items.iter().map(|t| read_text(t).path));
            pages += 1;
            cursor = page.get_next().unwrap().get_opaque().unwrap().to_vec();
            if cursor.is_empty() {
                break;
            }
        }
        assert!(pages > 1, "the page was cut by weight");
        let want: Vec<String> = (0..3).map(|i| format!("big/{i}")).collect();
        assert_eq!(seen, want, "and every text came exactly once");
        let _ = (get, kind);
    })
    .await;
}

use common::with_server as with_server_ok;
