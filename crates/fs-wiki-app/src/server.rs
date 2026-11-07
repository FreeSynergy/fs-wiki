//! Reaching the wiki server.
//!
//! Pattern: Factory — the app gets its `Texts` from a running `fs-wiki-server` and from nowhere
//! else. Without a server it does not fall back on a store of its own (ADR 0021): it says how to
//! start one (the sentences are in `words.rs`).

use std::path::{Path, PathBuf};

use fs_wiki_client::RpcTexts;

use crate::words::{self, say};

/// The socket to use: the one given, else the default place of the server.
///
/// # Errors
/// None was given and the environment names no runtime directory.
pub fn socket_path(given: Option<&Path>) -> Result<PathBuf, String> {
    given
        .map(Path::to_path_buf)
        .or_else(RpcTexts::default_socket)
        .ok_or_else(|| words::NO_SOCKET.to_string())
}

/// Connect to the server on `socket`.
///
/// # Errors
/// The message for the person, with the way to start the server.
pub fn connect(socket: &Path) -> Result<RpcTexts, String> {
    RpcTexts::connect(socket).map_err(|e| {
        say(
            words::SERVER_UNREACHABLE,
            &[("path", &socket.display()), ("error", &e)],
        )
    })
}
