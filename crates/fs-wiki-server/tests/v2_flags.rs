//! texts 0.2 over a real socket: templates, redirects and `resolve`.
//!
//! Pattern: Example-based integration tests against the real server and a real in-memory store.

mod common;

use common::v2::{expect_revision, p, short};
use common::{create, get, read_text, set_id, with_server};
use fs_interfaces::texts_capnp::texts;
use uuid::Uuid;

async fn redirect(
    root: &texts::Client,
    id: Uuid,
    target: &str,
    rev: Option<u64>,
) -> Result<common::Got, capnp::Error> {
    let mut req = root.set_redirect_request();
    set_id(req.get().init_id(), id);
    req.get().set_target(target);
    expect_revision(req.get().init_expect(), rev);
    let answer = req.send().promise.await?;
    Ok(read_text(answer.get()?.get_text()?))
}

async fn template(
    root: &texts::Client,
    id: Uuid,
    on: bool,
    rev: Option<u64>,
) -> Result<common::Got, capnp::Error> {
    let mut req = root.set_template_request();
    set_id(req.get().init_id(), id);
    req.get().set_template(on);
    expect_revision(req.get().init_expect(), rev);
    let answer = req.send().promise.await?;
    Ok(read_text(answer.get()?.get_text()?))
}

#[tokio::test(flavor = "current_thread")]
async fn a_template_is_marked_copied_and_a_stale_mark_is_a_conflict() {
    with_server(|root| async move {
        let t = create(&root, "tpl", "# Form\n").await;
        let marked = template(&root, t.id, true, Some(t.revision)).await.unwrap();
        assert!(marked.template && get(&root, t.id).await.unwrap().template);
        let err = template(&root, t.id, false, Some(t.revision))
            .await
            .err()
            .unwrap();
        assert_eq!(short(&err), "conflict");
        assert_eq!(p(&err).ext_u64("revision"), Some(marked.revision));
        let mut req = root.create_from_template_request();
        set_id(req.get().init_template(), t.id);
        req.get().set_path("docs/new");
        let answer = req.send().promise.await.unwrap();
        let copy = read_text(answer.get().unwrap().get_text().unwrap());
        assert!(!copy.template);
        assert_ne!(copy.id, t.id);
        assert_eq!(copy.path, "docs/new");
        let mut req = root.create_from_template_request();
        set_id(req.get().init_template(), t.id);
        req.get().set_path("docs/new");
        assert_eq!(
            short(&req.send().promise.await.err().unwrap()),
            "path-taken"
        );
        let plain = create(&root, "plain", "x\n").await;
        let mut req = root.create_from_template_request();
        set_id(req.get().init_template(), plain.id);
        req.get().set_path("docs/other");
        let err = req.send().promise.await.err().unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).ext("issue")),
            ("invalid", Some("not-a-template"))
        );
    })
    .await;
}

async fn resolve(root: &texts::Client, path: &str) -> (String, Vec<String>, Option<String>) {
    let mut req = root.resolve_request();
    req.get().set_path(path);
    let answer = req.send().promise.await.unwrap();
    let r = answer.get().unwrap().get_resolution().unwrap();
    let hops = r
        .get_hops()
        .unwrap()
        .iter()
        .map(|h| h.unwrap().to_string().unwrap())
        .collect();
    let text = r.has_text().then(|| read_text(r.get_text().unwrap()).path);
    (
        format!("{:?}", r.get_outcome().unwrap()),
        hops,
        text.filter(|t| !t.is_empty()),
    )
}

#[tokio::test(flavor = "current_thread")]
async fn a_redirect_is_followed_by_resolve_and_loops_are_results() {
    with_server(|root| async move {
        let old = create(&root, "old", "gone\n").await;
        let new = create(&root, "new", "here\n").await;
        let set = redirect(&root, old.id, "new", None).await.unwrap();
        assert_eq!(set.redirect_to, "new");
        assert_eq!(
            get(&root, old.id).await.unwrap().redirect_to,
            "new",
            "get does not follow"
        );
        assert_eq!(
            resolve(&root, "old").await,
            ("Resolved".into(), vec!["old".into()], Some("new".into()))
        );
        assert_eq!(
            resolve(&root, "nope").await,
            ("NotFound".into(), vec!["nope".into()], None)
        );
        redirect(&root, new.id, "old", None).await.unwrap();
        let (outcome, hops, text) = resolve(&root, "old").await;
        assert_eq!(
            (outcome.as_str(), hops, text),
            ("Loop", vec!["old".into(), "new".into(), "old".into()], None)
        );
        let err = redirect(&root, old.id, "old", None).await.err().unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).ext("issue")),
            ("invalid", Some("self-redirect"))
        );
        let err = redirect(&root, old.id, "https://example.org/x", None)
            .await
            .err()
            .unwrap();
        assert_eq!(
            (short(&err).as_str(), p(&err).ext("issue")),
            ("bad-path", Some("external-target"))
        );
        let err = redirect(&root, old.id, "", Some(old.revision))
            .await
            .err()
            .unwrap();
        assert_eq!(short(&err), "conflict");
        assert_eq!(
            redirect(&root, old.id, "", None).await.unwrap().redirect_to,
            ""
        );
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn nine_redirects_are_too_many_hops() {
    with_server(|root| async move {
        let mut ids = vec![];
        for i in 0..11 {
            ids.push(create(&root, &format!("c{i}"), "x\n").await.id);
        }
        for (i, id) in ids.iter().enumerate().take(9) {
            redirect(&root, *id, &format!("c{}", i + 1), None)
                .await
                .unwrap();
        }
        let (outcome, hops, text) = resolve(&root, "c0").await;
        assert_eq!(
            (outcome.as_str(), hops.len(), text),
            ("TooManyHops", 9, None)
        );
        assert_eq!(hops.last().map(String::as_str), Some("c8"));
    })
    .await;
}
