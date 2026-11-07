//! A server that is not there, goes away, or says nothing: an error, not a hang.
//!
//! Pattern: Integration tests against the trait. The mute listener is the only thing here that is not
//! the real server, because the real one answers.

mod common;

use std::os::unix::net::UnixListener;
use std::time::Duration;

use common::{make, ClientOf, Served};
use fs_wiki_client::{RpcTexts, Timeouts, CALL_TIMEOUT, CONNECT_TIMEOUT};
use fs_wiki_core::{Error, Paging, Texts};

#[test]
fn nothing_listening_is_an_error_at_connect() {
    let dir = tempfile::tempdir().unwrap();
    assert!(RpcTexts::connect(&dir.path().join("none.sock")).is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_server_that_stopped_answers_with_an_error_every_time() {
    let mut served = Served::in_memory().await;
    let texts = served.client();
    make(&texts, "a", "x\n").await;
    served.stop();
    for _ in 0..2 {
        let got = tokio::time::timeout(Duration::from_secs(10), texts.list("", Paging::default()))
            .await
            .expect("no hang");
        assert!(
            matches!(got, Err(Error::Store(fs_objects::Error::Backend(_)))),
            "{got:?}"
        );
    }
}

#[test]
fn a_listener_that_never_answers_ends_connect_with_the_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mute.sock");
    let listener = UnixListener::bind(&path).unwrap();
    let keep = std::thread::spawn(move || listener.accept().map(|(stream, _)| stream));
    let timeouts = Timeouts {
        connect: Duration::from_millis(300),
        ..Timeouts::default()
    };
    let said = RpcTexts::connect_with(&path, timeouts)
        .err()
        .expect("an error");
    assert!(
        said.to_string().contains("did not answer within 300ms"),
        "{said}"
    );
    drop(keep);
}

#[test]
fn the_defaults_are_five_seconds_to_connect_and_thirty_per_call() {
    assert_eq!(CONNECT_TIMEOUT, Duration::from_secs(5));
    assert_eq!(CALL_TIMEOUT, Duration::from_secs(30));
}
