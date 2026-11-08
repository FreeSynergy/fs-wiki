//! Spike: "redact first" against the real hub daemon. Mark an object sensitive locally, sync (the hub
//! gets a stub), purge, lift the mark. Holds for what was never synced in plain; not for what the hub
//! already has (the ignored test names the blocker).
//!
//! Pattern: Example-based spike tests at store level (an `Article` and its revision record, as the wiki
//! writes them) with the real `fs-hub` daemon and the real client session.

use std::sync::Arc;

use fs_hub::{Config, Daemon};
use fs_hub_client::{connect_at, HubSession, HubStatus, Manifest};
use fs_objects::{Envelope, ObjectId, OpenOptions, Store, StoredObject, TursoStore};
use serde_json::json;

const CATEGORY: &str = "dpv:Content";

fn bytes(dir: &std::path::Path) -> String {
    let mut out = String::new();
    for e in std::fs::read_dir(dir).unwrap() {
        if let Ok(b) = std::fs::read(e.unwrap().path()) {
            out.push_str(&String::from_utf8_lossy(&b));
        }
    }
    out
}

struct Rig {
    store: Arc<TursoStore>,
    session: HubSession,
    hub_dir: std::path::PathBuf,
    _hub: Daemon,
    _tmp: tempfile::TempDir,
}

async fn rig(open_sensitive: bool) -> Rig {
    let tmp = tempfile::tempdir().unwrap();
    let hub_dir = tmp.path().join("hub-data");
    let socket = tmp.path().join("run/hub.sock");
    let hub = Daemon::spawn(Config::new(socket.clone(), hub_dir.clone())).unwrap();
    let opts = OpenOptions {
        allow_sensitive_unencrypted: open_sensitive,
        ..OpenOptions::default()
    };
    let store = Arc::new(
        TursoStore::open_with(tmp.path().join("objects.db"), opts)
            .await
            .unwrap(),
    );
    let manifest = Manifest::new("wiki", "0")
        .read(&[])
        .write(&["Article", "WikiRevision"]);
    let HubStatus::Connected(session) = connect_at(&socket, manifest).await.unwrap() else {
        panic!("no hub");
    };
    Rig {
        store,
        session,
        hub_dir,
        _hub: hub,
        _tmp: tmp,
    }
}

fn article(id: ObjectId, text: &str, marked: bool) -> StoredObject {
    let mut env = Envelope::new("Article", "a");
    env.id = id;
    env.context = Some("texts".into());
    if marked {
        env.sensitive = vec![CATEGORY.into()];
    }
    StoredObject::new(env, "application/json", json!({ "document": text }))
}

async fn put(r: &Rig, id: ObjectId, text: &str, marked: bool) {
    r.store
        .put(article(id, text, marked), "wiki")
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn a_disk_store_refuses_the_mark_unless_it_is_opened_for_it() {
    let r = rig(false).await;
    let id = ObjectId::new();
    put(&r, id, "x", false).await;
    let err = r
        .store
        .put(article(id, "x", true), "wiki")
        .await
        .unwrap_err();
    assert!(
        matches!(err, fs_objects::Error::EncryptionRequired(_)),
        "{err:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn never_synced_content_marked_then_purged_leaves_only_a_stub_and_lifts_again() {
    let r = rig(true).await;
    let id = ObjectId::new();
    put(&r, id, "FIRST-SECRET", false).await;
    put(&r, id, "SECOND-SECRET", false).await;
    put(&r, id, "SECOND-SECRET", true).await; // redact first
    r.session.sync_then_purge(&*r.store, id).await.unwrap();
    assert!(!bytes(&r.hub_dir).contains("FIRST-SECRET"));
    assert!(!bytes(&r.hub_dir).contains("SECOND-SECRET"));
    // Lift the mark after the purge: the current copy goes, the purged past does not come back.
    put(&r, id, "CURRENT", false).await;
    r.session.sync_once(&*r.store).await.unwrap();
    let seen = bytes(&r.hub_dir);
    assert!(!seen.contains("FIRST-SECRET") && !seen.contains("SECOND-SECRET"));
    assert!(
        seen.contains("CURRENT"),
        "a lifted mark publishes the current copy"
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "BLOCKER: content the hub already got in plain stays in its log (before images of later edits); a mark \
            covers delivery only and the hub has no purge. Needs a hub-side forget of an object's log."]
async fn content_synced_before_the_mark_stays_at_the_hub() {
    let r = rig(true).await;
    let id = ObjectId::new();
    put(&r, id, "OLD-SECRET", false).await;
    r.session.sync_once(&*r.store).await.unwrap(); // any ordinary sync of a registered writer
    put(&r, id, "NEWER", false).await; // the hub log gets OLD-SECRET as the before image
    r.session.sync_once(&*r.store).await.unwrap();
    put(&r, id, "NEWER", true).await;
    r.session.sync_then_purge(&*r.store, id).await.unwrap();
    assert!(
        !bytes(&r.hub_dir).contains("OLD-SECRET"),
        "the hub still holds the purged text"
    );
}
