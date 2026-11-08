//! texts 0.2 over a real socket: search with snippets, filters, limits and a cursor.
//!
//! Pattern: Example-based integration tests against the real server and a real in-memory store.

mod common;

use common::v2::{p, short};
use common::{create, with_page_size, ME};
use fs_interfaces::texts_capnp::{texts, Tri};
use fs_wiki_server::TextsServer;
use std::sync::Arc;

type Found = (Vec<String>, Vec<u8>);

async fn search(
    root: &texts::Client,
    term: &str,
    prefix: bool,
    cursor: &[u8],
) -> Result<Found, capnp::Error> {
    let mut req = root.search_request();
    {
        let mut q = req.get().init_query();
        q.set_term(term);
        q.set_prefix(prefix);
    }
    req.get().init_cursor().set_opaque(cursor);
    let answer = req.send().promise.await?;
    let page = answer.get()?.get_page()?;
    let paths = page
        .get_items()?
        .iter()
        .map(|h| h.get_path().unwrap().to_string().unwrap())
        .collect();
    Ok((paths, page.get_next()?.get_opaque()?.to_vec()))
}

#[tokio::test(flavor = "current_thread")]
async fn snippets_carry_byte_ranges_and_filters_narrow_the_result() {
    common::with_server(|root| async move {
        let t = create(
            &root,
            "docs/tea",
            "# Tea\n\nÄpfel and a cup of tea, more TEA.\n",
        )
        .await;
        create(
            &root,
            "other/coffee",
            "# Coffee\n\nno tea here? only coffee\n",
        )
        .await;
        let mut req = root.search_request();
        req.get().init_query().set_term("TEA");
        let answer = req.send().promise.await.unwrap();
        let items = answer
            .get()
            .unwrap()
            .get_page()
            .unwrap()
            .get_items()
            .unwrap();
        assert_eq!(items.len(), 2);
        let hit = items
            .iter()
            .find(|h| h.get_path().unwrap() == "docs/tea")
            .unwrap();
        assert_eq!(hit.get_updated_by().unwrap().to_string().unwrap(), ME);
        let body = hit
            .get_snippets()
            .unwrap()
            .iter()
            .find(|s| s.get_field().unwrap() == "body")
            .unwrap();
        let text = body.get_text().unwrap().to_string().unwrap();
        assert!(!body.get_cut_before() && !body.get_cut_after());
        let marks: Vec<&str> = body
            .get_highlights()
            .unwrap()
            .iter()
            .map(|h| &text[h.get_start() as usize..h.get_end() as usize])
            .collect();
        assert_eq!(marks, ["tea", "TEA"]);

        let mut req = root.search_request();
        {
            let mut q = req.get().init_query();
            q.set_term("tea");
            let mut f = q.init_filter();
            f.set_path_prefix("docs/");
            f.set_redirect(Tri::Exclude);
            f.set_author(ME);
        }
        let answer = req.send().promise.await.unwrap();
        let items = answer
            .get()
            .unwrap()
            .get_page()
            .unwrap()
            .get_items()
            .unwrap();
        assert_eq!(
            (items.len(), items.get(0).get_id().unwrap().get_hi() != 0),
            (1, true)
        );
        assert_eq!(common::get(&root, t.id).await.unwrap().path, "docs/tea");
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn the_term_limits_are_typed_and_the_cursor_continues_a_result() {
    let store: Arc<dyn fs_objects::Store> =
        Arc::new(fs_objects::TursoStore::open_in_memory().await.unwrap());
    let repo = Arc::new(fs_wiki_core::WikiRepo::new(store, ME));
    with_page_size(
        TextsServer::new(repo.clone(), repo).with_page_size(2),
        |root| async move {
            for i in 0..5 {
                create(&root, &format!("p{i}"), "# Page\n\nkiwi\n").await;
            }
            let (mut seen, mut cursor) = (vec![], vec![]);
            loop {
                let (paths, next) = search(&root, "kiwi", false, &cursor).await.unwrap();
                assert!(paths.len() <= 2);
                seen.extend(paths);
                cursor = next;
                if cursor.is_empty() {
                    break;
                }
            }
            seen.sort();
            assert_eq!(seen, ["p0", "p1", "p2", "p3", "p4"], "each hit once");
            let issue = |e: capnp::Error| (short(&e), p(&e).ext("issue").map(str::to_string));
            let long = "x".repeat(257);
            let err = search(&root, &long, false, &[]).await.err().unwrap();
            assert_eq!(issue(err), ("invalid".into(), Some("term-too-long".into())));
            let err = search(&root, "a b c d e f g h i", false, &[])
                .await
                .err()
                .unwrap();
            assert_eq!(
                issue(err),
                ("invalid".into(), Some("too-many-words".into()))
            );
            let err = search(&root, "kiwi x", true, &[]).await.err().unwrap();
            assert_eq!(
                issue(err),
                ("invalid".into(), Some("prefix-too-short".into()))
            );
            assert_eq!(search(&root, "ki", true, &[]).await.unwrap().0.len(), 2);
            let err = search(&root, "kiwi", false, &[3, 0]).await.err().unwrap();
            assert_eq!(p(&err).status, 422);
        },
    )
    .await;
}
