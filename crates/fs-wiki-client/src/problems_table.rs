//! The table of problems, read back: every type of the server and the variant it must become.
//!
//! Pattern: none (tests only) — a table of inputs and answers for the types the real server cannot be made to send in a
//! test (an unfinished move, a store conflict, a type of a newer server); the others are covered over
//! the real socket in `tests/errors.rs` and `tests/moves.rs`.

use fs_interfaces::rpc::client::problems::PROBLEM_BASE as BASE;
use fs_interfaces::rpc::Problem;
use fs_objects::{Error as StoreError, ObjectId};
use fs_wiki_core::Error;

use crate::problems::{error_of, Subject};

fn said(kind: &str, status: u16, detail: &str, subject: &Subject) -> Error {
    let problem = Problem::new(format!("{BASE}{kind}"), "t", detail, status);
    error_of(&problem.into_error(), subject)
}

#[test]
fn the_rest_of_the_table_of_the_server_comes_back() {
    let none = Subject::None;
    let e = said(
        "texts/move-refused",
        409,
        "the move of \"a\" to \"b\" is not finished; repeat it before any other move",
        &none,
    );
    assert!(
        matches!(&e, Error::MoveUnfinished { from, to } if from == "a" && to == "b"),
        "{e:?}"
    );
    let e = said(
        "texts/conflict",
        409,
        "object store: conflict: moved on",
        &none,
    );
    assert!(
        matches!(&e, Error::Store(StoreError::Conflict(t)) if t == "moved on"),
        "{e:?}"
    );
    let e = said(
        "texts/invalid",
        422,
        "object store: invalid input: bad name",
        &none,
    );
    assert!(
        matches!(&e, Error::Store(StoreError::Invalid(t)) if t == "bad name"),
        "{e:?}"
    );
    let e = said(
        "texts/invalid",
        422,
        "unknown media type \"x/y\" (known: text/markdown)",
        &none,
    );
    assert!(matches!(&e, Error::MediaType(m) if m == "x/y"), "{e:?}");
    let e = said("too-large", 413, "5 bytes, at most 4 fit", &none);
    assert!(
        matches!(&e, Error::Store(StoreError::Invalid(t)) if t.starts_with("too large")),
        "{e:?}"
    );
    let e = said("invalid-request", 422, "no id", &none);
    assert!(
        matches!(&e, Error::Store(StoreError::Invalid(t)) if t == "no id"),
        "{e:?}"
    );
    let e = said("unsupported", 501, "text.find: not here", &none);
    assert!(
        matches!(&e, Error::Unsupported(u) if u.capability() == "text.find" && u.reason() == "not here"),
        "{e:?}"
    );
    let e = said("something-newer", 500, "who knows", &none);
    assert!(
        matches!(&e, Error::Store(StoreError::Backend(t)) if t.contains("(500)")),
        "{e:?}"
    );
}

#[test]
fn a_not_found_names_what_the_call_was_about() {
    let id = ObjectId::new();
    let e = said(
        "texts/not-found",
        404,
        "text x not found",
        &Subject::Text(id),
    );
    assert!(matches!(e, Error::NotFound(i) if i == id));
    let e = said(
        "texts/not-found",
        404,
        "nothing at \"f\" or below it",
        &Subject::Folder("f".into()),
    );
    assert!(matches!(&e, Error::NoSuchFolder(p) if p == "f"), "{e:?}");
    let text = format!("object store: object {id} not found");
    let e = said("texts/not-found", 404, &text, &Subject::None);
    assert!(matches!(e, Error::NotFound(i) if i == id));
}

#[test]
fn a_conflict_without_the_revision_numbers_stays_a_conflict_of_the_store() {
    let id = ObjectId::new();
    let e = said(
        "texts/conflict",
        409,
        "conflict: odd",
        &Subject::Revision(id, 3),
    );
    assert!(matches!(e, Error::Store(StoreError::Conflict(_))), "{e:?}");
    let e = said(
        "texts/conflict",
        409,
        &format!("conflict: text {id} has revision 9, not 3"),
        &Subject::Revision(id, 3),
    );
    assert!(
        matches!(
            e,
            Error::Conflict {
                expected: 3,
                found: 9,
                ..
            }
        ),
        "{e:?}"
    );
}

#[test]
fn a_method_the_server_does_not_know_is_unsupported() {
    let e = capnp::Error::unimplemented("no such method".into());
    assert!(matches!(
        error_of(&e, &Subject::None),
        Error::Unsupported(_)
    ));
    let e = capnp::Error::failed("disconnected".into());
    assert!(matches!(
        error_of(&e, &Subject::None),
        Error::Store(StoreError::Backend(_))
    ));
}
