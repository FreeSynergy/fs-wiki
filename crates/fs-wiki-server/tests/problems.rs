//! The facts behind a problem arrive as extensions (`id`, `kind`, `issue`, `path`, `size`, `limit`),
//! so a client does not cut them out of the detail text. The detail text is as it was.

mod common;

use common::{create, set_id, try_create, with_server};
use fs_interfaces::rpc::Problem;
use fs_wiki_server::MAX_SOURCE_BYTES;
use uuid::Uuid;

fn p(error: &capnp::Error) -> Problem {
    Problem::from_error(error).expect("a typed problem")
}

#[tokio::test(flavor = "current_thread")]
async fn an_unknown_text_and_an_unknown_folder_name_what_was_not_found() {
    with_server(|root| async move {
        let ghost = Uuid::now_v7();
        let mut req = root.delete_request();
        set_id(req.get().init_id(), ghost);
        let found = p(&req.send().promise.await.err().unwrap());
        assert_eq!(found.ext("id"), Some(ghost.to_string().as_str()));
        assert_eq!(found.ext("kind"), Some("page"));
        assert_eq!(found.detail, format!("text {ghost} not found"));

        let mut req = root.move_folder_request();
        req.get().set_prefix("nothing");
        req.get().set_path("c");
        let found = p(&req.send().promise.await.err().unwrap());
        assert_eq!(found.ext("kind"), Some("folder"));
        assert_eq!(found.ext("path"), Some("nothing"));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_taken_path_and_a_refused_move_name_the_path_in_the_way() {
    with_server(|root| async move {
        let a = create(&root, "a", "x\n").await;
        create(&root, "b", "y\n").await;
        create(&root, "d/x", "1\n").await;
        create(&root, "e/y", "2\n").await;
        let taken = p(&try_create(&root, "a", "z\n").await.err().unwrap());
        assert_eq!(taken.ext("path"), Some("a"));
        assert_eq!(taken.ext("kind"), Some("page"));
        let mut req = root.move_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_path("b");
        let taken = p(&req.send().promise.await.err().unwrap());
        assert_eq!(taken.ext("path"), Some("b"));

        let mut req = root.move_folder_request();
        req.get().set_prefix("d");
        req.get().set_path("e");
        let refused = p(&req.send().promise.await.err().unwrap());
        assert!(refused.problem_type.ends_with("texts/move-refused"));
        assert_eq!(refused.ext("kind"), Some("folder"));
        assert_eq!(
            refused.ext("path"),
            Some("e"),
            "the target that is in the way"
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn invalid_says_what_is_wrong_as_a_token() {
    with_server(|root| async move {
        let bad = p(&try_create(&root, "a//b", "z\n").await.err().unwrap());
        assert_eq!(bad.ext("issue"), Some("empty-segment"));
        assert_eq!(bad.ext("kind"), Some("page"));
        assert_eq!(bad.ext("path"), Some("a//b"));
        assert!(bad.detail.contains("empty segment"), "{}", bad.detail);

        let a = create(&root, "a", "x\n").await;
        let mut req = root.edit_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_document("not json");
        let bad = p(&req.send().promise.await.err().unwrap());
        assert_eq!(bad.ext("issue"), Some("document-unreadable"));

        let mut req = root.find_request();
        req.get().set_term("");
        let bad = p(&req.send().promise.await.err().unwrap());
        assert_eq!(bad.ext("issue"), Some("term-empty"));

        let mut req = root.move_folder_request();
        req.get().set_prefix("a");
        req.get().set_path("a/inner");
        let bad = p(&req.send().promise.await.err().unwrap());
        assert_eq!(bad.ext("issue"), Some("inside-itself"));
        assert_eq!(bad.ext("kind"), Some("folder"));
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_stale_edit_names_the_text_and_keeps_its_detail() {
    with_server(|root| async move {
        let a = create(&root, "a", "one\n").await;
        let mut req = root.edit_if_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_revision(a.revision.wrapping_add(2));
        req.get().set_document(a.document.as_str());
        let stale = p(&req.send().promise.await.err().unwrap());
        assert_eq!(stale.ext("id"), Some(a.id.to_string().as_str()));
        assert_eq!(stale.ext("kind"), Some("page"));
        assert!(stale.detail.starts_with("conflict:"), "{}", stale.detail);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn too_large_carries_size_and_limit() {
    with_server(|root| async move {
        let source = "x".repeat(MAX_SOURCE_BYTES + 1);
        let big = p(&try_create(&root, "big", &source).await.err().unwrap());
        assert_eq!(big.ext_u64("size"), Some(source.len() as u64));
        assert_eq!(big.ext_u64("limit"), Some(MAX_SOURCE_BYTES as u64));
    })
    .await;
}
