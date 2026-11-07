//! The problems of the server as the errors of the core.
//!
//! Pattern: Adapter — the inverse of `errors.rs` of `fs-wiki-server`: each RFC 9457 type becomes the
//! variant of `fs_wiki_core::Error` it came from, so the app's own reading of an error needs no change.
//! The wire carries a type, a status and a text, not the variant's fields. What can be read back from
//! the text is (ids, quoted paths, revisions); a path that was refused is parsed again by the core's
//! own `TextPath`, so its reason is never copied here; the rest comes from the call (`Subject`).

use fs_interfaces::rpc::client::problems::{
    local_type, object_id_in, unsupported_of, without_store_prefix, Failure,
};
use fs_interfaces::rpc::Problem;
use fs_interfaces::Unsupported;
use fs_store::{Error as StoreError, ObjectId};
use fs_wiki_core::{Error, TextPath};

use crate::quoted::{quoted, text_after};

/// What a call was about, for the variants that name it.
#[derive(Clone, Debug)]
pub(crate) enum Subject {
    /// Nothing in particular (a list, a search, a create).
    None,
    /// This text.
    Text(ObjectId),
    /// This text, written only if it still has this revision.
    Revision(ObjectId, u64),
    /// This folder (`move_folder`).
    Folder(String),
}

/// The reasons of `Error::InvalidMove`, which the variant holds as `&'static str`. A reason not in
/// this list stays a plain invalid request; the test `moves` fails if the core's text changes.
const MOVE_REASONS: [&str; 2] = [
    "the target lies inside the folder itself",
    "a moved text would land inside the old folder, which an interrupted move could not tell apart",
];

/// The error of the core for what the server (or the wire) said.
pub(crate) fn error_of(e: &capnp::Error, subject: &Subject) -> Error {
    match Failure::of(e) {
        Failure::Problem(problem) => from_problem(&problem, subject),
        Failure::Unimplemented => Unsupported::new(
            "texts",
            "the server does not know this method of the contract",
        )
        .into(),
        Failure::Other(text) => Error::Store(StoreError::Backend(format!("wiki server: {text}"))),
    }
}

fn from_problem(p: &Problem, subject: &Subject) -> Error {
    let detail = p.detail.as_str();
    match local_type(p) {
        "unsupported" => unsupported_of(detail, "texts").into(),
        "texts/not-found" => not_found(detail, subject),
        "texts/conflict" => conflict(detail, subject),
        "texts/path-taken" => path_taken(detail),
        "texts/move-refused" => move_refused(detail),
        "texts/invalid" => invalid(detail),
        "too-large" => Error::Store(StoreError::Invalid(format!("too large: {detail}"))),
        "invalid-request" => Error::Store(StoreError::Invalid(detail.to_string())),
        _ => Error::Store(StoreError::Backend(format!(
            "wiki server ({}): {detail}",
            p.status
        ))),
    }
}

fn not_found(detail: &str, subject: &Subject) -> Error {
    match subject {
        Subject::Text(id) | Subject::Revision(id, _) => Error::NotFound(*id),
        Subject::Folder(prefix) => Error::NoSuchFolder(prefix.clone()),
        Subject::None => object_id_in(detail).map_or_else(
            || Error::Store(StoreError::Backend(format!("wiki server: {detail}"))),
            Error::NotFound,
        ),
    }
}

/// `conflict: text <id> has revision <found>, not <expected>` is the refused `edit_if`; any other
/// conflict is the store's.
fn conflict(detail: &str, subject: &Subject) -> Error {
    let found = text_after(detail, " has revision ").and_then(|n| n.parse().ok());
    match (subject, found) {
        (Subject::Revision(id, expected), Some(found)) => Error::Conflict {
            id: *id,
            expected: *expected,
            found,
        },
        _ => Error::Store(StoreError::Conflict(
            detail
                .strip_prefix("object store: conflict: ")
                .unwrap_or(detail)
                .to_string(),
        )),
    }
}

fn path_taken(detail: &str) -> Error {
    quoted(detail).into_iter().next().map_or_else(
        || Error::Store(StoreError::Conflict(detail.to_string())),
        Error::PathTaken,
    )
}

/// `the move of "a" to "b" is not finished; ...` or `cannot move "a" to "b": there are already ...`.
fn move_refused(detail: &str) -> Error {
    let names = quoted(detail);
    let [from, to] = names.as_slice() else {
        return Error::Store(StoreError::Conflict(detail.to_string()));
    };
    let (from, to) = (from.clone(), to.clone());
    if detail.starts_with("the move of") {
        Error::MoveUnfinished { from, to }
    } else {
        Error::TargetNotEmpty { from, to }
    }
}

/// `invalid path "x": ...`, `cannot move "a" to "b": <reason>`, `unknown media type "x" ...`, or
/// another refusal of the request, which stays a text of the store.
fn invalid(detail: &str) -> Error {
    let names = quoted(detail);
    if detail.starts_with("invalid path ") {
        if let Some(Err(e)) = names.first().map(|p| TextPath::parse(p)) {
            return e;
        }
    }
    if let (Some(rest), [from, to]) = (detail.strip_prefix("cannot move "), names.as_slice()) {
        if let Some(reason) = MOVE_REASONS.iter().find(|r| rest.ends_with(**r)) {
            return Error::InvalidMove {
                from: from.clone(),
                to: to.clone(),
                reason,
            };
        }
    }
    if detail.starts_with("unknown media type ") {
        if let Some(media) = names.into_iter().next() {
            return Error::MediaType(media);
        }
    }
    Error::Store(StoreError::Invalid(
        without_store_prefix(detail).to_string(),
    ))
}
