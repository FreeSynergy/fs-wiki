//! A cursor is a place in the order of the texts, not a count: texts that come or go elsewhere
//! change neither what is read next nor what was read.

mod common;

use std::sync::Arc;

use common::{create, kind, read_text, with_page_size, ME};
use fs_interfaces::texts_capnp::texts;
use fs_store::{Store, TursoStore};
use fs_wiki_core::WikiRepo;
use fs_wiki_server::TextsServer;

async fn small_pages<F, Fut>(size: usize, test: F)
where
    F: FnOnce(texts::Client) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let repo = Arc::new(WikiRepo::new(store, ME));
    with_page_size(TextsServer::new(repo.clone(), repo).with_page_size(size), test).await;
}

/// One page of `list` after `cursor`: its paths and the cursor behind it.
async fn page(root: &texts::Client, prefix: &str, cursor: &[u8]) -> (Vec<String>, Vec<u8>) {
    let mut req = root.list_request();
    req.get().set_prefix(prefix);
    req.get().init_cursor().set_opaque(cursor);
    let answer = req.send().promise.await.unwrap();
    let page = answer.get().unwrap().get_page().unwrap();
    let paths = page.get_items().unwrap().iter().map(|t| read_text(t).path).collect();
    (paths, page.get_next().unwrap().get_opaque().unwrap().to_vec())
}

#[tokio::test(flavor = "current_thread")]
async fn texts_added_or_removed_before_the_cursor_neither_repeat_nor_skip() {
    small_pages(3, |root| async move {
        for i in (10..20).step_by(2) {
            create(&root, &format!("n/{i}"), "x\n").await;
        }
        let (first, cursor) = page(&root, "n/", &[]).await;
        assert_eq!(first, ["n/10", "n/12", "n/14"]);
        // One more before the cursor, and the first one gone: a position would now name the wrong text.
        let early = create(&root, "n/11", "x\n").await;
        let mut req = root.delete_request();
        common::set_id(req.get().init_id(), early.id);
        req.send().promise.await.unwrap();
        create(&root, "n/09", "x\n").await;
        let (second, cursor) = page(&root, "n/", &cursor).await;
        assert_eq!(second, ["n/16", "n/18"]);
        assert!(cursor.is_empty());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn paths_that_differ_only_in_case_come_once_each_whatever_the_page_size() {
    for size in [1, 2, 3] {
        small_pages(size, |root| async move {
            for path in ["Alpha", "alpha", "ALPHA", "beta", "Beta"] {
                // The same title, so name and key are equal and only the id tells them apart.
                create(&root, path, "# Same\n").await;
            }
            let (mut all, mut cursor) = (vec![], vec![]);
            loop {
                let (paths, next) = page(&root, "", &cursor).await;
                all.extend(paths);
                cursor = next;
                if cursor.is_empty() {
                    break;
                }
            }
            all.sort();
            assert_eq!(all, ["ALPHA", "Alpha", "Beta", "alpha", "beta"], "size {size}");
        })
        .await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn a_cursor_of_the_old_form_is_refused_and_the_connection_stays() {
    small_pages(3, |root| async move {
        create(&root, "a", "x\n").await;
        // The first byte 1 was a position in the whole list.
        let mut req = root.list_request();
        req.get().init_cursor().set_opaque(&[1, 3, 0, 0, 0, 0, 0, 0, 0]);
        let error = req.send().promise.await.err().unwrap();
        assert_eq!(kind(&error), ("invalid-request".into(), 422));
        assert_eq!(page(&root, "", &[]).await.0, ["a"]);
    })
    .await;
}
