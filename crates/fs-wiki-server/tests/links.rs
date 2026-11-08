//! `links` and `backlinks` over a real socket.
//!
//! Pattern: Example-based integration tests.

mod common;

use common::{create, set_id, with_server};
use fs_interfaces::rpc::read_uuid;

#[tokio::test(flavor = "current_thread")]
async fn links_lists_every_link_as_written_with_its_title_and_where_it_points() {
    with_server(|root| async move {
        let a = create(
            &root,
            "a",
            "See [[b#part|bee]], [c](c.md) and [out](https://example.org/x).\n",
        )
        .await;
        let mut req = root.links_request();
        set_id(req.get().init_id(), a.id);
        let answer = req.send().promise.await.unwrap();
        let rows = answer.get().unwrap().get_items().unwrap();
        let got: Vec<_> = rows
            .iter()
            .map(|r| {
                (
                    read_uuid(r.get_source().unwrap()).unwrap(),
                    r.get_target().unwrap().to_string().unwrap(),
                    r.get_at().unwrap().to_string().unwrap(),
                    r.get_title().unwrap().to_string().unwrap(),
                    r.get_internal(),
                )
            })
            .collect();
        let rows: Vec<_> = got
            .iter()
            .map(|g| (g.1.as_str(), g.3.as_str(), g.4))
            .collect();
        assert_eq!(
            rows,
            [
                ("b#part", "bee", true),
                ("c.md", "c", true),
                ("https://example.org/x", "out", false)
            ],
            "every link is a row; `internal` tells a text of the wiki from an outside address"
        );
        assert!(got.iter().all(|g| g.0 == a.id));
        assert!(
            got[0].2.contains(':') && got[0].2.contains('-'),
            "block:start-end: {}",
            got[0].2
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn backlinks_finds_the_texts_that_point_at_a_path() {
    with_server(|root| async move {
        let x = create(&root, "x", "to [[target]]\n").await;
        let y = create(&root, "y", "to [[target#h]] and [[elsewhere]]\n").await;
        create(&root, "z", "nothing\n").await;
        let mut req = root.backlinks_request();
        req.get().set_path("target");
        let answer = req.send().promise.await.unwrap();
        let rows = answer.get().unwrap().get_items().unwrap();
        let mut got: Vec<_> = rows
            .iter()
            .map(|r| {
                (
                    read_uuid(r.get_source().unwrap()).unwrap(),
                    r.get_target().unwrap().to_string().unwrap(),
                )
            })
            .collect();
        got.sort();
        let mut want = vec![(x.id, "target".to_string()), (y.id, "target#h".to_string())];
        want.sort();
        assert_eq!(got, want);
    })
    .await;
}
