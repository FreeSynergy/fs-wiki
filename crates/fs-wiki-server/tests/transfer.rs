//! `importText` and `exportText` over a real socket.
//!
//! Pattern: Example-based integration tests.

mod common;

use common::{create, export, get, problem, read_text, set_id, with_server};

async fn import(
    root: &fs_interfaces::texts_capnp::texts::Client,
    path: &str,
    media: &str,
    text: &str,
) -> Result<common::Got, capnp::Error> {
    let mut req = root.import_text_request();
    req.get().set_path(path);
    req.get().set_media_type(media);
    req.get().set_text(text);
    let answer = req.send().promise.await?;
    Ok(read_text(answer.get()?.get_entry()?))
}

#[tokio::test(flavor = "current_thread")]
async fn an_import_creates_then_applies_only_the_difference_and_an_unchanged_file_writes_nothing() {
    with_server(|root| async move {
        let first = import(&root, "notes/a", "text/markdown", "# A\n\none\n")
            .await
            .unwrap();
        assert_eq!((first.path.as_str(), first.name.as_str()), ("notes/a", "A"));
        let same = import(&root, "notes/a", "text/markdown", "# A\n\none\n")
            .await
            .unwrap();
        assert_eq!(same.revision, first.revision, "nothing was written");
        let changed = import(
            &root,
            "notes/a",
            "Text/Markdown; charset=utf-8",
            "# A\n\ntwo\n",
        )
        .await
        .unwrap();
        assert_eq!(
            changed.id, first.id,
            "an outside change is an edit of the same text"
        );
        assert_ne!(changed.revision, first.revision);
        assert_eq!(export(&root, first.id).await, "# A\n\ntwo\n");
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn an_unknown_media_type_is_the_explicit_not_possible_of_that_method() {
    with_server(|root| async move {
        let a = create(&root, "a", "x\n").await;
        let error = import(&root, "b", "text/html", "<p>x</p>")
            .await
            .err()
            .unwrap();
        assert!(fs_interfaces::rpc::Problem::from_error(&error)
            .unwrap()
            .is_unsupported());
        let (_, status, detail) = problem(&error);
        assert_eq!(status, 501);
        assert!(detail.starts_with("text.import"), "{detail}");

        let mut req = root.export_text_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_media_type("application/pdf");
        let error = req.send().promise.await.err().unwrap();
        let (_, status, detail) = problem(&error);
        assert_eq!(status, 501);
        assert!(detail.starts_with("text.export"), "{detail}");
        assert!(get(&root, a.id).await.is_some(), "and the connection stays");
    })
    .await;
}
