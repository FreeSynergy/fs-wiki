//! `subscribe` over a real socket.
//!
//! Pattern: Example-based integration tests of the Observer end to end.

mod common;

use common::seen::{watch, Seen};
use common::{create, set_id, with_server};

#[tokio::test(flavor = "current_thread")]
async fn subscribers_see_changes_until_they_drop_the_capability() {
    with_server(|root| async move {
        let seen = Seen::default();
        let answer = watch(&root, &seen).await;

        let a = create(&root, "a", "# Watched\n").await;
        let mut doc = root.edit_request();
        set_id(doc.get().init_id(), a.id);
        doc.get().set_document(a.document.as_str());
        doc.send().promise.await.unwrap();
        let mut delete = root.delete_request();
        set_id(delete.get().init_id(), a.id);
        delete.send().promise.await.unwrap();
        seen.wait_for(3).await;
        assert_eq!(
            *seen.0.borrow(),
            ["created Watched", "updated Watched", "deleted"]
        );

        drop(answer);
        // A second listener that stays is the clock: once it has heard of "b", the dropped one would have too.
        let witness = Seen::default();
        let _kept = watch(&root, &witness).await;
        create(&root, "b", "x\n").await;
        witness.wait_for(1).await;
        assert_eq!(seen.0.borrow().len(), 3, "still told after the drop");
    })
    .await;
}

#[tokio::test(flavor = "current_thread")]
async fn a_move_tells_the_texts_whose_links_followed() {
    with_server(|root| async move {
        let a = create(&root, "a", "# Moved\n").await;
        create(&root, "b", "# Linker\n\n[[a]]\n").await;
        let seen = Seen::default();
        let _answer = watch(&root, &seen).await;
        let mut req = root.move_request();
        set_id(req.get().init_id(), a.id);
        req.get().set_path("z");
        req.send().promise.await.unwrap();
        seen.wait_for(2).await;
        assert_eq!(*seen.0.borrow(), ["updated Linker", "updated Moved"]);
    })
    .await;
}
