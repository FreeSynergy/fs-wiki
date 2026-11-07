//! `list` and `find` over a real socket: path order, prefix, cursor across pages.
//!
//! Pattern: Example-based integration tests.

mod common;

use common::{create, kind, list_paths, read_text, with_server};

#[tokio::test(flavor = "current_thread")]
async fn list_follows_the_cursor_through_all_pages_in_path_order() {
    with_server(|root| async move {
        let mut want = vec![];
        for i in 0..250 {
            let path = format!("docs/p{i:03}");
            create(&root, &path, "x\n").await;
            want.push(path);
        }
        create(&root, "other/q", "x\n").await;
        let all = list_paths(&root, "docs/").await;
        assert_eq!(all, want, "250 texts, 100 per page, no gap and no repeat");
        assert_eq!(list_paths(&root, "").await.len(), 251);
        assert_eq!(list_paths(&root, "other").await, ["other/q"]);
        assert!(list_paths(&root, "nothing").await.is_empty());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn find_matches_name_path_and_text_and_pages_like_list() {
    with_server(|root| async move {
        create(&root, "guide/a", "# Kanban boards\n\nplain\n").await;
        create(&root, "guide/b", "mentions a Kanban in the text\n").await;
        create(&root, "kanban/c", "nothing\n").await;
        create(&root, "guide/d", "unrelated\n").await;
        let mut req = root.find_request();
        req.get().set_term("kanban");
        req.get().init_cursor();
        let answer = req.send().promise.await.unwrap();
        let page = answer.get().unwrap().get_page().unwrap();
        let mut paths: Vec<_> = page.get_items().unwrap().iter().map(|t| read_text(t).path).collect();
        paths.sort();
        assert_eq!(paths, ["guide/a", "guide/b", "kanban/c"]);
        assert!(page.get_next().unwrap().get_opaque().unwrap().is_empty());
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn an_empty_term_and_a_foreign_cursor_are_invalid_requests() {
    with_server(|root| async move {
        create(&root, "a", "x\n").await;
        let mut req = root.find_request();
        req.get().set_term("");
        let error = req.send().promise.await.err().unwrap();
        assert_eq!(kind(&error), ("texts/invalid".into(), 422));

        let mut req = root.list_request();
        req.get().init_cursor().set_opaque(&[9, 9, 9]);
        let error = req.send().promise.await.err().unwrap();
        assert_eq!(kind(&error).1, 422);
        assert_eq!(list_paths(&root, "").await, ["a"], "the connection stays");
    })
    .await;
}
