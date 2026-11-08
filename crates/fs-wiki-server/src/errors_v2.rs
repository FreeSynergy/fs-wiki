//! Errors of the core as the typed problems of texts 0.2.
//!
//! Pattern: Adapter — the same core errors as `errors.rs`, with the types and extensions
//! `texts.capnp` 0.2 names: `not-found` with `kind` `text` / `folder` / `revision`, `bad-path` with the
//! reason as `issue`, the typed `conflict` (`revision` = the current one, `expected` = the one the
//! caller sent), `invalid` with one `issue` per reason, `not-in-trash`, `not-empty`. A client reads
//! `type` and the extensions, never the sentence. The 0.1 methods keep their extensions (`kind` is
//! `page` there) except the conflict, which is typed for every method that can raise it.

use fs_interfaces::rpc::{problem_keys as keys, Problem};
use fs_interfaces::texts_capnp as c;
use fs_wiki_core::Error;

use crate::errors::rpc_error;
use crate::facts::PATH;

/// `kind` of a text in the 0.2 problems.
const TEXT: &str = "text";
const FOLDER: &str = "folder";
const REVISION: &str = "revision";
/// Extension: the number of a revision.
const NUMBER: &str = "number";

fn problem(e: &Error, kind: &str, title: &str, status: u16) -> Problem {
    Problem::new(kind, title, e.to_string(), status)
}

/// The typed conflict of `editIf`, `editWith`, `restoreRevision`, `setTemplate`, `setRedirect`.
/// `kind` is the noun of the text: `text` as the contract says, `page` for `editIf`, whose problems
/// have always said `page` (see DESIGN.md, contract feedback).
pub(crate) fn conflict(e: &Error, kind: &str) -> Option<capnp::Error> {
    let Error::Conflict {
        id,
        expected,
        found,
    } = e
    else {
        return None;
    };
    Some(
        problem(e, c::CONFLICT_PROBLEM_TYPE, "Conflict", 409)
            .with_ext(keys::ID, &id.to_string())
            .with_ext(keys::KIND, kind)
            .with_ext(keys::ISSUE, c::STALE_REVISION_ISSUE)
            .with_ext(c::CONFLICT_REVISION_KEY, &found.to_string())
            .with_ext(c::CONFLICT_EXPECTED_KEY, &expected.to_string())
            .into_error(),
    )
}

/// A core error as the problem of a 0.2 method.
#[allow(clippy::needless_pass_by_value)] // used as `map_err(rpc_error_v2)`
pub(crate) fn rpc_error_v2(e: Error) -> capnp::Error {
    if let Some(typed) = conflict(&e, TEXT) {
        return typed;
    }
    let found = |kind: &str, id: String| {
        problem(&e, c::NOT_FOUND_PROBLEM_TYPE, "Not found", 404)
            .with_ext(keys::ID, &id)
            .with_ext(keys::KIND, kind)
    };
    match &e {
        Error::NotFound(id) => found(TEXT, id.to_string()).into_error(),
        Error::NoSuchNode(id) => found(FOLDER, id.to_string()).into_error(),
        Error::NoSuchRevision { id, number } => found(REVISION, id.to_string())
            .with_ext_u64(NUMBER, *number)
            .into_error(),
        Error::InvalidPath { path, reason } => {
            problem(&e, c::BAD_PATH_PROBLEM_TYPE, "Bad path", 422)
                .with_ext(keys::ISSUE, reason.token())
                .with_ext(PATH, path)
                .into_error()
        }
        Error::Invalid { issue, .. } => {
            problem(&e, c::INVALID_PROBLEM_TYPE, "Invalid request", 422)
                .with_ext(keys::ISSUE, issue)
                .into_error()
        }
        Error::PathTaken(path) => problem(&e, c::PATH_TAKEN_PROBLEM_TYPE, "Path taken", 409)
            .with_ext(keys::KIND, TEXT)
            .with_ext(PATH, path)
            .into_error(),
        Error::NotInTrash(id) => problem(&e, c::NOT_IN_TRASH_PROBLEM_TYPE, "Not in the trash", 409)
            .with_ext(keys::ID, &id.to_string())
            .with_ext(keys::KIND, TEXT)
            .into_error(),
        Error::NotEmpty(id) => problem(&e, c::NOT_EMPTY_PROBLEM_TYPE, "Folder not empty", 409)
            .with_ext(keys::ID, &id.to_string())
            .with_ext(keys::KIND, FOLDER)
            .into_error(),
        _ => rpc_error(e),
    }
}

/// Like [`rpc_error_v2`], for `createFolder`: a taken path is the path of a folder.
pub(crate) fn rpc_error_folder(e: Error) -> capnp::Error {
    match &e {
        Error::PathTaken(path) => problem(&e, c::PATH_TAKEN_PROBLEM_TYPE, "Path taken", 409)
            .with_ext(keys::KIND, FOLDER)
            .with_ext(PATH, path)
            .into_error(),
        _ => rpc_error_v2(e),
    }
}
