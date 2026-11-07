//! What a consumer gets for a request the server cannot take: a typed problem, and the connection
//! stays.
//!
//! Pattern: Example-based integration tests.

mod common;

use common::{create, get, kind, list_paths, problem, set_id, try_create, with_server};

#[tokio::test(flavor = "current_thread")]
async fn bad_paths_and_taken_paths_are_typed_and_write_nothing() {
    with_server(|root| async move {
        create(&root, "a", "x\n").await;
        for bad in ["", "a//b", "../x", "a#b", "/lead"] {
            let error = try_create(&root, bad, "y\n").await.err().unwrap();
            assert_eq!(kind(&error), ("texts/invalid".into(), 422), "{bad:?}");
        }
        let taken = try_create(&root, "a", "z\n").await.err().unwrap();
        assert_eq!(kind(&taken), ("texts/path-taken".into(), 409));
        assert_eq!(list_paths(&root, "").await, ["a"]);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_document_that_does_not_read_and_an_unknown_text_are_refused() {
    with_server(|root| async move {
        let a = create(&root, "a", "x\n").await;
        for text in ["", "not json", "{\"blocks\": 7}", "[]"] {
            let mut req = root.edit_request();
            set_id(req.get().init_id(), a.id);
            req.get().set_document(text);
            let error = req.send().promise.await.err().unwrap();
            assert_eq!(kind(&error), ("texts/invalid".into(), 422), "{text:?}");
        }
        let mut req = root.edit_request();
        set_id(req.get().init_id(), uuid::Uuid::now_v7());
        req.get().set_document(a.document.as_str());
        let error = req.send().promise.await.err().unwrap();
        assert_eq!(kind(&error), ("texts/not-found".into(), 404));
        let mut req = root.links_request();
        set_id(req.get().init_id(), uuid::Uuid::now_v7());
        let (_, status, _) = problem(&req.send().promise.await.err().unwrap());
        assert_eq!(status, 404);
        assert_eq!(
            get(&root, a.id).await.unwrap().document,
            a.document,
            "untouched"
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_request_without_an_id_or_with_an_empty_required_text_is_invalid() {
    with_server(|root| async move {
        let error = root.get_request().send().promise.await.err().unwrap();
        assert_eq!(kind(&error).1, 422, "no id");
        let error = root
            .get_by_path_request()
            .send()
            .promise
            .await
            .err()
            .unwrap();
        assert_eq!(kind(&error), ("texts/invalid".into(), 422));
        let error = root.backlinks_request().send().promise.await.err().unwrap();
        assert_eq!(kind(&error), ("texts/invalid".into(), 422));
        assert!(
            try_create(&root, "ok", "fine\n").await.is_ok(),
            "the connection stays"
        );
    })
    .await;
}
