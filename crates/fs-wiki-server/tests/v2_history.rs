//! texts 0.2 over a real socket: edit with a summary, the typed conflict, history, revisions.
//!
//! Pattern: Example-based integration tests against the real server and a real in-memory store.

mod common;

use common::v2::{document_saying, edit_with, expect_revision, history, p, short};
use common::{create, get, set_id, with_server};

#[tokio::test(flavor = "current_thread")]
async fn a_stale_edit_with_is_the_typed_conflict_and_writes_nothing() {
    with_server(|root| async move {
        let a = create(&root, "a", "one\n").await;
        let doc = document_saying(&root, &a, "two\n").await;
        let second = edit_with(&root, a.id, Some(a.revision), &doc, "first fix")
            .await
            .unwrap();
        let err = edit_with(&root, a.id, Some(a.revision), &doc, "late")
            .await
            .err()
            .unwrap();
        let c = p(&err);
        assert_eq!(short(&err), "conflict");
        assert_eq!(c.status, 409);
        assert!(c.detail.starts_with("conflict:"), "{}", c.detail);
        assert_eq!(c.ext("id"), Some(a.id.to_string().as_str()));
        assert_eq!(c.ext("kind"), Some("text"));
        assert_eq!(c.ext("issue"), Some("stale-revision"));
        assert_eq!(c.ext_u64("revision"), Some(second.revision));
        assert_eq!(c.ext_u64("expected"), Some(a.revision));
        assert_eq!(get(&root, a.id).await.unwrap().revision, second.revision);
        let long = "x".repeat(201);
        let err = edit_with(&root, a.id, None, &doc, &long)
            .await
            .err()
            .unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).ext("issue")),
            ("invalid", Some("summary-too-long"))
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn history_is_paged_by_cursor_and_a_revision_can_be_read_and_restored() {
    with_server(|root| async move {
        let a = create(&root, "a", "v1\n").await;
        let mut now = a.clone();
        for i in 2..=5 {
            let doc = document_saying(&root, &now, &format!("v{i}\n")).await;
            now = edit_with(&root, a.id, None, &doc, &format!("step {i}"))
                .await
                .unwrap();
        }
        let (first, next) = history(&root, a.id, &[]).await;
        assert_eq!(first, [5, 4, 3, 2, 1], "one page holds them all");
        assert!(next.is_empty());
        let mut req = root.history_request();
        set_id(req.get().init_id(), a.id);
        let answer = req.send().promise.await.unwrap();
        let items = answer
            .get()
            .unwrap()
            .get_page()
            .unwrap()
            .get_items()
            .unwrap();
        assert_eq!(
            items.get(1).get_summary().unwrap().to_string().unwrap(),
            "step 4"
        );
        assert_eq!(
            items.get(0).get_author().unwrap().to_string().unwrap(),
            common::ME
        );
        assert_eq!(items.get(0).get_revision(), now.revision);

        let mut req = root.get_revision_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_number(1);
        let answer = req.send().promise.await.unwrap();
        let r = answer.get().unwrap();
        assert!(r.get_found());
        assert_eq!(
            r.get_revision()
                .unwrap()
                .get_document()
                .unwrap()
                .to_string()
                .unwrap(),
            a.document
        );
        let mut req = root.get_revision_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_number(99);
        assert!(!req.send().promise.await.unwrap().get().unwrap().get_found());

        let mut req = root.restore_revision_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_number(1);
        common::v2::expect_revision(req.get().init_expect(), Some(1));
        let err = req.send().promise.await.err().unwrap();
        assert_eq!(short(&err), "conflict", "a stale expectation");
        let mut req = root.restore_revision_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_number(1);
        expect_revision(req.get().init_expect(), Some(now.revision));
        let back = req.send().promise.await.unwrap();
        let back = common::read_text(back.get().unwrap().get_text().unwrap());
        assert_eq!(back.document, a.document);
        let (all, _) = history(&root, a.id, &[]).await;
        assert_eq!(all, [6, 5, 4, 3, 2, 1]);
    })
    .await;
}

/// A server whose pages hold two items, in front of a fresh store.
async fn small_pages<F, Fut>(test: F)
where
    F: FnOnce(fs_interfaces::texts_capnp::texts::Client) -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let store: std::sync::Arc<dyn fs_objects::Store> =
        std::sync::Arc::new(fs_objects::TursoStore::open_in_memory().await.unwrap());
    let repo = std::sync::Arc::new(
        fs_wiki_core::WikiRepo::new(store.clone(), common::ME)
            .with_log_purge(std::sync::Arc::new(common::ports::Direct(store))),
    );
    let server = fs_wiki_server::TextsServer::new(repo.clone(), repo).with_page_size(2);
    common::with_page_size(server, test).await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_long_history_continues_with_the_cursor_and_unknowns_are_typed() {
    small_pages(|root| async move {
        let a = create(&root, "a", "v1\n").await;
        let mut now = a.clone();
        for i in 2..=7 {
            let doc = document_saying(&root, &now, &format!("v{i}\n")).await;
            now = edit_with(&root, a.id, None, &doc, "").await.unwrap();
        }
        let (mut seen, mut cursor) = (vec![], vec![]);
        loop {
            let (numbers, next) = history(&root, a.id, &cursor).await;
            assert!(numbers.len() <= 2);
            seen.extend(numbers);
            cursor = next;
            if cursor.is_empty() {
                break;
            }
        }
        assert_eq!(seen, [7, 6, 5, 4, 3, 2, 1]);
        let mut req = root.history_request();
        set_id(req.get().init_id(), a.id);
        req.get().init_cursor().set_opaque(&[9, 9]);
        let bad = req.send().promise.await.err().unwrap();
        assert_eq!(
            p(&bad).problem_type,
            "https://freesynergy.net/problems/invalid-request"
        );
        let ghost = uuid::Uuid::now_v7();
        let mut req = root.history_request();
        set_id(req.get().init_id(), ghost);
        let err = req.send().promise.await.err().unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).ext("kind")),
            ("not-found", Some("text"))
        );
        let purge = |n: u64| {
            let mut req = root.purge_revision_request();
            set_id(req.get().init_id(), a.id);
            req.get().set_number(n);
            req.send().promise
        };
        let err = purge(99).await.err().unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).ext("kind")),
            ("not-found", Some("revision"))
        );
        let err = purge(7).await.err().unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).ext("issue")),
            ("invalid", Some("current-revision"))
        );
        purge(1).await.unwrap();
        assert_eq!(history(&root, a.id, &[]).await.0, [7, 6]);
        let (rest, _) = history(&root, a.id, &common_cursor(&root, a.id).await).await;
        assert_eq!(rest, [5, 4]);
    })
    .await;
}

/// The cursor behind the first page.
async fn common_cursor(
    root: &fs_interfaces::texts_capnp::texts::Client,
    id: uuid::Uuid,
) -> Vec<u8> {
    history(root, id, &[]).await.1
}
