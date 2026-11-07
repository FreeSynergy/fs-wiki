use std::sync::Arc;
use std::time::Instant;
use fs_store::{Store, TursoStore, Query};
use fs_wiki_core::{Texts, WikiRepo};

#[tokio::main]
async fn main() {
    let store: Arc<dyn Store> = Arc::new(TursoStore::open_in_memory().await.unwrap());
    let wiki = WikiRepo::new(store.clone(), "https://example.org/a");
    let mut total = 0;
    for target in [100usize, 200, 400] {
        while total < target { wiki.create(&format!("d/p{total:05}"), "hello [[x]]\n\nworld\n").await.unwrap(); total += 1; }
        let t = Instant::now(); for i in 0..20 { wiki.get_by_path(&format!("d/p{i:05}")).await.unwrap(); } let a = t.elapsed()/20;
        let t = Instant::now(); for i in 0..20 { wiki.get_by_path(&format!("zz/{i:05}")).await.unwrap(); } let a2 = t.elapsed()/20;
        let t = Instant::now(); let q = Query::new().of_type("Article").in_context("texts").with_index("path","d/p00001"); for _ in 0..20 { store.query(&q).await.unwrap(); } let b = t.elapsed()/20;
        let t = Instant::now(); for i in 0..10 { wiki.create(&format!("new{target}/{i}"), "hello\n").await.unwrap(); total += 1;} let c = t.elapsed()/10;
        println!("n={target} get_by_path hit {a:?} miss {a2:?} raw query {b:?} create {c:?}");
    }
}
