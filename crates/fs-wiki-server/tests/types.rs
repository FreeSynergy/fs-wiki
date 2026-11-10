//! `Texts.types` over a real socket: the catalog a generated frontend reads (ADR 0066).
//!
//! Pattern: Example-based integration test against the real server; the expectation is the registry of the core.

mod common;

use common::with_server;
use fs_types::TypeRegistry;

#[tokio::test(flavor = "current_thread")]
async fn the_server_answers_the_catalog_of_the_core() {
    with_server(|root| async move {
        let catalog = root
            .types_request()
            .send()
            .promise
            .await
            .unwrap()
            .get()
            .unwrap()
            .get_catalog()
            .unwrap();
        let answer = catalog.describe_request().send().promise.await.unwrap();
        let read = TypeRegistry::read(answer.get().unwrap().get_types().unwrap()).unwrap();
        let expected = fs_wiki_core::describe::registry().unwrap();
        assert_eq!(read.len(), 5);
        for t in expected.iter() {
            assert_eq!(read.get(&t.name), Some(t), "{}", t.name);
        }
    })
    .await;
}
