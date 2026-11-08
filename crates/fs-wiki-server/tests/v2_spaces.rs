//! texts 0.2 over a real socket: spaces, folders and the order of children.
//!
//! Pattern: Example-based integration tests against the real server and a real in-memory store.

mod common;

use common::v2::{folder, names, p, short};
use common::{create, set_id, with_server};
use fs_interfaces::rpc::write_uuid;

#[tokio::test(flavor = "current_thread")]
async fn folders_are_made_ordered_renamed_and_deleted_when_empty() {
    with_server(|root| async move {
        let (space, path, kind) = folder(&root, None, "Manual", "").await.unwrap();
        assert_eq!((path.as_str(), kind.as_str()), ("manual", "Space"));
        let (guide, path, kind) = folder(&root, Some(space), "User guide", "").await.unwrap();
        assert_eq!(
            (path.as_str(), kind.as_str()),
            ("manual/user-guide", "Folder")
        );
        let a = create(&root, "manual/a", "# A\n").await;
        let b = create(&root, "manual/b", "# B\n").await;
        assert_eq!(names(&root, Some(space)).await, ["User guide", "A", "B"]);
        let mut req = root.reorder_request();
        set_id(req.get().init_id(), b.id);
        req.get()
            .set_placement(fs_interfaces::texts_capnp::Placement::First);
        let answer = req.send().promise.await.unwrap();
        assert_eq!(answer.get().unwrap().get_node().unwrap().get_position(), 0);
        assert_eq!(names(&root, Some(space)).await, ["B", "User guide", "A"]);
        let mut req = root.reorder_request();
        set_id(req.get().init_id(), a.id);
        req.get()
            .set_placement(fs_interfaces::texts_capnp::Placement::Before);
        let err = req.send().promise.await.err().unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).ext("issue")),
            ("invalid", Some("anchor"))
        );
        let mut req = root.reorder_request();
        set_id(req.get().init_id(), a.id);
        let err = req.send().promise.await.err().unwrap();
        assert_eq!(
            p(&err).ext("issue"),
            Some("placement"),
            "unspecified is refused"
        );

        let mut req = root.rename_folder_request();
        set_id(req.get().init_id(), guide);
        req.get().set_name("Guide");
        req.send().promise.await.unwrap();
        let mut req = root.get_folder_request();
        set_id(req.get().init_id(), guide);
        let answer = req.send().promise.await.unwrap();
        let r = answer.get().unwrap();
        assert!(r.get_found());
        assert_eq!(
            r.get_node()
                .unwrap()
                .get_name()
                .unwrap()
                .to_string()
                .unwrap(),
            "Guide"
        );
        let mut req = root.get_folder_request();
        set_id(req.get().init_id(), a.id);
        assert!(!req.send().promise.await.unwrap().get().unwrap().get_found());

        let delete = |id| {
            let mut req = root.delete_folder_request();
            set_id(req.get().init_id(), id);
            req.send().promise
        };
        let err = delete(space).await.err().unwrap();
        assert_eq!((short(&err).as_str(), p(&err).status), ("not-empty", 409));
        assert_eq!(p(&err).ext("id"), Some(space.to_string().as_str()));
        delete(guide).await.unwrap();
        let err = delete(guide).await.err().unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).ext("kind")),
            ("not-found", Some("folder"))
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_folder_with_a_bad_name_or_path_is_a_typed_problem() {
    with_server(|root| async move {
        let (space, _, _) = folder(&root, None, "Docs", "docs").await.unwrap();
        for (name, path, issue) in [
            ("Привет", "", "empty-slug"),
            ("N", "other/x", "outside-parent"),
            ("N", "docs/../x", "dot-segment"),
            ("N", "docs//x", "empty-segment"),
        ] {
            let err = folder(&root, Some(space), name, path).await.err().unwrap();
            assert_eq!(
                (short(&err).as_str(), p(&err).status),
                ("bad-path", 422),
                "{path}"
            );
            assert_eq!(p(&err).ext("issue"), Some(issue), "{path}");
        }
        let err = folder(&root, None, "Again", "docs").await.err().unwrap();
        assert_eq!((short(&err).as_str(), p(&err).status), ("path-taken", 409));
        assert_eq!(p(&err).ext("kind"), Some("folder"));
        let err = folder(&root, None, &"n".repeat(256), "long")
            .await
            .err()
            .unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).ext("issue")),
            ("invalid", Some("name-too-long"))
        );
        let mut req = root.create_folder_request();
        write_uuid(req.get().init_parent(), Some(uuid::Uuid::now_v7()));
        req.get().set_name("N");
        let err = req.send().promise.await.err().unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).ext("kind")),
            ("not-found", Some("folder"))
        );
    })
    .await;
}
