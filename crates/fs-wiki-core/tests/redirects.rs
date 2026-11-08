//! Redirects: set and cleared, never followed by `get`, loops and long chains as results.
//!
//! Pattern: Example-based integration tests over a real in-memory store.

mod common;

use common::wiki;
use fs_wiki_core::{Error, Expect, Outcome, Reason, Texts, MAX_REDIRECT_HOPS};

#[tokio::test]
async fn a_redirect_is_set_cleared_and_never_followed_by_get() {
    let w = wiki().await;
    let a = w.create("old", "moved away\n").await.unwrap();
    let b = w.create("new", "here\n").await.unwrap();
    let r = w.set_redirect(a.id, "new", Expect::Any).await.unwrap();
    assert_eq!(r.redirect_to, "new");
    assert_eq!(
        w.get_by_path("old").await.unwrap().unwrap().id,
        a.id,
        "get does not follow"
    );
    let res = w.resolve("old").await.unwrap();
    assert_eq!(
        (res.outcome, res.hops.as_slice()),
        (Outcome::Resolved, ["old".to_string()].as_slice())
    );
    assert_eq!(res.text.unwrap().id, b.id);
    let plain = w.resolve("new").await.unwrap();
    assert!(
        plain.hops.is_empty() && plain.text.unwrap().id == b.id,
        "no redirect: the start itself"
    );
    let stale = w
        .set_redirect(a.id, "", Expect::Revision(a.revision))
        .await
        .unwrap_err();
    assert!(matches!(stale, Error::Conflict { .. }));
    assert!(w
        .set_redirect(a.id, "", Expect::Any)
        .await
        .unwrap()
        .redirect_to
        .is_empty());
    assert_eq!(w.resolve("old").await.unwrap().text.unwrap().id, a.id);
    let missing = w
        .set_redirect(a.id, "not/there", Expect::Any)
        .await
        .unwrap();
    assert_eq!(
        missing.redirect_to, "not/there",
        "the target need not exist"
    );
    let res = w.resolve("old").await.unwrap();
    assert_eq!(
        (res.outcome, res.hops.as_slice()),
        (
            Outcome::NotFound,
            ["old".to_string(), "not/there".to_string()].as_slice()
        )
    );
    let gone = w.resolve("nowhere").await.unwrap();
    assert_eq!(
        (gone.outcome, gone.hops),
        (Outcome::NotFound, vec!["nowhere".to_string()])
    );
}

#[tokio::test]
async fn bad_targets_are_refused_with_the_reason() {
    let w = wiki().await;
    let a = w.create("a", "x\n").await.unwrap();
    let reason = |e: Error| match e {
        Error::InvalidPath { reason, .. } => reason,
        other => panic!("{other:?}"),
    };
    for (target, expect) in [
        ("https://example.org/x", Reason::ExternalTarget),
        ("mailto:me@example.org", Reason::ExternalTarget),
        ("../x", Reason::DotSegment),
        ("x//y", Reason::EmptySegment),
        ("/x", Reason::EmptySegment),
    ] {
        let e = w.set_redirect(a.id, target, Expect::Any).await.unwrap_err();
        assert_eq!(reason(e), expect, "{target}");
    }
    let e = w.set_redirect(a.id, "a", Expect::Any).await.unwrap_err();
    assert!(
        matches!(
            e,
            Error::Invalid {
                issue: "self-redirect",
                ..
            }
        ),
        "{e:?}"
    );
    assert!(w.get(a.id).await.unwrap().unwrap().redirect_to.is_empty());
}

#[tokio::test]
async fn loops_and_long_chains_are_results_not_errors() {
    let w = wiki().await;
    let (a, b) = (
        w.create("a", "x\n").await.unwrap(),
        w.create("b", "x\n").await.unwrap(),
    );
    w.set_redirect(a.id, "b", Expect::Any).await.unwrap();
    w.set_redirect(b.id, "a", Expect::Any).await.unwrap();
    let r = w.resolve("a").await.unwrap();
    assert_eq!(
        (r.outcome, r.hops, r.text),
        (
            Outcome::Loop,
            ["a", "b", "a"].map(String::from).to_vec(),
            None
        )
    );

    let n = MAX_REDIRECT_HOPS;
    let mut ids = vec![];
    for i in 0..=n + 2 {
        ids.push(w.create(&format!("c{i}"), "x\n").await.unwrap().id);
    }
    for (i, id) in ids.iter().enumerate().take(n) {
        w.set_redirect(*id, &format!("c{}", i + 1), Expect::Any)
            .await
            .unwrap();
    }
    let ok = w.resolve("c0").await.unwrap();
    assert_eq!(
        (ok.outcome, ok.hops.len()),
        (Outcome::Resolved, n),
        "exactly 8 redirects are followed"
    );
    assert_eq!(ok.text.unwrap().path, format!("c{n}"));
    w.set_redirect(ids[n], &format!("c{}", n + 1), Expect::Any)
        .await
        .unwrap();
    let long = w.resolve("c0").await.unwrap();
    assert_eq!(
        (long.outcome, long.hops.len()),
        (Outcome::TooManyHops, n + 1)
    );
    assert_eq!(long.hops.last().unwrap(), &format!("c{n}"));
    // A cycle seen beyond the limit still wins: the 9th redirect leads back into the chain.
    w.set_redirect(ids[n + 1], "c0", Expect::Any).await.unwrap();
    w.set_redirect(ids[n], "c0", Expect::Any).await.unwrap();
    let cyc = w.resolve("c0").await.unwrap();
    assert_eq!(cyc.outcome, Outcome::Loop);
    assert_eq!(cyc.hops.len(), n + 2);
    assert_eq!(
        (cyc.hops.first(), cyc.hops.last()),
        (Some(&"c0".to_string()), Some(&"c0".to_string()))
    );
}
