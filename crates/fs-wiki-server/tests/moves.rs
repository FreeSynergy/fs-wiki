//! `move` and `moveFolder` over a real socket: links follow, clashes refuse.
//!
//! Pattern: Example-based integration tests.

mod common;

use common::{create, export, get, kind, list_paths, read_text, set_id, with_server};

#[tokio::test(flavor = "current_thread")]
async fn move_changes_the_path_and_the_links_in_other_texts_follow() {
    with_server(|root| async move {
        let a = create(&root, "old/a", "text\n").await;
        let b = create(&root, "b", "see [[old/a]]\n").await;
        let mut req = root.move_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_path("new/a");
        let answer = req.send().promise.await.unwrap();
        let moved = read_text(answer.get().unwrap().get_text().unwrap());
        assert_eq!((moved.id, moved.path.as_str()), (a.id, "new/a"));
        assert_eq!(export(&root, b.id).await, "see [[new/a]]\n");
        assert_ne!(get(&root, b.id).await.unwrap().revision, b.revision);
        assert_eq!(list_paths(&root, "").await, ["b", "new/a"]);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn move_to_a_taken_or_invalid_path_is_refused_and_writes_nothing() {
    with_server(|root| async move {
        let a = create(&root, "a", "x\n").await;
        create(&root, "b", "y\n").await;
        let mv = |path: &'static str| {
            let mut req = root.move_request();
            set_id(req.get().init_id(), a.id);
            req.get().set_path(path);
            req.send().promise
        };
        assert_eq!(kind(&mv("b").await.err().unwrap()), ("texts/path-taken".into(), 409));
        assert_eq!(kind(&mv("x//y").await.err().unwrap()), ("texts/invalid".into(), 422));
        assert_eq!(kind(&mv("").await.err().unwrap()), ("texts/invalid".into(), 422));
        assert_eq!(list_paths(&root, "").await, ["a", "b"]);
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn move_folder_moves_the_text_and_everything_below_and_links_follow() {
    with_server(|root| async move {
        let m = create(&root, "manual", "# M\n\n[[manual/a]]\n").await;
        let a = create(&root, "manual/a", "back [[manual]]\n").await;
        let d = create(&root, "manual/deep/b", "x\n").await;
        let o = create(&root, "other", "[[manual/deep/b]] and [[manual-old]]\n").await;
        create(&root, "manual-old", "stays\n").await;
        let mut req = root.move_folder_request();
        req.get().set_prefix("manual");
        req.get().set_path("guide");
        let answer = req.send().promise.await.unwrap();
        let moved: Vec<_> = answer.get().unwrap().get_moved().unwrap().iter().map(read_text).collect();
        let mut got: Vec<_> = moved.iter().map(|t| (t.id, t.path.as_str())).collect();
        got.sort_by_key(|g| g.1);
        assert_eq!(got, [(m.id, "guide"), (a.id, "guide/a"), (d.id, "guide/deep/b")]);
        assert_eq!(export(&root, a.id).await, "back [[guide]]\n");
        assert_eq!(export(&root, m.id).await, "# M\n\n[[guide/a]]\n");
        assert_eq!(export(&root, o.id).await, "[[guide/deep/b]] and [[manual-old]]\n");
        assert_eq!(
            list_paths(&root, "").await,
            ["guide", "guide/a", "guide/deep/b", "manual-old", "other"]
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn move_folder_onto_other_texts_or_from_nothing_is_refused_before_a_write() {
    with_server(|root| async move {
        create(&root, "a/x", "1\n").await;
        create(&root, "b/y", "2\n").await;
        let mv = |from: &'static str, to: &'static str| {
            let mut req = root.move_folder_request();
            req.get().set_prefix(from);
            req.get().set_path(to);
            req.send().promise
        };
        assert_eq!(kind(&mv("a", "b").await.err().unwrap()), ("texts/move-refused".into(), 409));
        assert_eq!(kind(&mv("nothing", "c").await.err().unwrap()), ("texts/not-found".into(), 404));
        assert_eq!(kind(&mv("a", "a/inner").await.err().unwrap()), ("texts/invalid".into(), 422));
        assert_eq!(list_paths(&root, "").await, ["a/x", "b/y"]);
    })
    .await;
}
