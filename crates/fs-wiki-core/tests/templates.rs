//! Templates: a flag that is a write, and a copy that is a new text with its own history.
//!
//! Pattern: Example-based integration tests over a real in-memory store.

mod common;

use common::{say, wiki};
use fs_wiki_core::{Error, Expect, Texts};

#[tokio::test]
async fn marking_a_template_is_a_write_with_a_revision() {
    let w = wiki().await;
    let t = w.create("tpl", "# Form\n\nFill in.\n").await.unwrap();
    assert!(!t.template);
    let marked = w
        .set_template(t.id, true, Expect::Revision(t.revision))
        .await
        .unwrap();
    assert!(marked.template);
    assert_ne!(marked.revision, t.revision);
    assert!(w.get(t.id).await.unwrap().unwrap().template);
    let stale = w
        .set_template(t.id, false, Expect::Revision(t.revision))
        .await
        .unwrap_err();
    assert!(matches!(stale, Error::Conflict { found, .. } if found == marked.revision));
    assert!(
        w.get(t.id).await.unwrap().unwrap().template,
        "nothing was written"
    );
    let history = w.history(t.id, None, 10).await.unwrap();
    assert_eq!(history.items.len(), 2);
    let edited = say(&w, &marked, "# Form v2\n").await;
    assert!(edited.template, "an edit keeps the flag");
    assert!(
        !w.set_template(t.id, false, Expect::Any)
            .await
            .unwrap()
            .template
    );
    assert!(matches!(
        w.set_template(fs_objects::ObjectId::new(), true, Expect::Any)
            .await,
        Err(Error::NotFound(_))
    ));
}

#[tokio::test]
async fn a_text_made_from_a_template_is_a_copy_with_a_history_of_its_own() {
    let w = wiki().await;
    let t = w.create("tpl", "# Form\n\nFill in.\n").await.unwrap();
    let err = w.create_from_template(t.id, "new").await.unwrap_err();
    assert!(
        matches!(
            err,
            Error::Invalid {
                issue: "not-a-template",
                ..
            }
        ),
        "{err:?}"
    );
    w.set_template(t.id, true, Expect::Any).await.unwrap();
    let copy = w.create_from_template(t.id, "docs/new").await.unwrap();
    assert_ne!(copy.id, t.id);
    assert!(!copy.template);
    assert_eq!(copy.path, "docs/new");
    assert_eq!(common::md(&w, copy.id).await, common::md(&w, t.id).await);
    assert_eq!(
        copy.document.id,
        copy.id.as_uuid(),
        "the copy has its own document id"
    );
    let history = w.history(copy.id, None, 10).await.unwrap();
    assert_eq!(history.items.len(), 1);
    assert_eq!(history.items[0].summary, "from template tpl");
    assert!(matches!(
        w.create_from_template(t.id, "docs/new").await,
        Err(Error::PathTaken(_))
    ));
    assert!(matches!(
        w.create_from_template(t.id, "a//b").await,
        Err(Error::InvalidPath { .. })
    ));
    assert!(matches!(
        w.create_from_template(fs_objects::ObjectId::new(), "x")
            .await,
        Err(Error::NotFound(_))
    ));
}
