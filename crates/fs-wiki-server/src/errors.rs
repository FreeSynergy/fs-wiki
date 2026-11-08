//! Errors of the core as RPC errors.
//!
//! Pattern: Adapter — each error the core can name becomes a typed RFC 9457 problem, so a consumer
//! can tell "not found" from "conflict" from "not possible" without reading a message. A refused
//! `editIf` carries a detail that starts with `conflict:` (`texts.capnp`).

use fs_interfaces::rpc::{failed, problem_keys as keys, unsupported, Problem};
use fs_interfaces::Unsupported;
use fs_wiki_core::Error;

use crate::facts::with_facts;

const NOT_FOUND: &str = "https://freesynergy.net/problems/texts/not-found";
const CONFLICT: &str = "https://freesynergy.net/problems/texts/conflict";
const PATH_TAKEN: &str = "https://freesynergy.net/problems/texts/path-taken";
const MOVE_REFUSED: &str = "https://freesynergy.net/problems/texts/move-refused";
const INVALID: &str = "https://freesynergy.net/problems/texts/invalid";

/// A core error as an RPC error. `Json`, `NotAText` and other store errors mean data already stored
/// no longer reads or the store failed: not the caller's fault. What a client sends is read before
/// the core sees it (`wire`).
#[allow(clippy::needless_pass_by_value)] // used as `map_err(rpc_error)`
pub(crate) fn rpc_error(e: Error) -> capnp::Error {
    use fs_objects::Error as Store;
    let text = e.to_string();
    let problem = |kind: &str, title: &str, status: u16| {
        with_facts(Problem::new(kind, title, text.clone(), status), &e).into_error()
    };
    match &e {
        Error::Unsupported(not_possible) => unsupported(not_possible),
        Error::NotFound(_) | Error::NoSuchFolder(_) | Error::Store(Store::NotFound(_)) => {
            problem(NOT_FOUND, "Text not found", 404)
        }
        Error::Conflict { .. } => crate::errors_v2::conflict(&e, crate::facts::PAGE)
            .unwrap_or_else(|| failed(text.clone())),
        Error::Store(Store::Conflict(_)) => problem(CONFLICT, "Conflict", 409),
        Error::PathTaken(_) => problem(PATH_TAKEN, "Path taken", 409),
        Error::MoveUnfinished { .. } | Error::TargetNotEmpty { .. } => {
            problem(MOVE_REFUSED, "Move refused", 409)
        }
        Error::InvalidPath { .. }
        | Error::InvalidMove { .. }
        | Error::MediaType(_)
        | Error::Store(Store::Invalid(_)) => problem(INVALID, "Invalid text", 422),
        // Only the 0.2 methods raise these; their own mapping (`errors_v2`) types them.
        Error::Invalid { .. }
        | Error::NotInTrash(_)
        | Error::NotEmpty(_)
        | Error::NoSuchRevision { .. }
        | Error::NoSuchNode(_) => crate::errors_v2::rpc_error_v2(e),
        Error::Json(_) | Error::NotAText(_) | Error::Stored(_) | Error::Store(_) => {
            failed(text.clone())
        }
    }
}

/// Like [`rpc_error`], for a method that takes a media type: one the provider does not know is the
/// explicit "not possible" of `capability`, not an invalid request.
pub(crate) fn rpc_error_for_format(capability: &str, e: Error) -> capnp::Error {
    match e {
        Error::MediaType(known) => unsupported(&Unsupported::new(
            capability,
            format!("no format for media type {known:?}; known: text/markdown"),
        )),
        other => rpc_error(other),
    }
}

/// The request itself is not acceptable: `detail` for people, `issue` for a program (the field or
/// the reason), as the extension `issue`.
pub(crate) fn invalid(detail: impl Into<String>, issue: &str) -> capnp::Error {
    Problem::new(INVALID, "Invalid text", detail, 422)
        .with_ext(keys::ISSUE, issue)
        .into_error()
}
