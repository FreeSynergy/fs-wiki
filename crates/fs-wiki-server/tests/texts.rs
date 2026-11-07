//! The core methods over a real socket: create, read, edit, compare-and-set edit, delete.
//!
//! Pattern: Example-based integration tests; values are compared, not only "it answered".

mod common;

use common::{create, export, get, kind, problem, read_text, set_id, with_server};
use fs_text::Document;

#[tokio::test(flavor = "current_thread")]
async fn a_created_text_reads_back_by_id_and_by_path_with_its_envelope() {
    with_server(|root| async move {
        let made = create(
            &root,
            "manual/start",
            "# Start here\n\nHello [[manual/next]].\n",
        )
        .await;
        assert_eq!(made.name, "Start here", "the name is derived (ADR 0032)");
        assert_eq!(made.context.as_deref(), Some("texts"));
        assert!(made.iri.unwrap().ends_with(&format!("/texts/{}", made.id)));
        assert_ne!(made.revision, 0);
        let doc: Document = serde_json::from_str(&made.document).unwrap();
        assert_eq!(doc.id, made.id, "the document carries the id of the text");

        assert_eq!(get(&root, made.id).await.unwrap().document, made.document);
        let mut req = root.get_by_path_request();
        req.get().set_path("manual/start");
        let answer = req.send().promise.await.unwrap();
        let by_path = read_text(answer.get().unwrap().get_text().unwrap());
        assert_eq!(by_path.id, made.id);
        assert_eq!(
            export(&root, made.id).await,
            "# Start here\n\nHello [[manual/next]].\n"
        );

        let mut missing = root.get_by_path_request();
        missing.get().set_path("manual/nothing");
        let answer = missing.send().promise.await.unwrap();
        assert!(!answer.get().unwrap().get_found());
        assert!(get(&root, uuid::Uuid::now_v7()).await.is_none());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn edit_replaces_the_document_and_keeps_id_path_and_iri() {
    with_server(|root| async move {
        let a = create(&root, "a", "one\n").await;
        let b = create(&root, "b", "two\n").await;
        let mut req = root.edit_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_document(b.document.as_str());
        let answer = req.send().promise.await.unwrap();
        let edited = read_text(answer.get().unwrap().get_text().unwrap());
        assert_eq!((edited.id, edited.path.as_str()), (a.id, "a"));
        assert_eq!(edited.iri, a.iri);
        assert_ne!(edited.revision, a.revision);
        let doc: Document = serde_json::from_str(&edited.document).unwrap();
        assert_eq!(doc.id, a.id, "the id of the document is forced to the text");
        assert_eq!(export(&root, a.id).await, "two\n");
        assert_eq!(export(&root, b.id).await, "two\n");
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn edit_if_writes_on_the_revision_read_and_refuses_a_stale_one_with_conflict() {
    with_server(|root| async move {
        let a = create(&root, "a", "one\n").await;
        let other = create(&root, "b", "two\n").await;
        let edit_if = |revision: u64| {
            let mut req = root.edit_if_request();
            set_id(req.get().init_id(), a.id);
            req.get().set_revision(revision);
            req.get().set_document(other.document.as_str());
            req.send().promise
        };
        let second = read_text(
            edit_if(a.revision)
                .await
                .unwrap()
                .get()
                .unwrap()
                .get_text()
                .unwrap(),
        );
        assert_ne!(second.revision, a.revision);

        let stale = edit_if(a.revision)
            .await
            .err()
            .expect("a stale writer is refused");
        let (kind, status, detail) = problem(&stale);
        assert_eq!((kind.as_str(), status), ("texts/conflict", 409));
        assert!(detail.starts_with("conflict:"), "{detail}");
        assert_eq!(get(&root, a.id).await.unwrap().revision, second.revision);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn delete_removes_the_text_and_a_second_delete_is_not_found() {
    with_server(|root| async move {
        let a = create(&root, "a", "one\n").await;
        let delete = |id| {
            let mut req = root.delete_request();
            set_id(req.get().init_id(), id);
            req.send().promise
        };
        delete(a.id).await.unwrap();
        assert!(get(&root, a.id).await.is_none());
        let again = delete(a.id).await.err().unwrap();
        assert_eq!(kind(&again), ("texts/not-found".into(), 404));
    })
    .await;
}
