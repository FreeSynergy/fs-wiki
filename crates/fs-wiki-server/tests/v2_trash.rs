//! texts 0.2 over a real socket: the trash, and the typed problems around it.
//!
//! Pattern: Example-based integration tests against the real server and a real in-memory store.

mod common;

use common::v2::{p, short, trash, trash_ids};
use common::{create, get, list_paths, set_id, with_server};

#[tokio::test(flavor = "current_thread")]
async fn trash_hides_a_text_and_restore_or_purge_ends_it() {
    with_server(|root| async move {
        let a = create(&root, "a", "one\n").await;
        let b = create(&root, "b", "two\n").await;
        trash(&root, a.id).await.unwrap();
        assert!(get(&root, a.id).await.is_none());
        assert_eq!(list_paths(&root, "").await, ["b"]);
        let err = trash(&root, a.id).await.err().unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).status, p(&err).ext("kind")),
            ("not-found", 404, Some("text"))
        );
        trash(&root, b.id).await.unwrap();
        let (ids, next) = trash_ids(&root, &[]).await;
        assert_eq!(ids, [b.id, a.id], "the most recently deleted first");
        assert_eq!(next, [] as [u8; 0]);

        create(&root, "a", "in the way\n").await;
        let mut req = root.restore_from_trash_request();
        set_id(req.get().init_id(), a.id);
        let err = req.send().promise.await.err().unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).status, p(&err).ext("path")),
            ("path-taken", 409, Some("a"))
        );
        let mut req = root.restore_from_trash_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_path("x//y");
        let err = req.send().promise.await.err().unwrap();
        assert_eq!((short(&err).as_str(), p(&err).status), ("bad-path", 422));
        assert_eq!(p(&err).ext("issue"), Some("empty-segment"));
        let mut req = root.restore_from_trash_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_path("elsewhere");
        let answer = req.send().promise.await.unwrap();
        assert_eq!(
            common::read_text(answer.get().unwrap().get_text().unwrap()).path,
            "elsewhere"
        );

        let purge = |id| {
            let mut req = root.purge_request();
            set_id(req.get().init_id(), id);
            req.send().promise
        };
        let err = purge(a.id).await.err().unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).status),
            ("not-in-trash", 409)
        );
        purge(b.id).await.unwrap();
        assert_eq!(trash_ids(&root, &[]).await.0, [] as [uuid::Uuid; 0]);
        let mut req = root.restore_from_trash_request();
        set_id(req.get().init_id(), b.id);
        assert_eq!(short(&req.send().promise.await.err().unwrap()), "not-found");
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn the_trash_is_paged_by_cursor() {
    with_server(|root| async move {
        let mut ids = vec![];
        for i in 0..3 {
            let t = create(&root, &format!("t{i}"), "x\n").await;
            trash(&root, t.id).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_millis(3)).await;
            ids.push(t.id);
        }
        let (all, _) = trash_ids(&root, &[]).await;
        assert_eq!(all, [ids[2], ids[1], ids[0]]);
        let mut req = root.list_trash_request();
        req.get().init_cursor().set_opaque(&[3, 1]);
        assert_eq!(
            p(&req.send().promise.await.err().unwrap()).status,
            422,
            "a cursor of another method"
        );
    })
    .await;
}
