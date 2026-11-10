//! Helpers of the Admin backend tests: the real server, a client, the runtime, and short forms for values.

#![allow(dead_code)] // each test file uses a different subset

use std::sync::Arc;

use fs_admin_frontend::{Backend, Values};
use fs_wiki_app::admin::{Ctx, backend_for};
use fs_wiki_client::RpcTexts;
use fs_wiki_testserver::Served;
use tokio::runtime::Runtime;

pub struct Wiki {
    pub rt: Runtime,
    pub ctx: Ctx,
    pub texts: Arc<RpcTexts>,
    pub _served: Served,
}

pub fn wiki() -> Wiki {
    let rt = Runtime::new().unwrap();
    let served = rt.block_on(Served::in_memory());
    let texts = Arc::new(RpcTexts::connect(&served.socket).unwrap());
    let ctx = Ctx::new(texts.clone(), rt.handle().clone());
    Wiki {
        rt,
        ctx,
        texts,
        _served: served,
    }
}

pub fn v(pairs: &[(&str, &[&str])]) -> Values {
    pairs
        .iter()
        .map(|(k, vs)| {
            (
                (*k).to_owned(),
                vs.iter().map(|s| (*s).to_owned()).collect(),
            )
        })
        .collect()
}

pub fn backend(w: &Wiki, name: &str) -> Arc<dyn Backend> {
    backend_for(name, &w.ctx).unwrap()
}

pub fn create(w: &Wiki, path: &str, content: &str) -> String {
    backend(w, "Page")
        .run("create", &v(&[("path", &[path]), ("content", &[content])]))
        .unwrap()
        .id
        .unwrap()
}
