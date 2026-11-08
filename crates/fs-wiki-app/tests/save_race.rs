//! Saves from several threads: an accepted save is never overwritten by another one.
//!
//! Pattern: Integration test over a real in-memory wiki and real threads.

mod common;

use std::sync::Arc;

use common::{markdown_at, wiki};
use fs_components_workbenches::DocumentSource;
use fs_wiki_app::WikiSource;
use fs_wiki_core::Texts;

#[test]
fn every_accepted_save_is_in_the_final_text() {
    let (rt, w) = wiki();
    rt.block_on(w.create("p.md", "start\n")).unwrap();
    let src = Arc::new(WikiSource::new(w.wiki(), rt.handle().clone(), vec![]));
    let threads: Vec<_> = (0..4)
        .map(|n| {
            let src = src.clone();
            std::thread::spawn(move || {
                let mut accepted = vec![];
                for i in 0..150 {
                    let word = format!("w{n}x{i}");
                    let cur = src.source_text("p.md").unwrap();
                    let text = format!("{}\n\n{word}\n", cur.text.trim());
                    if src.save("p.md", &text, cur.revision.as_deref()).is_ok() {
                        accepted.push(word);
                    }
                }
                accepted
            })
        })
        .collect();
    let accepted: Vec<String> = threads
        .into_iter()
        .flat_map(|t| t.join().unwrap())
        .collect();
    let last = markdown_at(&rt, &w, "p.md").unwrap();
    let missing: Vec<_> = accepted
        .iter()
        .filter(|x| !last.contains(x.as_str()))
        .collect();
    assert_ne!(accepted, [] as [std::string::String; 0]);
    assert!(
        missing.is_empty(),
        "{} accepted, lost: {missing:?}",
        accepted.len()
    );
}
