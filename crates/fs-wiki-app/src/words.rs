//! The sentences the app says to a person, each one whole, with named places for what changes.
//!
//! Pattern: none — constants and one small function. A sentence is never built from pieces, and no
//! code decides by comparing these texts: the kinds of error are types (`origins::OriginsError`).
//! `{name}` is filled by [`say`] from the pairs it is given; a name without a value stays visible.

use std::fmt::Display;

/// `{path}`: the socket. `{error}`: what went wrong. The way to start the server follows.
pub const SERVER_UNREACHABLE: &str = "cannot reach the wiki server on {path}: {error}\n\
The wiki lives in fs-wiki-server; start it first, in another terminal:\n\
\n    cargo run --release -p fs-wiki-server -- --socket {path}\n\
\nit keeps its data in its own folder (--data DIR; one server is one wiki). Then start this app \
again with --socket {path} (not needed when the server uses the default socket).";

/// No socket was given and the environment names none.
pub const NO_SOCKET: &str = "no socket: give --socket PATH of the running fs-wiki-server (or set \
XDG_RUNTIME_DIR, where the server puts its socket by default)";

/// `{count}`: how many records.
pub const RECORDS_IGNORED: &str = "the server holds none of the {count} imported files recorded \
here: it is another wiki, so the files are imported again";

/// The records of another program use this folder. `{path}`: the lock file.
pub const RECORDS_BUSY: &str = "the records of imported files are in use by another program \
({path}); run one app per documentation folder";

/// `{path}`: the folder. `{why}`: the system's reason.
pub const RECORDS_FOLDER: &str =
    "cannot create the folder {path} for the records of imported files: {why}";

/// `{path}`: the records file. `{why}`: the system's reason.
pub const RECORDS_READ: &str = "cannot read the records of imported files in {path}: {why}";

/// `{path}`: the records file. `{why}`: the system's reason.
pub const RECORDS_WRITE: &str = "cannot write the records of imported files in {path}: {why}";

/// `{path}`: the lock file. `{why}`: the system's reason.
pub const RECORDS_LOCK: &str = "cannot lock the records of imported files ({path}): {why}";

/// `{path}`: the records file. `{line}`: the line that does not read. `{why}`: the parser's reason.
pub const RECORDS_DAMAGED: &str =
    "the records of imported files in {path} are damaged: line {line} does not read: {why}";

/// A file whose text the server does not have (any more).
pub const TEXT_MISSING: &str = "its text is missing on the server; it is not imported again";

/// Heading of the problems page for such files.
pub const MISSING_TITLE: &str = "Missing on the server";

/// Note under that heading.
pub const MISSING_NOTE: &str = "These files were imported once, but the server does not have \
their texts now (deleted, or a server restored from an older copy). They are not imported again; \
import them by hand if you want them back.";

/// Fill the `{name}` places of `sentence` with `values`.
pub fn say(sentence: &str, values: &[(&str, &dyn Display)]) -> String {
    let mut out = sentence.to_string();
    for (name, value) in values {
        out = out.replace(&format!("{{{name}}}"), &value.to_string());
    }
    out
}
