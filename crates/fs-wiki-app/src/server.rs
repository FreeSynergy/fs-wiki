//! Reaching the wiki server.
//!
//! Pattern: Factory — the app gets its `Texts` from a running `fs-wiki-server` and from nowhere
//! else. Without a server it does not fall back on a store of its own (ADR 0021): it says how to
//! start one.

use std::path::{Path, PathBuf};

use fs_wiki_client::RpcTexts;

/// The socket to use: the one given, else the default place of the server.
///
/// # Errors
/// None was given and the environment names no runtime directory.
pub fn socket_path(given: Option<&Path>) -> Result<PathBuf, String> {
    given
        .map(Path::to_path_buf)
        .or_else(RpcTexts::default_socket)
        .ok_or_else(|| {
            "no socket: give --socket PATH of the running fs-wiki-server (or set XDG_RUNTIME_DIR, \
             where the server puts its socket by default)"
                .into()
        })
}

/// Connect to the server on `socket`.
///
/// # Errors
/// The message for the person, with the way to start the server.
pub fn connect(socket: &Path) -> Result<RpcTexts, String> {
    RpcTexts::connect(socket).map_err(|e| unreachable_message(socket, &e))
}

/// What to tell the person when the server does not answer.
pub fn unreachable_message(socket: &Path, error: &dyn std::fmt::Display) -> String {
    format!(
        "cannot reach the wiki server on {path}: {error}\n\
         The wiki lives in fs-wiki-server; start it first, in another terminal:\n\
         \n    cargo run --release -p fs-wiki-server -- --socket {path}\n\
         \nit keeps its data in its own folder (--data DIR; one server is one wiki). Then start this \
         app again with --socket {path} (not needed when the server uses the default socket).",
        path = socket.display()
    )
}
