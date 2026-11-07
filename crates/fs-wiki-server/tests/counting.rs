//! Reading all pages, and importing files, must cost work that grows with the data, not with its
//! square. The count is of rows the store hands back (and, for a query the store sorts, its
//! matches), not of seconds: a fixed time says nothing on another machine.

mod common;

use std::sync::atomic::Ordering;

use common::counting::Counting;
use common::{list_paths, with_page_size};
use fs_interfaces::texts_capnp::texts;
use fs_wiki_core::Texts;
use fs_wiki_server::TextsServer;

const PAGE: usize = 8;

#[derive(Clone, Copy)]
enum Read {
    List,
    Find,
}

async fn pages_of(root: &texts::Client, how: Read) -> usize {
    match how {
        Read::List => list_paths(root, "").await.len(),
        Read::Find => {
            let (mut seen, mut cursor) = (0, vec![]);
            loop {
                let mut req = root.find_request();
                req.get().set_term("page");
                req.get().init_cursor().set_opaque(&cursor);
                let answer = req.send().promise.await.unwrap();
                let page = answer.get().unwrap().get_page().unwrap();
                seen += page.get_items().unwrap().len() as usize;
                cursor = page.get_next().unwrap().get_opaque().unwrap().to_vec();
                if cursor.is_empty() {
                    return seen;
                }
            }
        }
    }
}

/// Rows read to page through `n` texts with `how`, and how many texts came back.
async fn rows_for(n: usize, how: Read) -> (usize, usize) {
    let (counting, repo) = Counting::wiki().await;
    for i in 0..n {
        let path = format!("docs/{:05}", (i * 7919) % n);
        repo.create(&path, "# A page\n\nsome text\n").await.unwrap();
    }
    counting.reset();
    let seen = std::rc::Rc::new(std::cell::Cell::new(0));
    let slot = seen.clone();
    let server = TextsServer::new(repo.clone(), repo).with_page_size(PAGE);
    with_page_size(server, |root| async move {
        slot.set(pages_of(&root, how).await);
    })
    .await;
    (counting.rows(), seen.get())
}

#[tokio::test(flavor = "current_thread")]
async fn reading_all_pages_costs_rows_in_proportion_to_the_data() {
    for (name, how) in [("list", Read::List), ("find", Read::Find)] {
        let (small, seen_small) = rows_for(80, how).await;
        let (large, seen_large) = rows_for(160, how).await;
        assert_eq!((seen_small, seen_large), (80, 160), "{name}: every text once");
        #[allow(clippy::cast_precision_loss)]
        let ratio = large as f64 / small as f64;
        eprintln!("{name}: {small} rows for 80 texts, {large} rows for 160, ratio {ratio:.2}");
        // Linear is about 2, a whole list read for every page is about 4.
        assert!(ratio < 3.0, "{name}: {small} -> {large} rows, ratio {ratio:.2}");
        assert!(large < 2 * 160 + 10 * 160 / PAGE, "{name}: {large} rows for 160");
    }
}

/// The work of importing `n` files over the socket, then importing the same files again with
/// a changed text (an outside change of every one).
async fn import_work(n: usize) -> (usize, usize) {
    let (counting, repo) = Counting::wiki().await;
    let work = std::rc::Rc::new(std::cell::Cell::new((0, 0)));
    let slot = work.clone();
    let counted = counting.clone();
    with_page_size(TextsServer::new(repo.clone(), repo), |root| async move {
        for round in 0..2 {
            for i in 0..n {
                let mut req = root.import_text_request();
                req.get().set_path(format!("docs/{i:05}"));
                req.get().set_media_type("text/markdown");
                req.get().set_text(format!("# Page {i}\n\nround {round}\n"));
                req.send().promise.await.unwrap();
            }
            let now = counted.touched.load(Ordering::Relaxed);
            let before = slot.get().0;
            slot.set(if round == 0 { (now, 0) } else { (before, now - before) });
        }
    })
    .await;
    work.get()
}

#[tokio::test(flavor = "current_thread")]
async fn importing_files_costs_work_in_proportion_to_their_number() {
    let (first_small, again_small) = import_work(50).await;
    let (first_large, again_large) = import_work(100).await;
    for (what, small, large) in [
        ("new files", first_small, first_large),
        ("changed files", again_small, again_large),
    ] {
        #[allow(clippy::cast_precision_loss)]
        let ratio = large.max(1) as f64 / small.max(1) as f64;
        eprintln!("{what}: {small} for 50 files, {large} for 100, ratio {ratio:.2}");
        assert!(ratio < 3.0, "{what}: {small} -> {large}, ratio {ratio:.2}");
    }
}
