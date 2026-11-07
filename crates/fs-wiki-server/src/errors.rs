//! Errors of the core as RPC errors.
//!
//! Pattern: Adapter — each error the core can name becomes a typed RFC 9457 problem, so a consumer
//! can tell "not found" from "conflict" from "not possible" without reading a message. A refused
//! `editIf` carries a detail that starts with `conflict:` (`texts.capnp`).

use fs_interfaces::rpc::{failed, unsupported, Problem};
use fs_interfaces::Unsupported;
use fs_wiki_core::Error;

const NOT_FOUND: &str = "https://freesynergy.net/problems/texts/not-found";
const CONFLICT: &str = "https://freesynergy.net/problems/texts/conflict";
const PATH_TAKEN: &str = "https://freesynergy.net/problems/texts/path-taken";
const MOVE_REFUSED: &str = "https://freesynergy.net/problems/texts/move-refused";
const INVALID: &str = "https://freesynergy.net/problems/texts/invalid";

/// A core error as an RPC error. `Json`, `NotAText` and other store errors mean data already stored
/// no longer reads or the store failed: not the caller's fault. What a client sends is read before
/// the core sees it (`wire`).
pub(crate) fn rpc_error(e: Error) -> capnp::Error {
    use fs_store::Error as Store;
    let text = e.to_string();
    match e {
        Error::Unsupported(not_possible) => unsupported(&not_possible),
        Error::NotFound(_) | Error::NoSuchFolder(_) | Error::Store(Store::NotFound(_)) => {
            Problem::new(NOT_FOUND, "Text not found", text, 404).into_error()
        }
        Error::Conflict { .. } | Error::Store(Store::Conflict(_)) => {
            Problem::new(CONFLICT, "Conflict", text, 409).into_error()
        }
        Error::PathTaken(_) => Problem::new(PATH_TAKEN, "Path taken", text, 409).into_error(),
        Error::MoveUnfinished { .. } | Error::TargetNotEmpty { .. } => {
            Problem::new(MOVE_REFUSED, "Move refused", text, 409).into_error()
        }
        Error::InvalidPath { .. }
        | Error::InvalidMove { .. }
        | Error::MediaType(_)
        | Error::Store(Store::Invalid(_)) => invalid(text),
        Error::Json(_) | Error::NotAText(_) | Error::Store(_) => failed(text),
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

/// The request itself is not acceptable.
pub(crate) fn invalid(detail: impl Into<String>) -> capnp::Error {
    Problem::new(INVALID, "Invalid text", detail, 422).into_error()
}
