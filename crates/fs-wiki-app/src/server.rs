//! Reaching the wiki server.
//!
//! Pattern: Factory — the app gets its `Texts` from a running `fs-wiki-server` of its own version (`gui.rs`, the
//! handshake) and from nowhere else. It does not fall back on a store of its own (ADR 0021).

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

/// Connect to the server that already listens on `socket`, without the version handshake and without starting anything:
/// for tools and examples that talk to a server they started themselves. The window uses `gui::run` (handshake).
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
