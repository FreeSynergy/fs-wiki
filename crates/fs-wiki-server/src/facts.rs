//! The facts behind a problem, as typed extensions.
//!
//! Pattern: none (a function that only computes from its argument). What a client used to cut out
//! of the detail text (`coding.md` §2a) is stated here as `id`, `kind`, `issue` and `path`; the detail
//! text stays as it was. The table is in `DESIGN.md`.

use fs_interfaces::rpc::{problem_keys as keys, Problem};
use fs_store::Error as StoreError;
use fs_wiki_core::Error;

/// `kind` of a text.
pub(crate) const PAGE: &str = "page";
/// `kind` of a folder (a path prefix).
pub(crate) const FOLDER: &str = "folder";
/// Extension: the path the problem is about (a taken path, the target in the way). Belongs to the
/// text problems, not to the standard keys.
pub(crate) const PATH: &str = "path";
/// `issue` of a media type no format is known for.
pub(crate) const MEDIA_TYPE: &str = "media-type";

/// `problem` with every fact the core error `e` knows.
pub(crate) fn with_facts(problem: Problem, e: &Error) -> Problem {
    let page = |p: Problem| p.with_ext(keys::KIND, PAGE);
    let folder = |p: Problem, path: &str| p.with_ext(keys::KIND, FOLDER).with_ext(PATH, path);
    match e {
        Error::NotFound(id) | Error::Conflict { id, .. } => {
            page(problem).with_ext(keys::ID, &id.to_string())
        }
        Error::Store(StoreError::NotFound(id)) => problem.with_ext(keys::ID, &id.to_string()),
        Error::NoSuchFolder(path) => folder(problem, path),
        Error::PathTaken(path) => page(problem).with_ext(PATH, path),
        Error::MoveUnfinished { from, .. } => folder(problem, from),
        Error::TargetNotEmpty { to, .. } => folder(problem, to),
        Error::InvalidPath { path, reason } => page(problem)
            .with_ext(keys::ISSUE, reason.token())
            .with_ext(PATH, path),
        Error::InvalidMove { from, reason, .. } => {
            folder(problem, from).with_ext(keys::ISSUE, reason.token())
        }
        Error::MediaType(_) => problem.with_ext(keys::ISSUE, MEDIA_TYPE),
        Error::Store(StoreError::Invalid(reason)) => problem.with_ext(keys::ISSUE, reason),
        _ => problem,
    }
}
