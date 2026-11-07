//! `subscribe` over a real socket.
//!
//! Pattern: Example-based integration tests of the Observer end to end.

mod common;

use std::cell::RefCell;
use std::rc::Rc;

use capnp_rpc::new_client;
use common::{create, set_id, with_server};
use fs_interfaces::common_capnp::{change, change_listener};
use fs_interfaces::rpc::read_envelope;
use fs_interfaces::texts_capnp::texts;

#[derive(Clone, Default)]
struct Seen(Rc<RefCell<Vec<String>>>);

impl change_listener::Server for Seen {
    async fn change(
        self: Rc<Self>,
        params: change_listener::ChangeParams,
    ) -> Result<(), capnp::Error> {
        let kind = match params.get()?.get_change()?.which()? {
            change::Which::Created(e) => format!("created {}", read_envelope(e?)?.name),
            change::Which::Updated(e) => format!("updated {}", read_envelope(e?)?.name),
            change::Which::Deleted(_) => "deleted".to_string(),
        };
        self.0.borrow_mut().push(kind);
        Ok(())
    }
}

async fn settle() {
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
}

async fn watch(root: &texts::Client, seen: &Seen) -> impl Sized {
    let listener: change_listener::Client = new_client(seen.clone());
    let mut sub = root.subscribe_request();
    sub.get().set_listener(listener);
    sub.send().promise.await.unwrap()
}

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
        settle().await;
        assert_eq!(
            *seen.0.borrow(),
            ["created Watched", "updated Watched", "deleted"]
        );

        drop(answer);
        settle().await;
        create(&root, "b", "x\n").await;
        settle().await;
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
        settle().await;
        assert_eq!(*seen.0.borrow(), ["updated Linker", "updated Moved"]);
    })
    .await;
}
