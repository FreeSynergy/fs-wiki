//! A folder move whose answer would not fit one message: the server does the move and answers
//! `too-large`; the client must report the move as done, with the moved texts, as the core does.
//!
//! Pattern: Integration test against the real server and the real store.

mod common;

use common::{ClientOf, Served};
use fs_wiki_client::{RpcTexts, Timeouts};
use fs_wiki_core::{Error, Texts};

#[tokio::test(flavor = "multi_thread")]
async fn a_move_that_is_done_but_too_large_to_answer_is_still_a_move() {
    let served = Served::in_memory().await;
    // Three texts of about 14 MB as documents: no 32 MiB answer can hold them all.
    let body = format!("{}\n\n", "word ".repeat(200)).repeat(6_000);
    for i in 0..3 {
        served
            .repo
            .create(&format!("big/{i}"), &body)
            .await
            .unwrap();
    }
    let slow = Timeouts {
        call: std::time::Duration::from_secs(600),
        ..Timeouts::default()
    };
    let texts = RpcTexts::connect_with(&served.socket, slow).unwrap();
    let moved = texts.move_folder("big", "moved").await.unwrap();
    let mut got: Vec<_> = moved.iter().map(|t| t.path.clone()).collect();
    got.sort();
    assert_eq!(got, ["moved/0", "moved/1", "moved/2"]);
    assert!(texts.get_by_path("big/0").await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_that_is_not_there_stays_an_error() {
    let served = Served::in_memory().await;
    let e = served
        .client()
        .move_folder("nothing", "x")
        .await
        .unwrap_err();
    assert!(matches!(e, Error::NoSuchFolder(_)), "{e:?}");
}
