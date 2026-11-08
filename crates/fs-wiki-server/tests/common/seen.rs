//! A listener for `subscribe` tests, and a way to wait for what it is told.

#![allow(dead_code)]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use capnp_rpc::new_client;
use fs_interfaces::common_capnp::{change, change_listener};
use fs_interfaces::rpc::{read_envelope, Problem};
use fs_interfaces::texts_capnp::texts;
use tokio::sync::Notify;

/// What a listener was told, and a way to wait for it: the test waits for the event, not for a time.
#[derive(Clone, Default)]
pub struct Seen(pub Rc<RefCell<Vec<String>>>, Rc<Notify>);

impl Seen {
    /// Wait until `n` lines are there (at most 10 s, so a lost event fails the test, not hangs it).
    pub async fn wait_for(&self, n: usize) {
        let waiting = async {
            while self.0.borrow().len() < n {
                self.1.notified().await;
            }
        };
        tokio::time::timeout(Duration::from_secs(10), waiting)
            .await
            .unwrap_or_else(|_| panic!("expected {n} events, got {:?}", self.0.borrow()));
    }

    fn push(&self, line: String) {
        self.0.borrow_mut().push(line);
        self.1.notify_one();
    }
}

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
        self.push(kind);
        Ok(())
    }

    async fn ended(
        self: Rc<Self>,
        params: change_listener::EndedParams,
        _: change_listener::EndedResults,
    ) -> Result<(), capnp::Error> {
        let problem = Problem::read(params.get()?.get_problem()?)?;
        self.push(format!("ended {}", problem.problem_type));
        Ok(())
    }
}

pub async fn watch(root: &texts::Client, seen: &Seen) -> impl Sized {
    let listener: change_listener::Client = new_client(seen.clone());
    let mut sub = root.subscribe_request();
    sub.get().set_listener(listener);
    sub.send().promise.await.unwrap()
}
