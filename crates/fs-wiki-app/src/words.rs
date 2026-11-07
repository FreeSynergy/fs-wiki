//! The sentences the app says to a person, each one whole, with named places for what changes.
//! **The sentences that have moved here so far**: those of the server connection, the records of
//! imported files and the check of the records against the server. The older ones are still in
//! `source/problems.rs`, `sync.rs`, `import.rs`, `drafts.rs`, `export*.rs` and `wiki.rs` and are to
//! follow (`DESIGN.md` lists them as open).
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

/// `{error}`: what the server said.
pub const CHECK_FAILED: &str =
    "cannot ask the server for the texts of the imported files: {error}; nothing was imported";

/// `{seconds}`: the limit.
pub const CHECK_TIMEOUT: &str = "the server did not answer within {seconds} s when asked for the texts of the \
imported files; nothing was imported (is the server stuck?)";

/// `{count}`: how many files.
pub const MISSING_SUMMARY: &str = "{count} imported files have no text on the server any more; \
they are listed on the problems page and not imported again";

/// Fill the `{name}` places of `sentence` with `values`, in one pass: what a value contains is never
/// looked at again, so a value that itself holds `{name}` stays as it is. A name with no value
/// stays in the sentence.
pub fn say(sentence: &str, values: &[(&str, &dyn Display)]) -> String {
    let mut out = String::with_capacity(sentence.len());
    let mut rest = sentence;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let named = after.find('}').and_then(|close| {
            let name = &after[..close];
            values
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| (close, v.to_string()))
        });
        if let Some((close, value)) = named {
            out.push_str(&value);
            rest = &after[close + 1..];
        } else {
            out.push('{');
            rest = after;
        }
    }
    out.push_str(rest);
    out
}
