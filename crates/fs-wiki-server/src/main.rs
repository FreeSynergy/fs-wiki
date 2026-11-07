//! `fs-wiki-server`. Usage: `fs-wiki-server [--socket PATH] [--data DIR] [--actor IRI] [--instance DOMAIN]`.
//!
//! Pattern: none (thin binary) — reads its options, opens the store, serves until a signal.

use std::sync::Arc;

use fs_interfaces::rpc::{prepare_data_dir, run_until_signal, ServerOptions};
use fs_store::{Store, TursoStore};
use fs_wiki_core::WikiRepo;
use fs_wiki_server::TextsServer;

fn main() {
    let ServerOptions {
        socket,
        data,
        actor,
        instance,
    } = ServerOptions::from_env_or_exit("wiki");
    let served = run_until_signal(&socket, || async move {
        let data = prepare_data_dir(&data)?;
        let store = TursoStore::open(data.join("objects.db"))
            .await
            .map_err(std::io::Error::other)?;
        let store: Arc<dyn Store> = Arc::new(store);
        let mut repo = WikiRepo::new(store, actor);
        if let Some(instance) = instance {
            repo = repo.with_instance(instance);
        }
        // Texts stored before 0.3.0 get the index fields the pages need; what cannot be repaired is
        // named, and a page says so as well.
        let report = repo.upgrade_index().await.map_err(std::io::Error::other)?;
        if report.rewritten > 0 {
            eprintln!(
                "fs-wiki-server: index completed for {} texts",
                report.rewritten
            );
        }
        for failed in &report.failed {
            eprintln!("fs-wiki-server: text not repaired: {failed}");
        }
        let repo = Arc::new(repo);
        Ok(TextsServer::new(repo.clone(), repo).into_client().client)
    });
    if let Err(e) = served {
        eprintln!("fs-wiki-server: {e}");
        std::process::exit(1);
    }
}
